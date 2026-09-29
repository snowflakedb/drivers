//! Session output formats vs this crate for DATE, TIME, and TIMESTAMP_*.
//! Built only with `--features e2e`. Nightly Rust Core CI runs it with `--ignored`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use sf_core::config::rest_parameters::{
    ClientInfo, DEFAULT_LOG_MAX_QUERY_LENGTH, LoginMethod, LoginParameters, QueryParameters,
};
use sf_core::crl::config::CrlConfig;
use sf_core::crl::{CrlWorker, SharedCrlWorker};
use sf_core::rest::snowflake::{
    QueryInput, QueryOptions, RestError, snowflake_login, snowflake_query,
};
use sf_core::sensitive::SensitiveString;
use sf_core::tls::config::TlsConfig;
use sf_output_format::datetime::{DateTimeFormat, DateTimeValue, Zone};

struct TypedSample {
    kind: &'static str,
    sql: String,
    value: DateTimeValue,
}

struct DateCase {
    format: &'static str,
    year: i32,
    month: u32,
    day: u32,
}

fn sql_string(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn date_sql(year: i32, month: u32, day: u32) -> String {
    format!("DATE '{year:04}-{month:02}-{day:02}'")
}

fn naive(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanos: u32,
) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_nano_opt(hour, minute, second, nanos)
        .unwrap()
}

fn value(
    local: chrono::NaiveDateTime,
    offset_minutes: i32,
    scale: u32,
    zone: Zone,
) -> DateTimeValue {
    DateTimeValue {
        local,
        offset_minutes,
        scale,
        zone,
    }
}

fn render(format: &str, value: &DateTimeValue) -> String {
    DateTimeFormat::compile(format).format(value)
}

fn date_value(year: i32, month: u32, day: u32) -> DateTimeValue {
    value(
        naive(year, month, day, 0, 0, 0, 0),
        0,
        0,
        Zone::Named("GMT"),
    )
}

fn time_value(hour: u32, minute: u32, second: u32, nanos: u32, scale: u32) -> DateTimeValue {
    value(
        naive(1970, 1, 1, hour, minute, second, nanos),
        0,
        scale,
        Zone::Named("GMT"),
    )
}

#[allow(clippy::too_many_arguments)]
fn ntz_value(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanos: u32,
    scale: u32,
) -> DateTimeValue {
    value(
        naive(year, month, day, hour, minute, second, nanos),
        0,
        scale,
        Zone::Named("GMT"),
    )
}

#[allow(clippy::too_many_arguments)]
fn tz_value(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanos: u32,
    scale: u32,
    offset_minutes: i32,
) -> DateTimeValue {
    value(
        naive(year, month, day, hour, minute, second, nanos),
        offset_minutes,
        scale,
        Zone::Offset,
    )
}

#[allow(clippy::too_many_arguments)]
fn ltz_value(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    nanos: u32,
    scale: u32,
) -> DateTimeValue {
    value(
        naive(year, month, day, hour, minute, second, nanos),
        0,
        scale,
        Zone::Named("UTC"),
    )
}

fn time_sql(text: &str, scale: u32) -> String {
    format!("'{text}'::TIME({scale})")
}

fn ntz_sql(text: &str, scale: u32) -> String {
    format!("'{text}'::TIMESTAMP_NTZ({scale})")
}

fn ltz_sql(text: &str, scale: u32) -> String {
    format!("'{text}'::TIMESTAMP_LTZ({scale})")
}

fn tz_sql(stamp: &str, offset: &str) -> String {
    format!("TO_TIMESTAMP_TZ('{stamp} {offset}', 'YYYY-MM-DD HH24:MI:SS.FF TZH:TZM')")
}

