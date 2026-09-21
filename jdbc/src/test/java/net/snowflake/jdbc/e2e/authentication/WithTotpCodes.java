package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.TestParameters.buildJdbcUrl;

import java.io.IOException;
import java.nio.channels.FileChannel;
import java.nio.channels.FileLock;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Properties;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;
import org.junit.jupiter.api.Assumptions;

interface WithTotpCodes extends WithNodeScripts {

  SFLogger logger = SFLoggerFactory.getLogger(WithTotpCodes.class);

  String TOTP_GENERATOR_SCRIPT = "/externalbrowser/totpGenerator.js";
  int TOTP_STEP_SECONDS = 30;
  // Matches totpGenerator.js MIN_VALIDITY_SECONDS. Image :4 does not wait
  // internally; callers must skip a soon-to-expire current window themselves.
  int MIN_TOTP_VALIDITY_SECONDS = 8;
  int MAX_TOTP_WINDOWS = 3;

  // JVM-wide on purpose: Snowflake rejects TOTP replay of the same 30s step, so any test
  // in this process — including parallel runs — must not submit another passcode from a
  // window that was already used. Passcodes themselves are never stored.
  Set<Long> USED_TOTP_WINDOWS = ConcurrentHashMap.newKeySet();
  // Circuit breaker for the shared MFA Jenkins user.
  // 394512: mark + skip this test. Budget exhaust after >=1 submit: mark + fail.
  // Zero submits: fail without marking.
  AtomicBoolean SHARED_MFA_EXHAUSTED = new AtomicBoolean();

  default String acquireTotpPasscode(String seed) {
    Assumptions.assumeFalse(
        isSharedMfaExhausted(), "Shared MFA account already exhausted TOTP retries in this run");
    int advances = 0;
    while (advances < MAX_TOTP_WINDOWS) {
      MintedTotp minted = freshTotpCode(seed);
      if (minted != null) {
        return minted.code;
      }
      // Parameterized form: the reference driver has no info(String) overload.
      logger.info("[mfa-helper] {}", "Current TOTP window already used, advancing");
      sleepToNextTotpWindow();
      advances++;
    }
    throw new RuntimeException(
        "Could not mint a TOTP for an unused window after " + MAX_TOTP_WINDOWS + " windows");
  }

  /**
   * Connect with USERNAME_PASSWORD_MFA, minting a TOTP per attempt. A window already used in this
   * JVM is skipped (does not consume the submit budget); after a retryable rejection, wait only if
   * still in that window, then mint a new passcode.
   */
  default Connection connectWithTotpRetry(
      Properties baseProps, String totpSeed, boolean passcodeInPassword) {
    Assumptions.assumeFalse(
        isSharedMfaExhausted(), "Shared MFA account already exhausted TOTP retries in this run");
    Exception lastError = null;
    String url = buildJdbcUrl(baseProps);
    String basePassword = baseProps.getProperty("password");
    int submits = 0;
    int advances = 0;

    while (submits < MAX_TOTP_WINDOWS) {
      MintedTotp minted = freshTotpCode(totpSeed);
      if (minted == null) {
        if (advances >= MAX_TOTP_WINDOWS) {
          break;
        }
        logger.info("[mfa-helper] {}", "Current TOTP window already used, advancing");
        sleepToNextTotpWindow();
        advances++;
        continue;
      }

      String passcode = minted.code;
      long windowId = minted.windowId;
      submits++;

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
        if (isMfaLockoutError(e)) {
          markSharedMfaExhausted();
          Assumptions.assumeTrue(
              false, "Shared MFA account locked (394512); skipping this and later MFA tests");
        }
        if (!isTotpRetryableError(e)) {
          throw new RuntimeException(e);
        }
        logger.info(
            "[mfa-helper] TOTP submit {} failed; retrying if a fresh window is available", submits);
        if (submits < MAX_TOTP_WINDOWS) {
          sleepIfStillInWindow(windowId);
        }
      }
    }

