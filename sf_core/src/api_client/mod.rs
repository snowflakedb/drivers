use crate::c_api::SfCoreApi;
use crate::thrift_gen::database_driver_v1::{DatabaseDriverSyncClient, TDatabaseDriverSyncClient};

mod handle_transport;
pub mod helpers;

pub fn new_database_driver_v1_client() -> Box<dyn TDatabaseDriverSyncClient + Send> {
    let span = tracing::info_span!(target: "database_driver", "DatabaseDriverV1Client");
    let _guard = span.enter();
    let api_handle = crate::c_api::sf_core_api_init(SfCoreApi::DatabaseDriverApiV1);
    tracing::debug!(api_handle=?api_handle, "Api handle created");
    let input_handle_transport = handle_transport::HandleTransport::new(api_handle);
    let output_handle_transport = handle_transport::HandleTransport::new(api_handle);
    let input_protocol = thrift::protocol::TCompactInputProtocol::new(input_handle_transport);
    let output_protocol = thrift::protocol::TCompactOutputProtocol::new(output_handle_transport);
    Box::new(DatabaseDriverSyncClient::new(
        input_protocol,
        output_protocol,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_logging;

    // Database operation tests
    #[test]
    fn test_database_new_and_release() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let db = client.database_new().unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_string() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let db = client.database_new().unwrap();
        client
            .database_set_option_string(
                db.clone(),
                "test_option".to_string(),
                "test_value".to_string(),
            )
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_bytes() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let db = client.database_new().unwrap();
        let test_bytes = vec![1, 2, 3, 4, 5];
        client
            .database_set_option_bytes(db.clone(), "test_option".to_string(), test_bytes)
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_int() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let db = client.database_new().unwrap();
        client
            .database_set_option_int(db.clone(), "test_option".to_string(), 42)
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_set_option_double() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let db = client.database_new().unwrap();
        client
            .database_set_option_double(
                db.clone(),
                "test_option".to_string(),
                std::f64::consts::PI.into(),
            )
            .unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_init() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let db = client.database_new().unwrap();
        client.database_init(db.clone()).unwrap();
        client.database_release(db).unwrap();
    }

    #[test]
    fn test_database_lifecycle() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        // Create database
        let db = client.database_new().unwrap();

        // Set various options
        client
            .database_set_option_string(db.clone(), "driver".to_string(), "test_driver".to_string())
            .unwrap();
        client
            .database_set_option_int(db.clone(), "timeout".to_string(), 30)
            .unwrap();

        // Initialize database
        client.database_init(db.clone()).unwrap();

        // Release database
        client.database_release(db).unwrap();
    }

    // Connection operation tests
    #[test]
    fn test_connection_new_and_release() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let conn = client.connection_new().unwrap();

        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_string() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let conn = client.connection_new().unwrap();
        client
            .connection_set_option_string(
                conn.clone(),
                "username".to_string(),
                "test_user".to_string(),
            )
            .unwrap();
        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_bytes() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let conn = client.connection_new().unwrap();
        let test_bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        client
            .connection_set_option_bytes(conn.clone(), "cert".to_string(), test_bytes)
            .unwrap();
        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_int() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let conn = client.connection_new().unwrap();
        client
            .connection_set_option_int(conn.clone(), "port".to_string(), 5432)
            .unwrap();
        client.connection_release(conn).unwrap();
    }

    #[test]
    fn test_connection_set_option_double() {
        setup_logging();
        let mut client = new_database_driver_v1_client();

        let conn = client.connection_new().unwrap();
        client
            .connection_set_option_double(conn.clone(), "timeout_seconds".to_string(), 30.5.into())
            .unwrap();
        client.connection_release(conn).unwrap();
    }
}