fn typed_samples() -> Vec<TypedSample> {
    let date = |kind, year, month, day| TypedSample {
        kind,
        sql: date_sql(year, month, day),
        value: date_value(year, month, day),
    };
    let time = |kind, text, hour, minute, second, nanos, scale| TypedSample {
        kind,
        sql: time_sql(text, scale),
        value: time_value(hour, minute, second, nanos, scale),
    };
    let ntz = |kind, text, year, month, day, hour, minute, second, nanos, scale| TypedSample {
        kind,
        sql: ntz_sql(text, scale),
        value: ntz_value(year, month, day, hour, minute, second, nanos, scale),
    };
    let ltz = |kind, text, year, month, day, hour, minute, second, nanos, scale| TypedSample {
        kind,
        sql: ltz_sql(text, scale),
        value: ltz_value(year, month, day, hour, minute, second, nanos, scale),
    };
    let tz = |kind,
              stamp,
              offset,
              year,
              month,
              day,
              hour,
              minute,
              second,
              nanos,
              scale,
              offset_minutes| {
        TypedSample {
            kind,
            sql: tz_sql(stamp, offset),
            value: tz_value(
                year,
                month,
                day,
                hour,
                minute,
                second,
                nanos,
                scale,
                offset_minutes,
            ),
        }
    };

    vec![
        date("DATE 2005-06-15", 2005, 6, 15),
        date("DATE 2005-06-05", 2005, 6, 5),
        date("DATE 0001-01-01", 1, 1, 1),
        date("DATE 9999-12-31", 9999, 12, 31),
        date("DATE 2024-02-29", 2024, 2, 29),
        date("DATE 2024-01-01", 2024, 1, 1),
        date("DATE 1970-01-01", 1970, 1, 1),
        time(
            "TIME 14:45:30.123456789",
            "14:45:30.123456789",
            14,
            45,
            30,
            123_456_789,
            9,
        ),
        time("TIME 04:05:06", "04:05:06", 4, 5, 6, 0, 0),
        time("TIME 00:00:00", "00:00:00", 0, 0, 0, 0, 0),
        time("TIME 11:00:00", "11:00:00", 11, 0, 0, 0, 0),
        time("TIME 12:00:00", "12:00:00", 12, 0, 0, 0, 0),
        time("TIME 13:00:00", "13:00:00", 13, 0, 0, 0, 0),
        time(
            "TIME 23:59:59.999999999",
            "23:59:59.999999999",
            23,
            59,
            59,
            999_999_999,
            9,
        ),
        time(
            "TIME 12:00:00.5 scale3",
            "12:00:00.5",
            12,
            0,
            0,
            500_000_000,
            3,
        ),
        ntz(
            "TIMESTAMP_NTZ afternoon",
            "2005-06-15 14:45:30.123456789",
            2005,
            6,
            15,
            14,
            45,
            30,
            123_456_789,
            9,
        ),
        ntz(
            "TIMESTAMP_NTZ single-digit",
            "2005-06-05 04:05:06",
            2005,
            6,
            5,
            4,
            5,
            6,
            0,
            0,
        ),
        ntz(
            "TIMESTAMP_NTZ midnight",
            "2005-06-15 00:00:00",
            2005,
            6,
            15,
            0,
            0,
            0,
            0,
            0,
        ),
        ntz(
            "TIMESTAMP_NTZ noon",
            "2005-06-15 12:00:00",
            2005,
            6,
            15,
            12,
            0,
            0,
            0,
            0,
        ),
        ntz(
            "TIMESTAMP_NTZ epoch",
            "1970-01-01 00:00:00.000000001",
            1970,
            1,
            1,
            0,
            0,
            0,
            1,
            9,
        ),
        ntz(
            "TIMESTAMP_NTZ year-1",
            "0001-01-01 00:00:00",
            1,
            1,
            1,
            0,
            0,
            0,
            0,
            0,
        ),
        ntz(
            "TIMESTAMP_NTZ year-9999",
            "9999-12-31 23:59:59.999999999",
            9999,
            12,
            31,
            23,
            59,
            59,
            999_999_999,
            9,
        ),
        ntz(
            "TIMESTAMP_NTZ leap",
            "2024-02-29 23:59:59",
            2024,
            2,
            29,
            23,
            59,
            59,
            0,
            0,
        ),
        tz(
            "TIMESTAMP_TZ -05:30",
            "2005-06-15 14:45:30.123456789",
            "-05:30",
            2005,
            6,
            15,
            14,
            45,
            30,
            123_456_789,
            9,
            -(5 * 60 + 30),
        ),
        tz(
            "TIMESTAMP_TZ +00:00",
            "2005-06-15 14:45:30.123456789",
            "+00:00",
            2005,
            6,
            15,
            14,
            45,
            30,
            123_456_789,
            9,
            0,
        ),
        tz(
            "TIMESTAMP_TZ +05:45",
            "2005-06-15 14:45:30.123456789",
            "+05:45",
            2005,
            6,
            15,
            14,
            45,
            30,
            123_456_789,
            9,
            5 * 60 + 45,
        ),
        tz(
            "TIMESTAMP_TZ -00:30",
            "2005-06-15 14:45:30.123456789",
            "-00:30",
            2005,
            6,
            15,
            14,
            45,
            30,
            123_456_789,
            9,
            -30,
        ),
        tz(
            "TIMESTAMP_TZ +14:00",
            "2005-06-15 14:45:30.123456789",
            "+14:00",
            2005,
            6,
            15,
            14,
            45,
            30,
            123_456_789,
            9,
            14 * 60,
        ),
        tz(
            "TIMESTAMP_TZ -12:00",
            "2005-06-15 00:00:00",
            "-12:00",
            2005,
            6,
            15,
            0,
            0,
            0,
            0,
            9,
            -(12 * 60),
        ),
        ltz(
            "TIMESTAMP_LTZ afternoon UTC",
            "2005-06-15 14:45:30.123456789",
            2005,
            6,
            15,
            14,
            45,
            30,
            123_456_789,
            9,
        ),
        ltz(
            "TIMESTAMP_LTZ midnight UTC",
            "2005-06-15 00:00:00",
            2005,
            6,
            15,
            0,
            0,
            0,
            0,
            0,
        ),
        ltz(
            "TIMESTAMP_LTZ single-digit UTC",
            "2005-06-05 04:05:06",
            2005,
            6,
            5,
            4,
            5,
            6,
            0,
            0,
        ),
        ltz(
            "TIMESTAMP_LTZ leap UTC",
            "2024-02-29 12:00:00",
            2024,
            2,
            29,
            12,
            0,
            0,
            0,
            0,
        ),
        ltz(
            "TIMESTAMP_LTZ year-9999 UTC",
            "9999-12-31 23:59:59.999999999",
            9999,
            12,
            31,
            23,
            59,
            59,
            999_999_999,
            9,
        ),
    ]
}

