package net.snowflake.jdbc.e2e.authentication;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Properties;
import java.util.Set;
import java.util.concurrent.TimeUnit;
import lombok.experimental.UtilityClass;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import net.snowflake.jdbc.utils.TestParameters;

/**
 * TOTP passcode helpers for USERNAME_PASSWORD_MFA E2E tests.
 *
 * <p>Requires the snowdrivers-test-external-browser-universal-driver Docker container
 * (/externalbrowser/totpGenerator.js generates TOTP passcodes for the MFA test user). Mirrors
 * python/tests/e2e/authentication/auth_helpers.py.
 */
@UtilityClass
public class MfaAuthHelpers {

  private static final SFLogger logger = SFLoggerFactory.getLogger(MfaAuthHelpers.class);

  private static final String TOTP_GENERATOR_SCRIPT = "/externalbrowser/totpGenerator.js";
  private static final int TOTP_STEP_SECONDS = 30;

  private static final Set<String> USED_TOTP_CODES = new HashSet<>();

  public static String getMfaParam(String key) throws Exception {
    String envVal = System.getenv(key);
    if (envVal != null && !envVal.isEmpty()) {
      return envVal;
    }
    return TestParameters.get(key);
  }

  public static String acquireTotpPasscode(String seed) {
    return acquireTotpPasscode(seed, 3);
  }

  public static String acquireTotpPasscode(String seed, int maxWindows) {
    for (int window = 0; window < maxWindows; window++) {
      List<String> fresh = freshTotpCodes(seed);
      if (!fresh.isEmpty()) {
        String passcode = fresh.get(0);
        USED_TOTP_CODES.add(passcode);
        return passcode;
      }
      if (window < maxWindows - 1) {
        logger.info("[mfa-helper] No unused codes in window {}, advancing", window + 1);
        sleepToNextTotpWindow();
      }
    }
    throw new RuntimeException(
        "No unused TOTP passcodes available after " + maxWindows + " windows");
  }

  /**
   * Connect using USERNAME_PASSWORD_MFA with TOTP dedup across tests.
   *
   * <p>Snowflake rejects reused TOTP codes within a time window. Codes already consumed in this JVM
   * process are skipped; when exhausted, waits for the next 30s window before regenerating
   * (totpGenerator yields 2-3 codes per window).
   */
  public static Connection connectWithTotpRetry(
      String url, Properties baseProps, String totpSeed, boolean passcodeInPassword) {
    return connectWithTotpRetry(url, baseProps, totpSeed, passcodeInPassword, 3);
  }

  public static Connection connectWithTotpRetry(
      String url,
      Properties baseProps,
      String totpSeed,
      boolean passcodeInPassword,
      int maxWindows) {
    Exception lastError = null;
    String basePassword = baseProps.getProperty("password");

    for (int window = 0; window < maxWindows; window++) {
      List<String> freshCodes = freshTotpCodes(totpSeed);
      if (freshCodes.isEmpty()) {
        if (window >= maxWindows - 1) {
          break;
        }
        logger.info("[mfa-helper] No unused codes in window {}, advancing", window + 1);
        sleepToNextTotpWindow();
        continue;
      }

      for (int codeIdx = 0; codeIdx < freshCodes.size(); codeIdx++) {
        String passcode = freshCodes.get(codeIdx);
        USED_TOTP_CODES.add(passcode);

        Properties props = new Properties();
        props.putAll(baseProps);
        if (passcodeInPassword) {
          props.setProperty("password", basePassword + passcode);
          props.setProperty("passcodeInPassword", "true");
        } else {
          props.setProperty("passcode", passcode);
        }

        try {
          return DriverManager.getConnection(url, props);
        } catch (SQLException e) {
          lastError = e;
          if (isTotpRetryableError(e)) {
            logger.info(
                "[mfa-helper] TOTP code {}/{} in window {} failed, retrying",
                codeIdx + 1,
                freshCodes.size(),
                window + 1);
            continue;
          }
          throw new RuntimeException(e);
        }
      }

      if (window < maxWindows - 1) {
        sleepToNextTotpWindow();
      }
    }

    throw new RuntimeException(
        "Failed to connect after " + maxWindows + " TOTP windows. Last error: " + lastError,
        lastError);
  }

  private static boolean isTotpRetryableError(Exception e) {
    String msg = e.getMessage();
    if (msg == null) {
      return false;
    }
    String lower = msg.toLowerCase();
    return msg.contains("TOTP Invalid") || lower.contains("invalid passcode");
  }

  private static List<String> getTotpCodes(String seed) {
    try {
      ProcessBuilder pb = new ProcessBuilder("node", TOTP_GENERATOR_SCRIPT);
      pb.environment().put("SNOWFLAKE_AUTH_MFA_SEED", seed);
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
      boolean rc = process.waitFor(40, TimeUnit.SECONDS);
      if (!rc) {
        process.destroyForcibly();
        throw new RuntimeException(
            "totpGenerator.js failed (rc=" + rc + "): " + output.toString().trim());
      }
      List<String> codes = new ArrayList<>();
      for (String token : output.toString().trim().split("\\s+")) {
        if (!token.isEmpty()) {
          codes.add(token);
        }
      }
      if (codes.isEmpty()) {
        throw new RuntimeException("totpGenerator.js produced no TOTP codes");
      }
      return codes;
    } catch (RuntimeException e) {
      throw e;
    } catch (Exception e) {
      throw new RuntimeException("Failed to run totpGenerator.js", e);
    }
  }

  private static List<String> freshTotpCodes(String seed) {
    List<String> fresh = new ArrayList<>();
    for (String code : getTotpCodes(seed)) {
      if (!USED_TOTP_CODES.contains(code)) {
        fresh.add(code);
      }
    }
    return fresh;
  }

  private static void sleepToNextTotpWindow() {
    double elapsed = System.currentTimeMillis() / 1000.0;
    double wait = TOTP_STEP_SECONDS - (elapsed % TOTP_STEP_SECONDS);
    if (wait > 0) {
      wait += 1.0;
      logger.info("[mfa-helper] Waiting {}s for next TOTP window", wait);
      try {
        Thread.sleep((long) (wait * 1000));
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
      }
    }
  }
}
