package net.snowflake.jdbc.e2e.authentication;

import java.io.IOException;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.Locale;
import lombok.SneakyThrows;
import org.apache.hc.client5.http.classic.methods.HttpPost;
import org.apache.hc.client5.http.impl.classic.CloseableHttpClient;
import org.apache.hc.client5.http.impl.classic.HttpClients;
import org.apache.hc.core5.http.ContentType;
import org.apache.hc.core5.http.io.entity.EntityUtils;
import org.apache.hc.core5.http.io.entity.StringEntity;
import org.json.JSONObject;
import org.json.JSONTokener;

interface WithOauthAccessToken {

  default String retrieveOauthAccessToken(
      String tokenUrl,
      String clientId,
      String clientSecret,
      String user,
      String password,
      String role) {
    String form =
        "username="
            + urlEncode(user)
            + "&password="
            + urlEncode(password)
            + "&grant_type=password"
            + "&scope="
            + urlEncode("session:role:" + role.toLowerCase(Locale.ROOT));

    HttpPost post = new HttpPost(tokenUrl);
    post.setHeader("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8");
    String raw = clientId + ":" + clientSecret;
    String basicAuth = Base64.getEncoder().encodeToString(raw.getBytes(StandardCharsets.UTF_8));
    post.setHeader("Authorization", "Basic " + basicAuth);
    post.setEntity(new StringEntity(form, ContentType.create("application/x-www-form-urlencoded")));

    try (CloseableHttpClient client = HttpClients.createDefault()) {
      String body =
          client.execute(
              post,
              response -> {
                String responseBody =
                    response.getEntity() == null
                        ? ""
                        : EntityUtils.toString(response.getEntity(), StandardCharsets.UTF_8);
                if (response.getCode() < 200 || response.getCode() >= 300) {
                  throw new IOException(
                      "OAuth token request failed (status="
                          + response.getCode()
                          + "): "
                          + responseBody);
                }
                return responseBody;
              });
      JSONObject json = new JSONObject(new JSONTokener(body));
      if (!json.has("access_token")) {
        throw new RuntimeException("OAuth token response missing 'access_token': " + body);
      }
      return json.getString("access_token");
    } catch (IOException | RuntimeException e) {
      throw new RuntimeException("Failed to mint OAuth access token", e);
    }
  }

  @SneakyThrows
  static String urlEncode(String value) {
    return URLEncoder.encode(value, StandardCharsets.UTF_8.name());
  }
}
