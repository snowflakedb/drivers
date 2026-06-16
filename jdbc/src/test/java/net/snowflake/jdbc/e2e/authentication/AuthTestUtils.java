package net.snowflake.jdbc.e2e.authentication;

import static java.util.concurrent.CompletableFuture.runAsync;

import java.net.InetSocketAddress;
import java.net.Socket;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import lombok.experimental.UtilityClass;

@UtilityClass
class AuthTestUtils {

  private static final String PROVIDE_CREDENTIALS_SCRIPT =
      "/externalbrowser/provideBrowserCredentials.js";
  private static final String CLEAN_BROWSER_SCRIPT = "/externalbrowser/cleanBrowserProcesses.js";
  private static final int CHROMIUM_DEBUG_PORT = 9222;

  static CompletableFuture<Void> browserLoginFuture(String login, String password) {
    return runAsync(
        () -> {
          if (!waitForChromium()) {
            throw new IllegalStateException(
                "Chromium did not start on port " + CHROMIUM_DEBUG_PORT + " within timeout");
          }
          runNode(PROVIDE_CREDENTIALS_SCRIPT, "success", login, password);
        });
  }

  static void cleanBrowserProcesses() {
    try {
      runNode(CLEAN_BROWSER_SCRIPT);
    } catch (RuntimeException ignored) {
      // Best-effort cleanup; nothing actionable.
    }
  }

  private static boolean waitForChromium() {
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

  private static void runNode(String script, String... args) {
    try {
      List<String> command = new ArrayList<>();
      command.add("node");
      command.add(script);
      Collections.addAll(command, args);
      Process process = new ProcessBuilder(command).inheritIO().start();
      int rc = process.waitFor();
      if (rc != 0) {
        throw new RuntimeException(script + " failed (rc=" + rc + ")");
      }
    } catch (RuntimeException e) {
      throw e;
    } catch (Exception e) {
      throw new RuntimeException("Failed to run " + script, e);
    }
  }
}
