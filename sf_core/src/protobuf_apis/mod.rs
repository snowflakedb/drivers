use crate::protobuf_apis::database_driver_v1::DatabaseDriverImpl;
use crate::protobuf_gen::database_driver_v1::DatabaseDriverServer;
use proto_utils::*;

pub mod database_driver_v1;

pub fn call_proto(api: &str, method: &str, message: &[u8]) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
    match api {
        "DatabaseDriver" => DatabaseDriverImpl::handle_message(method, message.to_vec()),
        _ => Err(ProtoError::Transport(format!("Unknown API: {}", api))),
    }
}

pub struct RustTransport {}

impl Transport for RustTransport {
    fn handle_message(
        service: &str,
        method: &str,
        message: Vec<u8>,
    ) -> Result<Vec<u8>, ProtoError<Vec<u8>>> {
        call_proto(service, method, &message)
    }
}
