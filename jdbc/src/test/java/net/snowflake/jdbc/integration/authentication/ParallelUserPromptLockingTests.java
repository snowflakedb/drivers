package net.snowflake.jdbc.integration.authentication;

import static net.snowflake.jdbc.utils.JsonTestUtils.parseJson;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Properties;
import java.util.UUID;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.wiremock.BaseWiremockTest;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/** WireMock integration tests for process-global serialization of interactive auth prompts. */
@SkipOldDriver("BD#55")
class ParallelUserPromptLockingTests extends BaseWiremockTest {

  private static final String AUTHN_REQUEST_PATTERN = "/session/authenticator-request.*";
  private static final String LOGIN_REQUEST_PATTERN = "/session/v1/login-request.*";
  private static final long WATCHER_TIMEOUT_NANOS = TimeUnit.SECONDS.toNanos(15);
  private static final long CONNECT_TIMEOUT_SECONDS = 60;
  private static final long POLL_INTERVAL_MS = 200;

  /** Serializes WireMock admin API calls from concurrent callback watcher threads. */
  private final Object wiremockLock = new Object();

  @BeforeAll
  void loadDriver() throws Exception {
    Class.forName(SnowflakeDriver.class.getName());
  }

  @Test
  void shouldShowOnlyOneExternalBrowserPromptWhenMultipleConnectionsAuthenticateConcurrently()
      throws Exception {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    Properties props = externalBrowserProps(uniqueUser("eb_lock_"), true, true);

    // And Wiremock returns valid ssoUrl and proofKey for authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request.json");

    // And Login endpoint returns success
    wiremock.addMapping("auth/login_success_external_browser_with_id_token.json");
    wiremock.addMapping("auth/login_success_cached_id_token.json");

    // When Multiple connections attempt external browser login concurrently
    List<Throwable> watcherErrors = synchronizedErrors();
    Thread watcher = startCallbackWatcher("browser_sso_token_locked", 0, 1, watcherErrors);
    try {
      boolean[] opened = connectConcurrently(props);
      joinWatchers(watcherErrors, watcher);

      // Then Only one authenticator-request is sent to the server
      assertEquals(1, countAuthenticatorRequests());

      // And All connections succeed
      assertBothSucceeded(opened);
    } finally {
      joinWatchersQuietly(watcher);
    }
  }

  @Test
  void shouldShowOnlyOneMfaPromptWhenMultipleConnectionsAuthenticateConcurrently()
      throws Exception {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is true
    Properties props = mfaProps(uniqueUser("mfa_lock_"));

    // And Wiremock returns successful login with MFA token for the first connection
    wiremock.addMapping("auth/mfa_login_success_with_mfa_token.json");
    wiremock.addMapping("auth/mfa_login_success_with_cached_token.json");

    // When Multiple connections attempt username_password_mfa login concurrently
    boolean[] opened = connectConcurrently(props);

    // Then Only one interactive MFA login-request is sent to the server
    assertEquals(1, countInteractiveMfaLogins());
    // 1 interactive + 1 cached-token login == 2 total login-requests
    assertEquals(2, countLoginRequests());

    // And All connections succeed using the cached MFA token
    assertBothSucceeded(opened);
  }

  @Test
  void shouldShowIndependentPromptsWhenDisableParallelUserPromptIsFalse() throws Exception {
    // Given clientStoreTemporaryCredential is enabled and DISABLE_PARALLEL_USER_PROMPT is false
    Properties props = externalBrowserProps(uniqueUser("eb_nolock_"), true, false);

    // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request.json");

    // And Login endpoint returns success
    wiremock.addMapping("auth/login_success_external_browser.json");

    // When Multiple connections attempt external browser login concurrently
    List<Throwable> watcherErrors = synchronizedErrors();
    Thread watcherOne = startCallbackWatcher("nlock_token_1", 0, 1, watcherErrors);
    Thread watcherTwo = startCallbackWatcher("nlock_token_2", 1, 2, watcherErrors);
    try {
      boolean[] opened = connectConcurrently(props);
      joinWatchers(watcherErrors, watcherOne, watcherTwo);

      // Then Each connection sends its own authenticator-request to the server
      assertTrue(countAuthenticatorRequests() >= 2);

      // And All connections succeed independently
      assertBothSucceeded(opened);
    } finally {
      joinWatchersQuietly(watcherOne, watcherTwo);
    }
  }