fn typed_select_sql(samples: &[TypedSample]) -> String {
    let cols: Vec<String> = samples
        .iter()
        .map(|sample| format!("{}::VARCHAR", sample.sql))
        .collect();
    format!("SELECT {}", cols.join(", "))
}

fn formats() -> &'static [&'static str] {
    &[
        "YYYY",
        "YY",
        "Y",
        "UUUU",
        "MM",
        "MO",
        "MON",
        "MMMM",
        "DD",
        "D",
        "DY",
        "HH24",
        "H24",
        "HH12",
        "H12",
        "HH",
        "H",
        "MI",
        "ME",
        "SS",
        "S",
        "AM",
        "PM",
        "P",
        "FF",
        "FF3",
        "TZH:TZM",
        "TZHTZM",
        "TZH",
        "TZD",
        "YYYY-MM-DD",
        "YYYY-MM-DD HH24:MI:SS",
        "YYYY-MM-DD HH24:MI:SS.FF3 TZH:TZM",
        "MMMM DD, YYYY DY",
        "DY, DD MON YYYY HH12:MI:SS AM",
        "YYYYMMDDHH24MISS",
        "MM/DD/YYYY",
        "H12:ME P",
        "yyyy-mm-dd hh24:mi:ss",
        "YyYy-Mm-Dd Hh12:Mi:Ss aM",
        "mon mmmm dy",
        "ff3",
        "tzh:tzm tzhtzm tzh tzd",
        "mo/d/yy h24:me:s p",
        "Uuuu",
        "UuUu",
        "uuuu",
        "uUUU",
        "MMM",
        "MMMMM",
        "TZHTZH",
        "YYY",
        "FF10",
        "FF3X",
        "March",
        "seconds",
        "",
        "[](){}<>/-:.,;|+*=?!'@#$^&~ ",
        "YYYY[QQ]DD",
        "%",
        "DDD",
        "DAY",
        "MONTH",
        "SYYYY",
        "TZM",
        "TZISO",
        r#""Year:" YYYY"#,
        r#"YYYY"-"MM"-"DD"#,
        r#""YYYY MM DD""#,
        r#"YYYY"年"MM"月"DD"日""#,
        r#"YYYY""MM"#,
        "HH12:MI:SS AM",
        "HH24:MI:SS.FF",
        "HH24:MI:SS TZH:TZM",
        "DY MON YY",
        "FF0",
        "FF1",
        "FF9",
        "HH24:MI:SS.FF0",
        "HH24:MI:SS,FF",
    ]
}

