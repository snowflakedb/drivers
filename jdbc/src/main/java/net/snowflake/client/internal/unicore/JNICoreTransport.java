package net.snowflake.client.internal.unicore;

import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

public class JNICoreTransport implements CoreTransport {
  private static final SFLogger logger = SFLoggerFactory.getLogger(JNICoreTransport.class);

  public JNICoreTransport() {
    NativeLibraryLoader.init();
  }

  @Override
  public TransportResponse handleMessage(String serviceName, String methodName, byte[] requestBytes)
      throws TransportException {
    logger.debug(
        "JNI transport request: service={}, method={}, requestBytes={}",
        serviceName,
        methodName,
        requestBytes == null ? -1 : requestBytes.length);
    TransportResponse response = nativeHandleMessage(serviceName, methodName, requestBytes);
    if (response == null) {
      logger.warn(
          "JNI transport returned null response: service={}, method={}", serviceName, methodName);
      throw new TransportException("Empty transport response");
    }
    byte[] responseBytes = response.getResponseBytes();
    logger.debug(
        "JNI transport response: service={}, method={}, code={}, responseBytes={}",
        serviceName,
        methodName,
        response.getCode(),
        responseBytes == null ? -1 : responseBytes.length);
    return response;
  }

  @Override
  public long submitMessage(String serviceName, String methodName, byte[] requestBytes)
      throws TransportException {
    logger.debug(
        "JNI async submit: service={}, method={}, requestBytes={}",
        serviceName,
        methodName,
        requestBytes == null ? -1 : requestBytes.length);
    return nativeSubmitMessage(serviceName, methodName, requestBytes);
  }

  @Override
  public TransportResponse awaitMessage(long handle) throws TransportException {
    TransportResponse response = nativeAwaitMessage(handle);
    if (response == null) {
      throw new TransportException("Empty transport response for handle " + handle);
    }
    logger.debug("JNI async await: handle={}, code={}", handle, response.getCode());
    return response;
  }

  @Override
  public void cancel(long handle) {
    nativeCancel(handle);
  }

  private static native TransportResponse nativeHandleMessage(
      String serviceName, String methodName, byte[] requestBytes);

  private static native long nativeSubmitMessage(
      String serviceName, String methodName, byte[] requestBytes);

  private static native TransportResponse nativeAwaitMessage(long handle);

  private static native void nativeCancel(long handle);
}
