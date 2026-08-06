package net.snowflake.client.internal.unicore;

import com.google.protobuf.InvalidProtocolBufferException;
import com.google.protobuf.Message;
import com.google.protobuf.Parser;
import lombok.RequiredArgsConstructor;

/**
 * Decodes a core {@link CoreTransport.TransportResponse} into a typed protobuf response, applying
 * the status-code contract (success / application error / transport error) in one place.
 *
 * <p>Used by both the blocking generated client methods and {@link CoreFuture} (async), so the
 * decode logic is not duplicated per generated method. Implements {@link CoreFuture.Decoder} so it
 * can be handed straight to a {@link CoreFuture}.
 *
 * @param <T> response message type
 */
@RequiredArgsConstructor
public final class ResponseDecoder<T extends Message> implements CoreFuture.Decoder<T> {
  private final Parser<T> responseParser;
  private final Parser<? extends Message> errorParser;

  @Override
  public T decode(CoreTransport.TransportResponse response) throws TransportException {
    int code = response.getCode();
    byte[] bytes = response.getResponseBytes();
    try {
      if (code == CoreTransport.CODE_SUCCESS) {
        return responseParser.parseFrom(bytes);
      } else if (code == CoreTransport.CODE_APPLICATION_ERROR) {
        throw new ServiceException(errorParser.parseFrom(bytes));
      } else if (code == CoreTransport.CODE_TRANSPORT_ERROR) {
        throw new TransportException(new String(bytes));
      } else {
        throw new TransportException("Unknown error code: " + code);
      }
    } catch (InvalidProtocolBufferException e) {
      throw new TransportException("Invalid protocol buffer exception: " + e.getMessage());
    }
  }
}
