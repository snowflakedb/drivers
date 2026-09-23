use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use sf_core::apis::database_driver_v1::{DatabaseDriverV1, Setting};
use sf_core::config::rest_parameters::BrowserOpenFn;
use url::Url;
use wiremock::MockServer;

pub fn get_loopback(url: &Url) -> std::io::Result<()> {
    let host = url.host_str().expect("loopback host");
    let port = url.port().expect("loopback port");
    let mut stream = std::net::TcpStream::connect((host, port))?;
    let path = match url.query() {
        Some(query) => format!("{}?{query}", url.path()),
        None => url.path().to_string(),
    };
    stream.write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes())?;
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    Ok(())
}

pub fn opener_recording(
    seen: Arc<Mutex<Option<String>>>,
    on_url: impl Fn(&str) + Send + Sync + 'static,
) -> BrowserOpenFn {
    Arc::new(move |url: &str| {
        *seen.lock().expect("seen lock") = Some(url.to_string());
        on_url(url);
        Ok(())
    })
}

pub async fn connect_with_opener(
    server: &MockServer,
    extras: &[(&str, &str)],
    opener: BrowserOpenFn,
) -> Result<(), String> {
    let uri = Url::parse(&server.uri()).expect("mock uri");
    let mut options: HashMap<String, Setting> = [
        ("account", "testaccount"),
        ("user", "alice"),
        ("host", uri.host_str().expect("mock host")),
        ("protocol", "http"),
        ("server_url", server.uri().as_str()),
        ("authentication_timeout", "15"),
        ("client_store_temporary_credential", "false"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), Setting::String(v.to_string())))
    .collect();
    options.insert(
        "port".to_string(),
        Setting::Int(i64::from(uri.port().expect("mock port"))),
    );
    for (k, v) in extras {
        options.insert((*k).to_string(), Setting::String((*v).to_string()));
    }

    let ds = DatabaseDriverV1::new();
    let db = ds.database_new();
    ds.database_init(db).map_err(|e| e.to_string())?;
    let conn = ds.connection_new();
    ds.connection_set_options(conn, options, false, None)
        .await
        .map_err(|e| e.to_string())?;
    ds.connection_set_browser_opener(conn, opener)
        .await
        .map_err(|e| e.to_string())?;
    let result = ds
        .connection_init(None, conn, db)
        .await
        .map_err(|e| e.to_string());
    let _ = ds.connection_release(conn);
    result
}
