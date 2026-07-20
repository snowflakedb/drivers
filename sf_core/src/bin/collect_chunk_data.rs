use std::path::{Path, PathBuf};

use clap::Parser;
use sf_core::chunks::get_chunk_data;
use sf_core::config::rest_parameters::{
    ClientInfo, DEFAULT_LOG_MAX_QUERY_LENGTH, LoginMethod, LoginParameters, QueryParameters,
};
use sf_core::crl::CrlWorker;
use sf_core::crl::config::CrlConfig;
use sf_core::rest::snowflake::query_response::Data;
use sf_core::rest::snowflake::{QueryExecutionMode, QueryInput, snowflake_login, snowflake_query};
use sf_core::sensitive::SensitiveString;
use sf_core::tls::config::TlsConfig;
use sf_core::tls::create_tls_client_with_config;

#[derive(Parser)]
#[command(name = "collect_chunk_data")]
#[command(about = "Capture raw Snowflake query chunk data to disk for benchmarking")]
struct Cli {
    #[arg(long)]
    sql: String,

    #[arg(long, default_value = "chunk_test_data")]
    output_dir: PathBuf,

    /// Path to the test parameters JSON file (same format as PARAMETER_PATH in tests).
    /// Falls back to the PARAMETER_PATH env var if not provided.
    #[arg(long, env = "PARAMETER_PATH")]
    parameter_path: PathBuf,

    /// Result format: "arrow" or "json". Sets the session format via ALTER SESSION
    /// before executing the query.
    #[arg(long, default_value = "arrow")]
    format: ResultFormat,
}

#[derive(Clone, clap::ValueEnum)]
enum ResultFormat {
    Arrow,
    Json,
}

#[derive(serde::Deserialize)]
struct ParametersFile {
    testconnection: Parameters,
}

#[derive(serde::Deserialize)]
struct Parameters {
    #[serde(rename = "SNOWFLAKE_TEST_ACCOUNT")]
    account: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_USER")]
    user: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_HOST")]
    host: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_DATABASE")]
    database: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SCHEMA")]
    schema: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE")]
    warehouse: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_WAREHOUSE_CORE")]
    warehouse_core: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_ROLE")]
    role: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_SERVER_URL")]
    server_url: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PORT")]
    port: Option<i64>,
    #[serde(rename = "SNOWFLAKE_TEST_PROTOCOL")]
    protocol: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_FILE")]
    private_key_file: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")]
    private_key_contents: Option<Vec<String>>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")]
    private_key_password: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PASSWORD")]
    password: Option<String>,
}

impl Parameters {
    fn warehouse(&self) -> Option<String> {
        self.warehouse_core
            .clone()
            .or_else(|| self.warehouse.clone())
    }

    fn get_server_url(&self) -> Option<String> {
        self.server_url.clone().or_else(|| {
            self.host.as_ref().map(|host| {
                let protocol = self.protocol.as_deref().unwrap_or("https");
                match self.port {
                    Some(port) => format!("{protocol}://{host}:{port}"),
                    None => format!("{protocol}://{host}"),
                }
            })
        })
    }

    fn get_private_key(&self) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(ref path) = self.private_key_file {
            return Ok(std::fs::read_to_string(path)?);
        }
        if let Some(ref lines) = self.private_key_contents {
            return Ok(lines.join("\n") + "\n");
        }
        Err("Neither SNOWFLAKE_TEST_PRIVATE_KEY_FILE nor SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS set in parameters".into())
    }
}

fn load_parameters(path: &PathBuf) -> Result<Parameters, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read parameter file '{}': {e}", path.display()))?;
    let file: ParametersFile = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse parameter file '{}': {e}", path.display()))?;
    Ok(file.testconnection)
}

fn default_client_info() -> ClientInfo {
    ClientInfo {
        client_app_id: env!("CARGO_PKG_NAME").to_string(),
        application: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        os_version: sf_core::telemetry::environment::detect_os_version(),
        ocsp_mode: Some("FAIL_OPEN".to_string()),
        runtime_name: None,
        runtime_version: None,
        compiler: None,
        crl_config: CrlConfig::default(),
        tls_config: TlsConfig::default(),
        proxy_config: sf_core::tls::config::ProxyConfig::default(),
        platforms: Vec::new(),
        os_details: None,
    }
}

#[derive(serde::Serialize)]
struct Metadata {
    format: String,
    chunk_count: usize,
    has_initial: bool,
    total_rows: Option<i64>,
    row_types: Option<Vec<RowTypeMeta>>,
    chunks: Vec<ChunkMeta>,
}

