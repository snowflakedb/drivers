package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;

import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

interface WithTotpCodes extends WithNodeScripts {

  SFLogger logger = SFLoggerFactory.getLogger(WithTotpCodes.class);

  String TOTP_GENERATOR_SCRIPT = "/externalbrowser/totpGenerator.js";
  int TOTP_STEP_SECONDS = 30;
  int MAX_TOTP_WINDOWS = 3;

  // JVM-wide on purpose: Snowflake rejects TOTP replay within a time window, so any test in this
  // process — including parallel runs — must not resend a code another test already used.
  Set<String> USED_TOTP_CODES = ConcurrentHashMap.newKeySet();

  default String acquireTotpPasscode(String seed) {
    for (int window = 0; window < MAX_TOTP_WINDOWS; window++) {
      List<String> fresh = freshTotpCodes(seed);
      if (!fresh.isEmpty()) {
        String passcode = fresh.get(0);
        USED_TOTP_CODES.add(passcode);
        return passcode;
      }
      if (window < MAX_TOTP_WINDOWS - 1) {
        logger.info("[mfa-helper] No unused codes in window {}, advancing", window + 1);
        sleepToNextTotpWindow();
      }
    }
    throw new RuntimeException(
        "No unused TOTP passcodes available after " + MAX_TOTP_WINDOWS + " windows");
  }

  /**
   * Connect with USERNAME_PASSWORD_MFA, retrying across fresh TOTP codes. Codes already used in
   * this JVM are skipped (Snowflake rejects replay within a window); when exhausted, waits for the
   * next window before regenerating.
   */
  default Connection connectWithTotpRetry(
      Properties baseProps, String totpSeed, boolean passcodeInPassword) {
    Exception lastError = null;
    String url = buildJdbcUrl(baseProps);
    String basePassword = baseProps.getProperty("password");

    for (int window = 0; window < MAX_TOTP_WINDOWS; window++) {
      List<String> freshCodes = freshTotpCodes(totpSeed);
      if (freshCodes.isEmpty()) {
        if (window >= MAX_TOTP_WINDOWS - 1) {
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

      if (window < MAX_TOTP_WINDOWS - 1) {
        sleepToNextTotpWindow();
      }
    }

    throw new RuntimeException(
        "Failed to connect after " + MAX_TOTP_WINDOWS + " TOTP windows. Last error: " + lastError,
        lastError);
  }

  static boolean isTotpRetryableError(Exception e) {
    String msg = e.getMessage();
    if (msg == null) {
      return false;
    }
    return msg.contains("TOTP Invalid")
        || msg.toLowerCase(Locale.ROOT).contains("invalid passcode");
  }

  static List<String> freshTotpCodes(String seed) {
    List<String> fresh = new ArrayList<>();
    for (String code : getTotpCodes(seed)) {
      if (!USED_TOTP_CODES.contains(code)) {
        fresh.add(code);
      }
    }
    return fresh;
  }

  static List<String> getTotpCodes(String seed) {
    List<String> codes =
        WithNodeScripts.runNodeCapture(TOTP_GENERATOR_SCRIPT, 40, "SNOWFLAKE_AUTH_MFA_SEED", seed);
    if (codes.isEmpty()) {
      throw new RuntimeException("totpGenerator.js produced no TOTP codes");
    }
    return codes;
  }

  static void sleepToNextTotpWindow() {
    double elapsed = System.currentTimeMillis() / 1000.0;
    double wait = TOTP_STEP_SECONDS - (elapsed % TOTP_STEP_SECONDS) + 1.0;
    logger.info("[mfa-helper] Waiting {}s for next TOTP window", wait);
    try {
      Thread.sleep((long) (wait * 1000));
    } catch (InterruptedException e) {
      Thread.currentThread().interrupt();
    }
  }
}
