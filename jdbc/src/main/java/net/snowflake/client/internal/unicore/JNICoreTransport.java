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
    logger.trace(
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
    logger.trace(
        "JNI transport response: service={}, method={}, code={}, responseBytes={}",
        serviceName,
        methodName,
        response.getCode(),
        responseBytes == null ? -1 : responseBytes.length);
    return response;
  }

  private static native TransportResponse nativeHandleMessage(
      String serviceName, String methodName, byte[] requestBytes);
}
