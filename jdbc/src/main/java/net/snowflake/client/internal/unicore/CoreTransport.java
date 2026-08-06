package net.snowflake.client.internal.unicore;

public interface CoreTransport {
  /**
   * Send a message to the server
   *
   * @param serviceName Service name
   * @param methodName Method name
   * @param requestBytes Serialized request
   * @return TransportResponse containing status code and response bytes
   * @throws Exception if transport fails
   */
  public static final int CODE_SUCCESS = 0;

  public static final int CODE_APPLICATION_ERROR = 1;
  public static final int CODE_TRANSPORT_ERROR = 2;

  class TransportResponse {
    private final int code;
    private final byte[] responseBytes;

    public TransportResponse(int code, byte[] responseBytes) {
      this.code = code;
      this.responseBytes = responseBytes;
    }

    public int getCode() {
      return code;
    }

    public byte[] getResponseBytes() {
      return responseBytes;
    }

    public static TransportResponse error(String response) {
      return new TransportResponse(CODE_TRANSPORT_ERROR, response.getBytes());
    }
  }

  TransportResponse handleMessage(String serviceName, String methodName, byte[] requestBytes)
      throws TransportException;

  /**
   * Submit a message asynchronously. Returns immediately with an opaque handle identifying the
   * in-flight operation; the work runs in the background. Pair with {@link #awaitMessage(long)} to
   * obtain the result and {@link #cancel(long)} to abort it.
   *
   * @return a non-zero handle, or 0 if the request could not be submitted
   */
  long submitMessage(String serviceName, String methodName, byte[] requestBytes)
      throws TransportException;

  /**
   * Block until the operation identified by {@code handle} (from {@link #submitMessage}) completes,
   * returning its {@link TransportResponse}.
   */
  TransportResponse awaitMessage(long handle) throws TransportException;

  /**
   * Cancel the in-flight operation identified by {@code handle}. No-op for unknown or already
   * completed handles.
   */
  void cancel(long handle);
}
