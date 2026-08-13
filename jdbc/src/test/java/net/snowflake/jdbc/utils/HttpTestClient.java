package net.snowflake.jdbc.utils;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import org.apache.hc.client5.http.classic.methods.HttpPost;
import org.apache.hc.client5.http.config.ConnectionConfig;
import org.apache.hc.client5.http.config.RequestConfig;
import org.apache.hc.client5.http.impl.classic.CloseableHttpClient;
import org.apache.hc.client5.http.impl.classic.HttpClients;
import org.apache.hc.client5.http.impl.io.PoolingHttpClientConnectionManager;
import org.apache.hc.client5.http.impl.io.PoolingHttpClientConnectionManagerBuilder;
import org.apache.hc.core5.http.ClassicHttpRequest;
import org.apache.hc.core5.http.ContentType;
import org.apache.hc.core5.http.HttpEntity;
import org.apache.hc.core5.http.io.entity.EntityUtils;
import org.apache.hc.core5.http.io.entity.StringEntity;
import org.apache.hc.core5.http.io.support.ClassicRequestBuilder;
import org.apache.hc.core5.util.Timeout;

/** Minimal Apache HttpClient 5.x wrapper for synchronous test use. Java 8 source compatible. */
public final class HttpTestClient implements AutoCloseable {

  private static final Timeout DEFAULT_TIMEOUT = Timeout.ofSeconds(5);

  private final CloseableHttpClient client;

  public HttpTestClient() {
    ConnectionConfig connectionConfig =
        ConnectionConfig.custom().setConnectTimeout(DEFAULT_TIMEOUT).build();
    PoolingHttpClientConnectionManager connectionManager =
        PoolingHttpClientConnectionManagerBuilder.create()
            .setDefaultConnectionConfig(connectionConfig)
            .build();
    RequestConfig requestConfig =
        RequestConfig.custom()
            .setResponseTimeout(DEFAULT_TIMEOUT)
            .setConnectionRequestTimeout(DEFAULT_TIMEOUT)
            .build();
    this.client =
        HttpClients.custom()
            .setConnectionManager(connectionManager)
            .setDefaultRequestConfig(requestConfig)
            .build();
  }

  public Response get(String url) {
    return execute(ClassicRequestBuilder.get(url).build());
  }

  public Response post(String url, String body) {
    HttpPost post = new HttpPost(url);
    if (body != null) {
      post.setEntity(new StringEntity(body, ContentType.APPLICATION_JSON));
    }
    return execute(post);
  }

  @Override
  public void close() {
    try {
      client.close();
    } catch (IOException e) {
      throw new RuntimeException("Failed to close HttpTestClient", e);
    }
  }

  private Response execute(ClassicHttpRequest request) {
    try {
      return client.execute(
          request,
          response -> {
            HttpEntity entity = response.getEntity();
            String body =
                entity == null ? "" : EntityUtils.toString(entity, StandardCharsets.UTF_8);
            return new Response(response.getCode(), body);
          });
    } catch (IOException e) {
      throw new RuntimeException(
          request.getMethod() + " " + request.getRequestUri() + " failed", e);
    }
  }

  public static final class Response {
    private final int status;
    private final String body;

    Response(int status, String body) {
      this.status = status;
      this.body = body;
    }

    public int status() {
      return status;
    }

    public String body() {
      return body;
    }

    public boolean ok() {
      return status >= 200 && status < 300;
    }

    public JsonNode json() {
      return JsonTestUtils.parseJson(body);
    }

    @Override
    public String toString() {
      return "Response(status=" + status + ", body=" + body + ")";
    }
  }
}