fn extra_date_cases() -> Vec<DateCase> {
    let mut out = Vec::new();
    let push = |out: &mut Vec<DateCase>, format: &'static str, year: i32, month: u32, day: u32| {
        out.push(DateCase {
            format,
            year,
            month,
            day,
        });
    };

    for format in [
        "MM", "MO", "DD", "D", "HH24", "H24", "HH12", "H12", "MI", "ME", "SS", "S",
    ] {
        push(&mut out, format, 2005, 6, 5);
    }
    for month in 1..=12 {
        push(&mut out, "MON", 2024, month, 1);
        push(&mut out, "MMMM", 2024, month, 1);
    }
    for day in 1..=7 {
        push(&mut out, "DY", 2024, 1, day);
    }
    for year in [1, 9, 10, 99, 100, 999, 1000, 1970, 2000, 2005, 2015, 9999] {
        push(&mut out, "YYYY", year, 1, 1);
        push(&mut out, "YY", year, 1, 1);
        push(&mut out, "Y", year, 1, 1);
        push(&mut out, "UUUU", year, 1, 1);
    }

    let mut seen = BTreeSet::new();
    out.retain(|case| seen.insert((case.format, case.year, case.month, case.day)));
    out
}

fn set_output_formats_sql(fmt: &str) -> String {
    let quoted = sql_string(fmt);
    format!(
        "ALTER SESSION SET \
         DATE_OUTPUT_FORMAT = {quoted}, \
         TIME_OUTPUT_FORMAT = {quoted}, \
         TIMESTAMP_OUTPUT_FORMAT = {quoted}, \
         TIMESTAMP_NTZ_OUTPUT_FORMAT = {quoted}, \
         TIMESTAMP_LTZ_OUTPUT_FORMAT = {quoted}, \
         TIMESTAMP_TZ_OUTPUT_FORMAT = {quoted}"
    )
}

#[derive(serde::Deserialize)]
struct ParametersFile {
    testconnection: Parameters,
}

#[derive(serde::Deserialize)]
struct Parameters {
    #[serde(rename = "SNOWFLAKE_TEST_ACCOUNT")]
    account: String,
    #[serde(rename = "SNOWFLAKE_TEST_USER")]
    user: String,
    #[serde(rename = "SNOWFLAKE_TEST_SERVER_URL")]
    server_url: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_HOST")]
    host: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_FILE")]
    private_key_file: Option<String>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")]
    private_key_contents: Option<Vec<String>>,
    #[serde(rename = "SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")]
    private_key_password: Option<String>,
}

impl Parameters {
    fn server_url(&self) -> String {
        self.server_url
            .clone()
            .or_else(|| self.host.as_ref().map(|host| format!("https://{host}")))
            .expect("SNOWFLAKE_TEST_SERVER_URL or SNOWFLAKE_TEST_HOST")
    }

    fn private_key(&self) -> String {
        if let Some(ref path) = self.private_key_file {
            return std::fs::read_to_string(path).unwrap();
        }
        self.private_key_contents
            .as_ref()
            .map(|lines| lines.join("\n") + "\n")
            .expect("SNOWFLAKE_TEST_PRIVATE_KEY_FILE or SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS")
    }
}

fn load_parameters(path: &Path) -> Parameters {
    let content = std::fs::read_to_string(path).unwrap();
    let file: ParametersFile = serde_json::from_str(&content).unwrap();
    file.testconnection
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
        release_type: None,
        crl_config: CrlConfig::default(),
        tls_config: TlsConfig::default(),
        proxy_config: sf_core::tls::config::ProxyConfig::default(),
        platforms: Vec::new(),
        os_details: None,
    }
}

fn build_login_params(
    params: &Parameters,
    client_info: ClientInfo,
    server_url: String,
) -> LoginParameters {
    LoginParameters {
        account_name: params.account.clone(),
        login_method: LoginMethod::PrivateKey {
            username: params.user.clone(),
            private_key: SensitiveString::from(params.private_key()),
            passphrase: params
                .private_key_password
                .clone()
                .map(SensitiveString::from),
        },
        server_url,
        database: None,
        schema: None,
        warehouse: None,
        role: None,
        secondary_roles: None,
        client_info,
        session_parameters: None,
        spcs_token: None,
        disable_parallel_user_prompt: false,
        validate_session_token: true,
        browser_opener: None,
    }
}

