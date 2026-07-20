use crate::protobuf::apis::database_driver_v1::{DatabaseDriverImpl, DriverProviders};
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
        Self::new_with(DriverProviders::default())
    }

    pub fn new_with(providers: DriverProviders) -> Self {
        Self {
            driver: DatabaseDriverImpl::new_with(providers),
        }
    }
}

impl Transport for RustTransport {
    async fn handle_message(
        &self,
        service: &str,
        method: &str,
        message: Vec<u8>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        match service {
            "DatabaseDriver" => self.driver.handle_message(method, message, cancel).await,
            _ => Err(ProtoError::Transport(format!("Unknown API: {}", service))),
        }
    }
}