  @Test
  void shouldShowIndependentPromptsWhenClientStoreTemporaryCredentialIsFalse() throws Exception {
    // Given clientStoreTemporaryCredential is disabled and DISABLE_PARALLEL_USER_PROMPT is true
    Properties props = externalBrowserProps(uniqueUser("eb_nocache_"), false, true);

    // And Wiremock returns valid ssoUrl and proofKey for each authenticator-request
    wiremock.addMapping("auth/external_browser_authenticator_request.json");

    // And Login endpoint returns success
    wiremock.addMapping("auth/login_success_external_browser.json");

    // When Multiple connections attempt external browser login concurrently
    List<Throwable> watcherErrors = synchronizedErrors();
    Thread watcherOne = startCallbackWatcher("nocache_token_1", 0, 1, watcherErrors);
    Thread watcherTwo = startCallbackWatcher("nocache_token_2", 1, 2, watcherErrors);
    try {
      boolean[] opened = connectConcurrently(props);
      joinWatchers(watcherErrors, watcherOne, watcherTwo);

      // Then Each connection sends its own authenticator-request to the server
      assertTrue(countAuthenticatorRequests() >= 2);

      // And All connections succeed independently
      assertBothSucceeded(opened);
    } finally {
      joinWatchersQuietly(watcherOne, watcherTwo);
    }
  }

  private Properties externalBrowserProps(
      String user, boolean storeTemporaryCredential, boolean disableParallelUserPrompt) {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", user);
    props.setProperty("protocol", "http");
    props.setProperty("authenticator", "EXTERNALBROWSER");
    props.setProperty("authentication_timeout", "30");
    if (storeTemporaryCredential) {
      props.setProperty("clientStoreTemporaryCredential", "true");
    }
    props.setProperty("DISABLE_PARALLEL_USER_PROMPT", Boolean.toString(disableParallelUserPrompt));
    return props;
  }

  private Properties mfaProps(String user) {
    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", user);
    props.setProperty("password", "test_password");
    props.setProperty("protocol", "http");
    props.setProperty("authenticator", "USERNAME_PASSWORD_MFA");
    props.setProperty("clientStoreTemporaryCredential", "true");
    props.setProperty("DISABLE_PARALLEL_USER_PROMPT", "true");
    return props;
  }

