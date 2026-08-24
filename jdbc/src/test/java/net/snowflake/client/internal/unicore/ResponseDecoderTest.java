package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionInitResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DriverException;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ErrorKind;
import org.junit.jupiter.api.Test;

class ResponseDecoderTest {

  private final ResponseDecoder<ConnectionInitResponse> decoder =
      new ResponseDecoder<>(ConnectionInitResponse.parser(), DriverException.parser());

  @Test
  void shouldReturnParsedResponseOnSuccess() throws Exception {
    ConnectionInitResponse expected = ConnectionInitResponse.getDefaultInstance();
    CoreTransport.TransportResponse response =
        new CoreTransport.TransportResponse(CoreTransport.CODE_SUCCESS, expected.toByteArray());

    assertEquals(expected, decoder.decode(response));
  }

  @Test
  void shouldThrowServiceExceptionOnApplicationError() {
    DriverException error =
        DriverException.newBuilder().setKind(ErrorKind.ERROR_KIND_CANCELLED).build();
    CoreTransport.TransportResponse response =
        new CoreTransport.TransportResponse(
            CoreTransport.CODE_APPLICATION_ERROR, error.toByteArray());

    ServiceException ex = assertThrows(ServiceException.class, () -> decoder.decode(response));
    assertEquals(error, ex.error);
  }

  @Test
  void shouldThrowTransportExceptionOnTransportError() {
    CoreTransport.TransportResponse response =
        new CoreTransport.TransportResponse(CoreTransport.CODE_TRANSPORT_ERROR, "boom".getBytes());

    assertThrows(TransportException.class, () -> decoder.decode(response));
  }

  @Test
  void shouldThrowTransportExceptionOnUnknownCode() {
    CoreTransport.TransportResponse response = new CoreTransport.TransportResponse(99, new byte[0]);

    assertThrows(TransportException.class, () -> decoder.decode(response));
  }
}
