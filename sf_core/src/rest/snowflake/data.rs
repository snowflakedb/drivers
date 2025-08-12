pub struct LoginParameters {
    pub account_name: String,
    pub login_name: String,
    pub password: String,
    pub server_url: String,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub warehouse: Option<String>,
    pub role: Option<String>,
}

pub struct ClientInfo {
    pub application: String,
    pub version: String,
    pub os: String,
    pub os_version: String,
    pub ocsp_mode: Option<String>,
}
