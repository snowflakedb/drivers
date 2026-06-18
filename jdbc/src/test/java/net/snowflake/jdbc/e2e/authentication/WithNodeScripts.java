package net.snowflake.jdbc.e2e.authentication;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.TimeUnit;

interface WithNodeScripts {

  /** Max time a node script may run before it is forcibly killed and the test fails fast. */
  long NODE_SCRIPT_TIMEOUT_SECONDS = 40;

  /** Run a node script with inherited I/O, asserting a zero exit code. */
  static void runNode(String script, int timeout, String... args) {
    try {
      List<String> command = new ArrayList<>();
      command.add("node");
      command.add(script);
      Collections.addAll(command, args);
      Process process = new ProcessBuilder(command).inheritIO().start();
      int rc = awaitBounded(process, script, timeout);
      if (rc != 0) {
        throw new RuntimeException(script + " failed (rc=" + rc + ")");
      }
    } catch (RuntimeException e) {
      throw e;
    } catch (Exception e) {
      throw new RuntimeException("Failed to run " + script, e);
    }
  }

  /** Run a node script, capture its stdout, and return the whitespace-separated tokens. */
  static List<String> runNodeCapture(String script, int timeout, String... envVars) {
    try {
      ProcessBuilder pb = new ProcessBuilder("node", script);
      for (int i = 0; i + 1 < envVars.length; i += 2) {
        pb.environment().put(envVars[i], envVars[i + 1]);
      }
      pb.redirectErrorStream(true);
      Process process = pb.start();
      StringBuilder output = new StringBuilder();
      try (BufferedReader reader =
          new BufferedReader(new InputStreamReader(process.getInputStream()))) {
        String line;
        while ((line = reader.readLine()) != null) {
          output.append(line).append(" ");
        }
      }
      int rc = awaitBounded(process, script, timeout);
      if (rc != 0) {
        throw new RuntimeException(script + " failed (rc=" + rc + "): " + output.toString().trim());
      }
      List<String> tokens = new ArrayList<>();
      for (String token : output.toString().trim().split("\\s+")) {
        if (!token.isEmpty()) {
          tokens.add(token);
        }
      }
      return tokens;
    } catch (RuntimeException e) {
      throw e;
    } catch (Exception e) {
      throw new RuntimeException("Failed to run " + script, e);
    }
  }

  static int awaitBounded(Process process, String script, int timeout) throws InterruptedException {
    if (!process.waitFor(timeout, TimeUnit.SECONDS)) {
      process.destroyForcibly();
      throw new RuntimeException(script + " timed out after " + timeout + "s");
    }
    return process.exitValue();
  }
}
