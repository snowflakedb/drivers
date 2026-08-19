package net.snowflake.jdbc.utils;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import com.sun.net.httpserver.HttpHandler;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.InputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicReference;
import net.snowflake.jdbc.utils.HttpTestClient.Response;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

/** Hermetic self-tests for {@link HttpTestClient} using a JDK-built-in {@link HttpServer}. */
public class HttpTestClientTest {

  private HttpServer server;
  private HttpTestClient client;
  private String baseUrl;

  @BeforeEach
  public void setUp() throws IOException {
    server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
    server.start();
    baseUrl = "http://127.0.0.1:" + server.getAddress().getPort();
    client = new HttpTestClient();
  }

  @AfterEach
  public void tearDown() {
    if (client != null) {
      client.close();
    }
    if (server != null) {
      server.stop(0);
    }
  }

  @Test
  public void getReturns200AndBody() {
    server.createContext("/hello", respondWith(200, "world", "text/plain"));

    Response response = client.get(baseUrl + "/hello");

    assertEquals(200, response.status());
    assertEquals("world", response.body());
    assertTrue(response.ok());
  }

  @Test
  public void postSendsJsonBodyAndContentTypeHeader() {
    AtomicReference<String> capturedBody = new AtomicReference<>();
    AtomicReference<String> capturedContentType = new AtomicReference<>();
    server.createContext(
        "/echo",
        exchange -> {
          capturedContentType.set(exchange.getRequestHeaders().getFirst("Content-Type"));
          try (InputStream in = exchange.getRequestBody()) {
            capturedBody.set(new String(IoTestUtils.readAllBytes(in), StandardCharsets.UTF_8));
          }
          byte[] resp = "{\"ack\":true}".getBytes(StandardCharsets.UTF_8);
          exchange.getResponseHeaders().add("Content-Type", "application/json");
          exchange.sendResponseHeaders(201, resp.length);
          exchange.getResponseBody().write(resp);
          exchange.close();
        });

    Response response = client.post(baseUrl + "/echo", "{\"x\":1}");

    assertEquals(201, response.status());
    assertTrue(response.ok());
    assertEquals("{\"x\":1}", capturedBody.get());
    assertNotNull(capturedContentType.get());
    assertTrue(
        capturedContentType.get().toLowerCase().contains("application/json"),
        "Expected JSON content type, got: " + capturedContentType.get());

    JsonNode parsed = response.json();
    assertEquals(true, parsed.get("ack").asBoolean());
  }

  @Test
  public void postWithNullBodySendsZeroLengthRequest() {
    AtomicReference<Integer> capturedLength = new AtomicReference<>();
    server.createContext(
        "/zero",
        exchange -> {
          try (InputStream in = exchange.getRequestBody()) {
            capturedLength.set(IoTestUtils.readAllBytes(in).length);
          }
          exchange.sendResponseHeaders(200, -1);
          exchange.close();
        });

    Response response = client.post(baseUrl + "/zero", null);

    assertEquals(200, response.status());
    assertEquals(Integer.valueOf(0), capturedLength.get());
  }

  @Test
  public void nonSuccessStatusIsExposedWithoutThrowing() {
    server.createContext("/missing", respondWith(404, "nope", "text/plain"));

    Response response = client.get(baseUrl + "/missing");

    assertEquals(404, response.status());
    assertEquals("nope", response.body());
    assertFalse(response.ok());
  }

  private static HttpHandler respondWith(int status, String body, String contentType) {
    return exchange -> {
      byte[] payload = body.getBytes(StandardCharsets.UTF_8);
      exchange.getResponseHeaders().add("Content-Type", contentType);
      exchange.sendResponseHeaders(status, payload.length);
      exchange.getResponseBody().write(payload);
      exchange.close();
    };
  }
}
