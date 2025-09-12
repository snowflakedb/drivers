use clap::{Arg, Command};
use sf_core::tls::config::TlsConfig;
use sf_core::tls::create_tls_client_with_config;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("tls_client")
        .version("1.0")
        .author("Snowflake Universal Driver Team")
        .about("Minimal TLS client")
        .arg(
            Arg::new("url")
                .help("URL to connect to")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("cert-store")
                .long("cert-store")
                .short('s')
                .help("Path to certificate store/bundle")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("no-verify-hostname")
                .long("no-verify-hostname")
                .help("Disable hostname verification (INSECURE)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("no-verify-certs")
                .long("no-verify-certs")
                .help("Disable certificate verification (INSECURE)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("http-timeout")
                .long("http-timeout")
                .help("HTTP timeout in seconds")
                .value_name("SECONDS")
                .default_value("30")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new("connect-timeout")
                .long("connect-timeout")
                .help("Connection timeout in seconds")
                .value_name("SECONDS")
                .default_value("10")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose logging")
                .action(clap::ArgAction::Count),
        )
        .arg(
            Arg::new("method")
                .long("method")
                .help("HTTP method to use")
                .value_name("METHOD")
                .default_value("GET")
                .value_parser(["GET", "POST", "HEAD", "OPTIONS"]),
        )
        .arg(
            Arg::new("header")
                .long("header")
                .short('H')
                .help("Add HTTP header (format: 'Name: Value')")
                .value_name("HEADER")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("body")
                .long("body")
                .short('d')
                .help("Request body data")
                .value_name("DATA"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output response to file")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("insecure")
                .long("insecure")
                .short('k')
                .help("Allow insecure TLS connections")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let log_level = match matches.get_count("verbose") {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    let url = matches.get_one::<String>("url").unwrap();

    info!("TLS Client starting");
    info!("Target URL: {}", url);

    let mut tls_config = TlsConfig {
        custom_root_store_path: matches.get_one::<String>("cert-store").map(PathBuf::from),
        verify_hostname: !matches.get_flag("no-verify-hostname"),
        verify_certificates: !matches.get_flag("no-verify-certs"),
    };
    if matches.get_flag("insecure") {
        warn!("Insecure mode enabled - disabling all verification");
        tls_config.verify_hostname = false;
        tls_config.verify_certificates = false;
    }

    let client = create_tls_client_with_config(tls_config)
        .map_err(|e| format!("Failed to build TLS client: {:?}", e))?;

    let method = matches.get_one::<String>("method").unwrap();
    let mut request_builder = match method.as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "HEAD" => client.head(url),
        "OPTIONS" => client.request(reqwest::Method::OPTIONS, url),
        _ => unreachable!(),
    };

    if let Some(headers) = matches.get_many::<String>("header") {
        for header in headers {
            if let Some((name, value)) = header.split_once(':') {
                let name = name.trim();
                let value = value.trim();
                request_builder = request_builder.header(name, value);
                debug!("Added header: {}: {}", name, value);
            } else {
                warn!(
                    "Invalid header format: '{}' (expected 'Name: Value')",
                    header
                );
            }
        }
    }

    if let Some(body) = matches.get_one::<String>("body") {
        request_builder = request_builder.body(body.clone());
        debug!("Added body: {} bytes", body.len());
    }

    let total_timeout = Duration::from_secs(
        *matches.get_one::<u64>("http-timeout").unwrap()
            + *matches.get_one::<u64>("connect-timeout").unwrap(),
    );
    request_builder = request_builder.timeout(total_timeout);

    info!("Sending {} request to {}", method, url);
    let start_time = std::time::Instant::now();

    match request_builder.send().await {
        Ok(response) => {
            let elapsed = start_time.elapsed();
            let status = response.status();
            let headers = response.headers().clone();

            info!("Response received in {:?}", elapsed);
            info!("Status: {}", status);
            debug!("Response headers:");
            for (name, value) in headers.iter() {
                debug!("  {}: {:?}", name, value);
            }

            let body = response.text().await?;
            if let Some(output_file) = matches.get_one::<String>("output") {
                std::fs::write(output_file, &body)?;
                info!("Response saved to: {}", output_file);
            } else {
                println!("\nResponse Body ({} bytes):", body.len());
                if body.len() > 1000 {
                    println!("{}...\n[truncated]", &body[..1000]);
                } else {
                    println!("{}", body);
                }
            }

            println!("\nSummary:");
            println!("  Status: {}", status);
            println!("  Size: {} bytes", body.len());
            println!("  Time: {:?}", elapsed);
            if status.is_success() {
                info!("Request completed successfully");
            } else {
                warn!("Request completed with non-success status: {}", status);
            }
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            error!("Request failed after {:?}: {}", elapsed, e);
            if e.is_timeout() {
                error!("Suggestion: Try increasing --http-timeout or --connect-timeout");
            } else if e.is_connect() {
                error!("Suggestion: Check network connectivity and URL");
            } else if e.to_string().contains("certificate") {
                error!(
                    "Suggestion: Try --insecure flag or provide --cert-store for custom certificates"
                );
            }
            return Err(e.into());
        }
    }

    Ok(())
}