    if (submits == 0) {
      throw new RuntimeException(
          "Could not mint a TOTP for an unused window after " + MAX_TOTP_WINDOWS + " windows");
    }
    markSharedMfaExhausted();
    throw new RuntimeException(
        "Failed to connect after " + submits + " TOTP submits. Last error: " + lastError,
        lastError);
  }

  static boolean isTotpRetryableError(Exception e) {
    String msg = e.getMessage();
    if (msg == null) {
      return false;
    }
    // 390100 is first-factor rejection, not a bad TOTP. Do not retry it.
    return msg.contains("TOTP Invalid")
        || msg.toLowerCase(Locale.ROOT).contains("invalid passcode");
  }

  static boolean isMfaLockoutError(Exception e) {
    for (Throwable t = e; t != null; t = t.getCause()) {
      if (t instanceof SQLException) {
        SQLException sql = (SQLException) t;
        if (sql.getErrorCode() == 394512) {
          return true;
        }
      }
      String msg = t.getMessage();
      if (msg != null
          && (msg.contains("394512")
              || msg.toLowerCase(Locale.ROOT).contains("too many failed mfa"))) {
        return true;
      }
    }
    return false;
  }

  static long totpWindowId() {
    return System.currentTimeMillis() / 1000 / TOTP_STEP_SECONDS;
  }

  /** Passcode plus the 30s window the digits belong to. */
  class MintedTotp {
    final String code;
    final long windowId;

    MintedTotp(String code, long windowId) {
      this.code = code;
      this.windowId = windowId;
    }
  }

  /**
   * Mint a TOTP and claim the window that code belongs to. Sample the window id after generate:
   * totpGenerator.js (and the pre-generate boundary wait) can cross a 30s step.
   */
  static MintedTotp freshTotpCode(String seed) {
    String code = getCurrentTotpCode(seed);
    long windowId = totpWindowId();
    return claimTotpWindow(windowId) ? new MintedTotp(code, windowId) : null;
  }

  static String mfaBuildTag() {
    String tag = System.getenv("BUILD_TAG");
    return (tag == null || tag.isEmpty()) ? "local" : tag;
  }

  static Path mfaStateDir() {
    String root = System.getenv("WORKSPACE_ROOT");
    if (root == null || root.isEmpty()) {
      root = System.getenv("WORKSPACE");
    }
    if (root == null || root.isEmpty()) {
      root = System.getProperty("java.io.tmpdir");
    }
    return Paths.get(root, ".ud-mfa-totp-state", mfaBuildTag());
  }

  static Path usedWindowsPath() {
    return mfaStateDir().resolve("ud-mfa-used-totp-windows");
  }

  static Path exhaustedFlagPath() {
    return mfaStateDir().resolve("ud-mfa-connect-exhausted");
  }

  static boolean isSharedMfaExhausted() {
    return SHARED_MFA_EXHAUSTED.get() || Files.exists(exhaustedFlagPath());
  }

  static void markSharedMfaExhausted() {
    SHARED_MFA_EXHAUSTED.set(true);
    try {
      Files.createDirectories(mfaStateDir());
      Files.write(
          exhaustedFlagPath(),
          "1\n".getBytes(StandardCharsets.UTF_8),
          StandardOpenOption.CREATE,
          StandardOpenOption.TRUNCATE_EXISTING);
    } catch (IOException ignored) {
      // In-memory flag still stops later tests in this JVM.
    }
  }

  static boolean fileContainsWindow(Path path, long windowId) throws IOException {
    if (!Files.exists(path)) {
      return false;
    }
    String expected = Long.toString(windowId);
    for (String line : Files.readAllLines(path, StandardCharsets.UTF_8)) {
      if (expected.equals(line.trim())) {
        return true;
      }
    }
    return false;
  }

  /**
   * Exclusive check-then-append of a TOTP window id within this JVM; Java channel.lock() and
   * ODBC/Python flock() are independent lock spaces on Linux.
   */
  static boolean claimTotpWindow(long windowId) {
    if (USED_TOTP_WINDOWS.contains(windowId)) {
      return false;
    }
    try {
      Files.createDirectories(mfaStateDir());
      Path path = usedWindowsPath();
      try (FileChannel channel =
              FileChannel.open(
                  path,
                  StandardOpenOption.CREATE,
                  StandardOpenOption.READ,
                  StandardOpenOption.WRITE);
          FileLock ignored = channel.lock()) {
        if (fileContainsWindow(path, windowId)) {
          USED_TOTP_WINDOWS.add(windowId);
          return false;
        }
        Files.write(
            path, (windowId + "\n").getBytes(StandardCharsets.UTF_8), StandardOpenOption.APPEND);
        USED_TOTP_WINDOWS.add(windowId);
        return true;
      }
    } catch (IOException e) {
      logger.warn("[mfa-helper] claimTotpWindow failed: {}", e.getMessage());
      return false;
    }
  }

  static String getCurrentTotpCode(String seed) {
    waitIfNearTotpBoundary();
    List<String> codes =
        WithNodeScripts.runNodeCapture(TOTP_GENERATOR_SCRIPT, 40, "SNOWFLAKE_AUTH_MFA_SEED", seed);
    List<String> tokens = new ArrayList<>();
    for (String token : codes) {
      if (token.matches("\\d{6}")) {
        tokens.add(token);
      }
    }
    if (tokens.size() == 1) {
      return tokens.get(0);
    }
    if (tokens.size() == 2 || tokens.size() == 3) {
      // Image :4: past/current/future or current/future. Second-to-last is current.
      return tokens.get(tokens.size() - 2);
    }
    throw new RuntimeException(
        "totpGenerator.js produced " + tokens.size() + " 6-digit tokens; expected 1 or 2-3");
  }

  static void waitIfNearTotpBoundary() {
    double remaining =
        TOTP_STEP_SECONDS - (System.currentTimeMillis() / 1000.0 % TOTP_STEP_SECONDS);
    if (remaining < MIN_TOTP_VALIDITY_SECONDS) {
      try {
        Thread.sleep((long) ((remaining + 1.0) * 1000));
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new RuntimeException("Interrupted while waiting for a safe TOTP window", e);
      }
    }
  }

  static void sleepToNextTotpWindow() {
    double elapsed = System.currentTimeMillis() / 1000.0;
    double wait = TOTP_STEP_SECONDS - (elapsed % TOTP_STEP_SECONDS);
    if (wait > 0) {
      wait += 1.0;
      logger.info("[mfa-helper] Waiting {}s for next TOTP window", wait);
      try {
        Thread.sleep((long) (wait * 1000));
      } catch (InterruptedException e) {
        Thread.currentThread().interrupt();
        throw new RuntimeException("Interrupted while waiting for the next TOTP window", e);
      }
    }
  }

  static void sleepIfStillInWindow(long windowId) {
    if (totpWindowId() == windowId) {
      sleepToNextTotpWindow();
    }
  }
}