async fn run_sql(
    query_params: QueryParameters,
    session_token: &str,
    sql: String,
    crl_worker: SharedCrlWorker,
) -> Result<sf_core::rest::snowflake::query_response::Data, String> {
    match snowflake_query(
        query_params,
        session_token,
        QueryInput::new(sql),
        QueryOptions::default(),
        crl_worker,
    )
    .await
    {
        Ok(response) if response.success => Ok(response.data),
        Ok(response) => Err(response
            .message
            .unwrap_or_else(|| "query failed".to_string())),
        Err(RestError::QueryFailed { message, .. }) => Err(message),
        Err(err) => Err(err.to_string()),
    }
}

fn cells(data: &sf_core::rest::snowflake::query_response::Data) -> Vec<String> {
    let (rowset, _) = data.to_json_rowset().expect("JSON rowset");
    rowset[0]
        .iter()
        .map(|cell| cell.clone().expect("non-null VARCHAR"))
        .collect()
}

fn push_diff(diffs: &mut Vec<String>, kind: &str, format: &str, local: &str, backend: &str) {
    if local != backend {
        diffs.push(format!(
            "{kind} {format:?}: local={local:?} backend={backend:?}"
        ));
    }
}

#[tokio::test]
#[ignore = "session datetime output formats; nightly only"]
async fn should_match_session_output_formats_for_date_time_and_timestamps() {
    let parameter_path = std::env::var_os("PARAMETER_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("parameters.json"));
    let params = load_parameters(&parameter_path);
    let crl_worker = CrlWorker::shared_lazy();
    let client_info = default_client_info();
    let server_url = params.server_url();
    let login_params = build_login_params(&params, client_info.clone(), server_url.clone());
    let login_result = snowflake_login(&login_params, None, crl_worker.clone())
        .await
        .unwrap();
    let query_params = QueryParameters {
        server_url,
        client_info,
        log_max_query_length: DEFAULT_LOG_MAX_QUERY_LENGTH,
        log_query_text: false,
        log_query_parameters: false,
        include_retry_reason: false,
    };
    let session_token = login_result.tokens.session_token.reveal().to_string();

    run_sql(
        query_params.clone(),
        &session_token,
        "ALTER SESSION SET PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = 'JSON', TIMEZONE = 'UTC'"
            .to_string(),
        crl_worker.clone(),
    )
    .await
    .unwrap();

    let typed = typed_samples();
    let select_sql = typed_select_sql(&typed);

    let mut diffs = Vec::new();
    for fmt in formats() {
        if let Err(msg) = run_sql(
            query_params.clone(),
            &session_token,
            set_output_formats_sql(fmt),
            crl_worker.clone(),
        )
        .await
        {
            diffs.push(format!("{fmt:?}: SET failed: {msg}"));
            continue;
        }
        let data = match run_sql(
            query_params.clone(),
            &session_token,
            select_sql.clone(),
            crl_worker.clone(),
        )
        .await
        {
            Ok(data) => data,
            Err(msg) => {
                diffs.push(format!("{fmt:?}: SELECT failed: {msg}"));
                continue;
            }
        };
        let backend = cells(&data);
        for (idx, sample) in typed.iter().enumerate() {
            push_diff(
                &mut diffs,
                sample.kind,
                fmt,
                &render(fmt, &sample.value),
                &backend[idx],
            );
        }
    }

    let mut by_format: BTreeMap<&str, Vec<&DateCase>> = BTreeMap::new();
    let extra = extra_date_cases();
    for case in &extra {
        by_format.entry(case.format).or_default().push(case);
    }
    for (fmt, group) in by_format {
        if let Err(msg) = run_sql(
            query_params.clone(),
            &session_token,
            set_output_formats_sql(fmt),
            crl_worker.clone(),
        )
        .await
        {
            diffs.push(format!("{fmt:?}: SET failed: {msg}"));
            continue;
        }
        for case in group {
            let data = run_sql(
                query_params.clone(),
                &session_token,
                format!(
                    "SELECT {}::VARCHAR",
                    date_sql(case.year, case.month, case.day)
                ),
                crl_worker.clone(),
            )
            .await
            .unwrap();
            let backend = cells(&data);
            let local = render(fmt, &date_value(case.year, case.month, case.day));
            push_diff(&mut diffs, "DATE", fmt, &local, &backend[0]);
        }
    }

    assert!(
        diffs.is_empty(),
        "session output format mismatches:\n{}",
        diffs.join("\n")
    );
}
