pub mod client;
mod database_driver_v1;
pub mod server;
pub use database_driver_v1::DatabaseDriverV1;

use thrift::protocol::TOutputProtocol;
use thrift::{protocol::TInputProtocol, server::TProcessor};

pub trait ThriftApi {
    type ClientInterface;

    fn server() -> Box<dyn TProcessor + Send + Sync>;
    fn client(
        input_protocol: impl TInputProtocol + Send + 'static,
        output_protocol: impl TOutputProtocol + Send + 'static,
    ) -> Self::ClientInterface;
}