  /**
   * Opens two connections in parallel, then closes each inside its worker. Awaits both futures even
   * if the first fails so a late-finishing JDBC connect cannot leak into later tests.
   */
  private boolean[] connectConcurrently(Properties props) throws Exception {
    ExecutorService executor = Executors.newFixedThreadPool(2);
    try {
      String url = wiremockJdbcUrl();
      Future<Boolean> futureOne = executor.submit(() -> openThenClose(url, copy(props)));
      Future<Boolean> futureTwo = executor.submit(() -> openThenClose(url, copy(props)));
      Exception firstError = null;
      boolean firstOpen = false;
      boolean secondOpen = false;
      try {
        firstOpen = futureOne.get(CONNECT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
      } catch (Exception e) {
        firstError = unwrapExecutionException(e);
      }
      try {
        secondOpen = futureTwo.get(CONNECT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
      } catch (Exception e) {
        if (firstError == null) {
          firstError = unwrapExecutionException(e);
        }
      }
      if (firstError != null) {
        throw firstError;
      }
      return new boolean[] {firstOpen, secondOpen};
    } finally {
      executor.shutdown();
      try {
        if (!executor.awaitTermination(CONNECT_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
          executor.shutdownNow();
          executor.awaitTermination(CONNECT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
        }
      } catch (InterruptedException e) {
        executor.shutdownNow();
        Thread.currentThread().interrupt();
      }
    }
  }

  private static boolean openThenClose(String url, Properties props) throws SQLException {
    try (Connection connection = DriverManager.getConnection(url, props)) {
      return !connection.isClosed();
    }
  }

  private static Exception unwrapExecutionException(Exception e) {
    if (e instanceof ExecutionException && e.getCause() instanceof Exception) {
      return (Exception) e.getCause();
    }
    return e;
  }

  private Thread startCallbackWatcher(
      final String token, final int n, final int minRequests, final List<Throwable> errors) {
    Thread thread =
        new Thread(
            () -> {
              try {
                long deadline = System.nanoTime() + WATCHER_TIMEOUT_NANOS;
                while (System.nanoTime() < deadline) {
                  if (countAuthenticatorRequests() >= minRequests) {
                    simulateBrowserCallbackNth(token, n);
                    return;
                  }
                  // Poll interval, not a wait-for-completion sleep.
                  Thread.sleep(POLL_INTERVAL_MS);
                }
                throw new RuntimeException(
                    "timed out waiting for " + minRequests + " authenticator-request(s)");
              } catch (Throwable t) {
                errors.add(t);
              }
            },
            "browser-callback-" + n);
    thread.setDaemon(true);
    thread.start();
    return thread;
  }

  /**
   * Poll WireMock for the n-th authenticator-request (0-indexed), extract sf_core's redirect port,
   * then deliver {@code token} to its localhost callback listener.
   */
  private void simulateBrowserCallbackNth(String token, int n) {
    long deadline = System.nanoTime() + WATCHER_TIMEOUT_NANOS;
    while (System.nanoTime() < deadline) {
      List<JsonNode> requests;
      synchronized (wiremockLock) {
        requests = wiremock.getRequests(AUTHN_REQUEST_PATTERN);
      }
      if (requests.size() > n) {
        JsonNode body = parseJson(requests.get(n).get("body").asText());
        int port = Integer.parseInt(body.get("data").get("BROWSER_MODE_REDIRECT_PORT").asText());
        try (Socket sock = new Socket()) {
          sock.connect(new InetSocketAddress("127.0.0.1", port), 5_000);
          OutputStream out = sock.getOutputStream();
          String httpRequest = "GET /?token=" + token + " HTTP/1.1\r\nHost: localhost\r\n\r\n";
          out.write(httpRequest.getBytes(StandardCharsets.UTF_8));
          out.flush();
          sock.getInputStream().read(new byte[4096]);
        } catch (Exception e) {
          throw new RuntimeException("Failed to deliver browser callback to port " + port, e);
        }
        return;
      }
      try {
        Thread.sleep(POLL_INTERVAL_MS);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new RuntimeException("Interrupted while waiting for authenticator-request", e);
      }
    }
    throw new RuntimeException("authenticator-request #" + n + " never arrived at WireMock");
  }

  private int countAuthenticatorRequests() {
    synchronized (wiremockLock) {
      return wiremock.getRequests(AUTHN_REQUEST_PATTERN).size();
    }
  }

  private int countLoginRequests() {
    synchronized (wiremockLock) {
      return wiremock.getRequests(LOGIN_REQUEST_PATTERN).size();
    }
  }

  private int countInteractiveMfaLogins() {
    List<JsonNode> requests;
    synchronized (wiremockLock) {
      requests = wiremock.getRequests(LOGIN_REQUEST_PATTERN);
    }
    int count = 0;
    for (JsonNode request : requests) {
      JsonNode data = parseJson(request.get("body").asText()).get("data");
      if (data == null) {
        continue;
      }
      JsonNode authenticator = data.get("AUTHENTICATOR");
      if (authenticator == null || !"USERNAME_PASSWORD_MFA".equals(authenticator.asText())) {
        continue;
      }
      JsonNode token = data.get("TOKEN");
      if (token == null || token.asText().isEmpty()) {
        count++;
      }
    }
    return count;
  }

  private static void assertBothSucceeded(boolean[] opened) {
    assertTrue(opened[0]);
    assertTrue(opened[1]);
  }

  private static void joinWatchers(List<Throwable> errors, Thread... threads) {
    joinWatchersQuietly(threads);
    for (Thread thread : threads) {
      assertFalse(thread.isAlive(), "callback watcher did not finish in time");
    }
    if (!errors.isEmpty()) {
      throw new RuntimeException("callback watcher failed", errors.get(0));
    }
  }

  private static void joinWatchersQuietly(Thread... threads) {
    for (Thread thread : threads) {
      try {
        thread.join(TimeUnit.SECONDS.toMillis(20));
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
      }
    }
  }

  private static String uniqueUser(String prefix) {
    return prefix + UUID.randomUUID().toString().replace("-", "");
  }

  private static List<Throwable> synchronizedErrors() {
    return Collections.synchronizedList(new ArrayList<Throwable>());
  }

  private static Properties copy(Properties source) {
    Properties copy = new Properties();
    copy.putAll(source);
    return copy;
  }
}
