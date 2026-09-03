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
import java.util.Objects;
import java.util.Properties;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;
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

  // JVM-wide on purpose: Snowflake rejects TOTP replay within a time window, so any test in this
  // process — including parallel runs — must not resend a code another test already used.
  Set<String> USED_TOTP_CODES = ConcurrentHashMap.newKeySet();
  // Circuit breaker for the shared MFA Jenkins user.
  // 394512: mark + skip this test. Budget exhaust after >=1 submit: mark + fail.
  // Zero submits: fail without marking.
  AtomicBoolean SHARED_MFA_EXHAUSTED = new AtomicBoolean();

  final class CachedTotp {
    final long window;
    final String seed;
    final String code;

    CachedTotp(long window, String seed, String code) {
      this.window = window;
      this.seed = seed;
      this.code = code;
    }

    boolean matches(long otherWindow, String otherSeed) {
      return window == otherWindow && Objects.equals(seed, otherSeed);
    }
  }

  AtomicReference<CachedTotp> CACHED_TOTP = new AtomicReference<>();

  default String acquireTotpPasscode(String seed) {
    Assumptions.assumeFalse(
        isSharedMfaExhausted(), "Shared MFA account already exhausted TOTP retries in this run");
    int advances = 0;
    while (advances < MAX_TOTP_WINDOWS) {
      String passcode = freshTotpCode(seed);
      if (passcode != null) {
        return passcode;
      }
      // Parameterized form: the reference driver has no info(String) overload.
      logger.info("[mfa-helper] {}", "No unused codes in this window, advancing");
      sleepToNextTotpWindow();
      advances++;
    }
    throw new RuntimeException(
        "No unused TOTP passcodes available after " + MAX_TOTP_WINDOWS + " windows");
  }

  /**
   * Connect with USERNAME_PASSWORD_MFA, retrying once per unused TOTP submit. A code already used
   * in this JVM is skipped (does not consume the submit budget); after a retryable rejection, wait
   * only if still in that window rather than submitting adjacent-window codes.
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
      String passcode = freshTotpCode(totpSeed);
      if (passcode == null) {
        if (advances >= MAX_TOTP_WINDOWS) {
          break;
        }
        logger.info("[mfa-helper] {}", "No unused codes in this window, advancing");
        sleepToNextTotpWindow();
        advances++;
        continue;
      }

      long windowId = totpWindowId();
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
      throw new RuntimeException("No unused TOTP passcodes after " + MAX_TOTP_WINDOWS + " windows");
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

  static String freshTotpCode(String seed) {
    String code = getCurrentTotpCode(seed);
    return claimTotpCode(code) ? code : null;
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

  static Path usedCodesPath() {
    return mfaStateDir().resolve("ud-mfa-used-totp-codes");
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

  static boolean fileContainsCode(Path path, String code) throws IOException {
    if (!Files.exists(path)) {
      return false;
    }
    for (String line : Files.readAllLines(path, StandardCharsets.UTF_8)) {
      if (code.equals(line.trim())) {
        return true;
      }
    }
    return false;
  }

  /**
   * Exclusive check-then-append within this JVM; Java channel.lock() and ODBC/Python flock() are
   * independent lock spaces on Linux.
   */
  static boolean claimTotpCode(String code) {
    if (USED_TOTP_CODES.contains(code)) {
      return false;
    }
    try {
      Files.createDirectories(mfaStateDir());
      Path path = usedCodesPath();
      try (FileChannel channel =
              FileChannel.open(
                  path,
                  StandardOpenOption.CREATE,
                  StandardOpenOption.READ,
                  StandardOpenOption.WRITE);
          FileLock ignored = channel.lock()) {
        if (fileContainsCode(path, code)) {
          USED_TOTP_CODES.add(code);
          return false;
        }
        Files.write(
            path, (code + "\n").getBytes(StandardCharsets.UTF_8), StandardOpenOption.APPEND);
        USED_TOTP_CODES.add(code);
        return true;
      }
    } catch (IOException e) {
      logger.warn("[mfa-helper] claimTotpCode failed: {}", e.getMessage());
      return false;
    }
  }

  static String getCurrentTotpCode(String seed) {
    waitIfNearTotpBoundary();
    long window = totpWindowId();
    CachedTotp cached = CACHED_TOTP.get();
    if (cached != null && cached.matches(window, seed)) {
      return cached.code;
    }
    List<String> codes =
        WithNodeScripts.runNodeCapture(TOTP_GENERATOR_SCRIPT, 40, "SNOWFLAKE_AUTH_MFA_SEED", seed);
    List<String> tokens = new ArrayList<>();
    for (String token : codes) {
      if (token.matches("\\d{6}")) {
        tokens.add(token);
      }
    }
    if (tokens.size() == 1) {
      CACHED_TOTP.set(new CachedTotp(totpWindowId(), seed, tokens.get(0)));
      return tokens.get(0);
    }
    if (tokens.size() == 2 || tokens.size() == 3) {
      // Image :4: past/current/future or current/future. Second-to-last is current.
      String code = tokens.get(tokens.size() - 2);
      CACHED_TOTP.set(new CachedTotp(totpWindowId(), seed, code));
      return code;
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