#[derive(serde::Serialize)]
struct ChunkMeta {
    index: usize,
    row_count: i32,
    uncompressed_size: i64,
    compressed_size: i64,
    saved_size: u64,
}

#[derive(serde::Serialize)]
struct RowTypeMeta {
    name: String,
    #[serde(rename = "type")]
    type_: String,
    nullable: bool,
    scale: Option<u64>,
    precision: Option<u64>,
    length: Option<u64>,
    byte_length: Option<u64>,
}

async fn save_arrow_data(
    data: &Data,
    output_dir: &Path,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut has_initial = false;

    if let Some(initial_b64) = data.to_initial_base64_opt() {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let bytes = STANDARD.decode(initial_b64)?;
        tokio::fs::write(output_dir.join("initial.bin"), &bytes).await?;
        has_initial = true;
        println!("  Saved initial chunk ({} bytes)", bytes.len());
    }

    let chunks = data.to_chunk_download_data().unwrap_or_default();
    let chunk_count = chunks.len();
    println!("  Downloading {chunk_count} remote chunks...");

    let mut chunk_metas = Vec::with_capacity(chunk_count);
    for (i, chunk) in chunks.into_iter().enumerate() {
        let row_count = chunk.row_count;
        let uncompressed_size = chunk.uncompressed_size;
        let compressed_size = chunk.compressed_size;
        let bytes = get_chunk_data(
            client.clone(),
            chunk,
            tokio_util::sync::CancellationToken::new(),
        )
        .await?;
        let saved_size = bytes.len() as u64;
        let path = output_dir.join(format!("chunk_{i}.bin"));
        tokio::fs::write(&path, &bytes).await?;
        println!("  chunk_{i}.bin ({saved_size} bytes, {row_count} rows)");
        chunk_metas.push(ChunkMeta {
            index: i,
            row_count,
            uncompressed_size,
            compressed_size,
            saved_size,
        });
    }

    let metadata = Metadata {
        format: "arrow".to_string(),
        chunk_count,
        has_initial,
        total_rows: data.total,
        row_types: None,
        chunks: chunk_metas,
    };
    let json = serde_json::to_string_pretty(&metadata)?;
    tokio::fs::write(output_dir.join("metadata.json"), json).await?;

    Ok(())
}

async fn save_json_data(
    data: &Data,
    output_dir: &Path,
    client: &reqwest::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut has_initial = false;

    let row_type_metas: Option<Vec<RowTypeMeta>> =
        if let Some((rowset, row_types)) = data.to_json_rowset() {
            let json = serde_json::to_vec(rowset)?;
            tokio::fs::write(output_dir.join("initial.bin"), &json).await?;
            has_initial = true;
            println!(
                "  Saved initial rowset ({} rows, {} bytes)",
                rowset.len(),
                json.len()
            );

            Some(
                row_types
                    .iter()
                    .map(|rt| RowTypeMeta {
                        name: rt.name.clone(),
                        type_: rt.type_.clone(),
                        nullable: rt.nullable,
                        scale: rt.scale,
                        precision: rt.precision,
                        length: rt.length,
                        byte_length: rt.byte_length,
                    })
                    .collect(),
            )
        } else {
            None
        };

    let chunks = data.to_chunk_download_data().unwrap_or_default();
    let chunk_count = chunks.len();
    println!("  Downloading {chunk_count} remote chunks...");

    let mut chunk_metas = Vec::with_capacity(chunk_count);
    for (i, chunk) in chunks.into_iter().enumerate() {
        let row_count = chunk.row_count;
        let uncompressed_size = chunk.uncompressed_size;
        let compressed_size = chunk.compressed_size;
        let bytes = get_chunk_data(
            client.clone(),
            chunk,
            tokio_util::sync::CancellationToken::new(),
        )
        .await?;
        let saved_size = bytes.len() as u64;
        let path = output_dir.join(format!("chunk_{i}.bin"));
        tokio::fs::write(&path, &bytes).await?;
        println!("  chunk_{i}.bin ({saved_size} bytes, {row_count} rows)");
        chunk_metas.push(ChunkMeta {
            index: i,
            row_count,
            uncompressed_size,
            compressed_size,
            saved_size,
        });
    }

    let metadata = Metadata {
        format: "json".to_string(),
        chunk_count,
        has_initial,
        total_rows: data.total,
        row_types: row_type_metas,
        chunks: chunk_metas,
    };
    let json = serde_json::to_string_pretty(&metadata)?;
    tokio::fs::write(output_dir.join("metadata.json"), json).await?;

    Ok(())
}

