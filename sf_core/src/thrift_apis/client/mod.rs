use crate::c_api::SfCoreApi;
use crate::thrift_apis::ThriftApi;

mod handle_transport;

pub fn create_client<T: ThriftApi>() -> T::ClientInterface {
    let span = tracing::info_span!(target: "database_driver", "DatabaseDriverV1Client");
    let _guard = span.enter();
    let api_handle = crate::c_api::sf_core_api_init(SfCoreApi::DatabaseDriverApiV1);
    tracing::debug!(api_handle=?api_handle, "Api handle created");
    let input_handle_transport = handle_transport::HandleTransport::new(api_handle);
    let output_handle_transport = handle_transport::HandleTransport::new(api_handle);
    let input_protocol = thrift::protocol::TCompactInputProtocol::new(input_handle_transport);
    let output_protocol = thrift::protocol::TCompactOutputProtocol::new(output_handle_transport);
    T::client(input_protocol, output_protocol)
}
