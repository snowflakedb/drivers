package net.snowflake.jdbc.e2e.authentication;

import static java.util.concurrent.CompletableFuture.runAsync;

import java.net.InetSocketAddress;
import java.net.Socket;
import java.sql.Connection;
import java.sql.SQLException;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.concurrent.atomic.AtomicReference;

interface WithBrowserAutomation extends WithNodeScripts {

  String PROVIDE_CREDENTIALS_SCRIPT = "/externalbrowser/provideBrowserCredentials.js";
  String CLEAN_BROWSER_SCRIPT = "/externalbrowser/cleanBrowserProcesses.js";
  int CHROMIUM_DEBUG_PORT = 9222;

  @FunctionalInterface
  interface ConnectSupplier {
    Connection connect() throws SQLException;
  }

  /**
   * Run the connect call and browser automation concurrently. The connect leg is authoritative: its
   * result is returned even if the browser leg fails, and the wait is bounded so a hung connect
   * fails the test instead of blocking forever.
   *
   * @throws SQLException the driver's exception when connect fails, so negative tests can assert on
   *     it
   */
  default Connection connectWithBrowserAutomation(
      ConnectSupplier connectFn, String scenario, String login, String password)
      throws SQLException {
    return connectWithBrowserAutomation(connectFn, scenario, login, password, null);
  }

  /**
   * Same as {@link #connectWithBrowserAutomation(ConnectSupplier, String, String, String)}, but
   * forwards {@code totpSeed} to the browser leg so it can fill Snowflake's authenticator-app MFA
   * verification step, if presented. Pass {@code null} for accounts that don't require MFA.
   */
  default Connection connectWithBrowserAutomation(
      ConnectSupplier connectFn, String scenario, String login, String password, String totpSeed)
      throws SQLException {
    AtomicReference<Connection> connection = new AtomicReference<>();
    AtomicReference<SQLException> connectError = new AtomicReference<>();

    CompletableFuture<Void> connect =
        runAsync(
            () -> {
              try {
                connection.set(connectFn.connect());
              } catch (SQLException e) {
                connectError.set(e);
              }
            });

    CompletableFuture<Void> browser =
        runAsync(
            () -> {
              if (!waitForChromium()) {
                throw new IllegalStateException(
                    "Chromium did not start on port " + CHROMIUM_DEBUG_PORT + " within timeout");
              }
              if (totpSeed != null) {
                WithNodeScripts.runNode(
                    PROVIDE_CREDENTIALS_SCRIPT, 90, scenario, login, password, totpSeed);
              } else {
                WithNodeScripts.runNode(PROVIDE_CREDENTIALS_SCRIPT, 60, scenario, login, password);
              }
            });

    Throwable browserError = awaitQuietly(browser, 120);
    browser.cancel(true);

    try {
      connect.get(90, TimeUnit.SECONDS);
    } catch (TimeoutException e) {
      throw new RuntimeException("Connect thread did not finish within 90s", e);
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      throw new RuntimeException("Interrupted while awaiting connect thread", e);
    } catch (ExecutionException e) {
      throw new RuntimeException("Connect thread failed", e.getCause());
    }

    if (connectError.get() != null) {
      throw connectError.get();
    }
    if (connection.get() != null) {
      return connection.get();
    }

    if (browserError != null) {
      throw new RuntimeException("Browser automation failed", browserError);
    }
    throw new IllegalStateException("Connection was not established");
  }

  default void cleanBrowserProcesses() {
    try {
      WithNodeScripts.runNode(CLEAN_BROWSER_SCRIPT, 30);
    } catch (RuntimeException ignored) {
      // Best-effort cleanup; nothing actionable.
    }
  }

  static Throwable awaitQuietly(CompletableFuture<Void> future, long timeoutSeconds) {
    try {
      future.get(timeoutSeconds, TimeUnit.SECONDS);
      return null;
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
      return e;
    } catch (ExecutionException e) {
      return e.getCause();
    } catch (TimeoutException e) {
      return e;
    }
  }

  static boolean waitForChromium() {
    long deadline = System.nanoTime() + 60000 * 1_000_000L;
    while (System.nanoTime() < deadline) {
      try (Socket sock = new Socket()) {
        sock.connect(new InetSocketAddress("127.0.0.1", CHROMIUM_DEBUG_PORT), 1000);
        return true;
      } catch (Exception ignored) {
        // Not up yet.
      }
      try {
        Thread.sleep(1000);
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        return false;
      }
    }
    return false;
  }
}