fn build_login_params(
    params: &Parameters,
    client_info: ClientInfo,
    server_url: String,
) -> Result<LoginParameters, Box<dyn std::error::Error>> {
    let account = params
        .account
        .clone()
        .ok_or("SNOWFLAKE_TEST_ACCOUNT not set in parameters")?;
    let user = params
        .user
        .clone()
        .ok_or("SNOWFLAKE_TEST_USER not set in parameters")?;

    let login_method = if params.private_key_file.is_some() || params.private_key_contents.is_some()
    {
        let private_key = params.get_private_key()?;
        LoginMethod::PrivateKey {
            username: user,
            private_key: SensitiveString::from(private_key),
            passphrase: params
                .private_key_password
                .clone()
                .map(SensitiveString::from),
        }
    } else if let Some(ref password) = params.password {
        LoginMethod::Password {
            username: user,
            password: SensitiveString::from(password.clone()),
            passcode_in_password: false,
            passcode: None,
        }
    } else {
        return Err("No authentication method available: set private_key_file, private_key_contents, or password in parameters".into());
    };

    Ok(LoginParameters {
        account_name: account,
        login_method,
        server_url,
        database: params.database.clone(),
        schema: params.schema.clone(),
        warehouse: params.warehouse(),
        role: params.role.clone(),
        client_info,
        session_parameters: None,
        spcs_token: None,
        disable_parallel_user_prompt: false,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let params = load_parameters(&cli.parameter_path)?;

    let crl_worker = CrlWorker::shared_lazy();

    let client_info = default_client_info();
    let server_url = params
        .get_server_url()
        .ok_or("Cannot determine server URL: set SNOWFLAKE_TEST_HOST or SNOWFLAKE_TEST_SERVER_URL in parameters")?;

    let login_params = build_login_params(&params, client_info.clone(), server_url.clone())?;

    println!("Logging in to Snowflake...");
    let login_result = snowflake_login(
        &login_params,
        None,
        crl_worker.clone(),
        tokio_util::sync::CancellationToken::new(),
    )
    .await?;
    println!(
        "Login successful (session_id={})",
        login_result.tokens.session_id
    );

    let query_params = QueryParameters {
        server_url,
        client_info,
        log_max_query_length: DEFAULT_LOG_MAX_QUERY_LENGTH,
        log_query_text: false,
        log_query_parameters: false,
    };
    let session_token = login_result.tokens.session_token.reveal().to_string();

    let alter_sql = match cli.format {
        ResultFormat::Arrow => "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = 'ARROW'",
        ResultFormat::Json => "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = 'JSON'",
    };
    println!("Setting format: {alter_sql}");
    let alter_response = snowflake_query(
        query_params.clone(),
        &session_token,
        QueryInput::new(alter_sql),
        QueryExecutionMode::Blocking,
        crl_worker.clone(),
        tokio_util::sync::CancellationToken::new(),
    )
    .await?;

    if !alter_response.success {
        let msg = alter_response.message.as_deref().unwrap_or("unknown error");
        return Err(format!("ALTER SESSION failed: {msg}").into());
    }

    println!("Executing query: {}", cli.sql);
    let response = snowflake_query(
        query_params,
        &session_token,
        QueryInput::new(cli.sql.clone()),
        QueryExecutionMode::Blocking,
        crl_worker.clone(),
        tokio_util::sync::CancellationToken::new(),
    )
    .await?;

    if !response.success {
        let msg = response.message.as_deref().unwrap_or("unknown error");
        return Err(format!("Query failed: {msg}").into());
    }

    let total_rows = response.data.total.unwrap_or(0);
    println!("Query returned {total_rows} total rows");

    tokio::fs::create_dir_all(&cli.output_dir).await?;

    let tls_client = create_tls_client_with_config(TlsConfig::default(), crl_worker)?;

    let format_label = match cli.format {
        ResultFormat::Arrow => "arrow",
        ResultFormat::Json => "json",
    };
    assert_eq!(
        response.data.query_result_format.as_deref().unwrap(),
        format_label
    );

    println!("Format: {format_label}");

    match format_label {
        "arrow" => save_arrow_data(&response.data, &cli.output_dir, &tls_client).await?,
        "json" => save_json_data(&response.data, &cli.output_dir, &tls_client).await?,
        other => return Err(format!("Unsupported query result format: {other}").into()),
    }

    println!("Done! Chunk data saved to {:?}", cli.output_dir);
    Ok(())
}
