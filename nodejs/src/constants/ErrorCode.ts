// Defined here as a plain TypeScript enum instead of exported from the Rust bridge with `#[napi]`:
// - A napi enum does not behave like the 3.x TypeScript enum. `ErrorCode[401001]` returns
//   `undefined` instead of the member name, and `Object.keys(ErrorCode)` returns an empty list.
// - Users who open `ErrorCode` in their editor see the names and values right here, instead of
//   an object built at runtime from the napi enum.
enum ErrorCode {
  ERR_SF_NETWORK_COULD_NOT_CONNECT = 401001,
  ERR_SF_RESPONSE_FAILURE = 401002,
  ERR_LARGE_RESULT_SET_NETWORK_COULD_NOT_CONNECT = 402001,
  ERR_LARGE_RESULT_SET_RESPONSE_FAILURE = 402002,
  ERR_CONN_CONNECT_STATUS_CONNECTING = 405501, // sql state: 08002
  ERR_CONN_CONNECT_STATUS_CONNECTED = 405502, // sql state: 08002
  ERR_CONN_CONNECT_STATUS_DISCONNECTED = 405503, // sql state: 08003
  ERR_CONN_DESTROY_STATUS_PRISTINE = 406501,
  ERR_CONN_DESTROY_STATUS_DISCONNECTED = 406502,
  ERR_CONN_REQUEST_STATUS_PRISTINE = 407001, // sql state: 08003
  ERR_CONN_REQUEST_STATUS_DISCONNECTED = 407002, // sql state: 08003
  ERR_GET_RESULTS_QUERY_ID_NO_DATA = 460002,
  ERR_GET_RESULTS_QUERY_ID_NOT_SUCCESS_STATUS = 460003,
}

export default ErrorCode;
