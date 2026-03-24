use crate::protobuf::apis::database_driver_v1::DatabaseDriverImpl;
use crate::protobuf::generated::database_driver_v1::DatabaseDriverServer;
use proto_utils::*;

pub mod database_driver_v1;

pub struct RustTransport {
    driver: DatabaseDriverImpl,
}

impl Default for RustTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl RustTransport {
    pub fn new() -> Self {
        Self {
            driver: DatabaseDriverImpl::new(),
        }
    }
}

impl Transport for RustTransport {
    async fn handle_message(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        match service {
            "DatabaseDriver" => self.driver.handle_message(method, message).await,
            _ => Err(ProtoError::Transport(format!("Unknown API: {}", service))),
        }
    }
}
