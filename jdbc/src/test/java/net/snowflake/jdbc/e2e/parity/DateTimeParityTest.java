package net.snowflake.jdbc.e2e.parity;

import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.EnumMap;
import java.util.EnumSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.parallel.Execution;
import org.junit.jupiter.api.parallel.ExecutionMode;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

/**
 * Single-pass parity oracle: runs each (type, timezone, output-format, profile) cell against
 * universal-driver and snowflake-jdbc 4.3.1 in the same JVM via isolated classloaders, and asserts
 * byte-identical output cell-by-cell.
 *
 * <p>Two families of test methods, split per Snowflake type so each type can be enabled or disabled
 * independently (e.g. {@code readDateParity}/{@code writeDateParity}, {@code readTimeParity}, the
 * three {@code readTimestamp*Parity} flavours, etc.):
 *
 * <ul>
 *   <li>{@code read<Type>Parity} - one multi-column SELECT bakes every (scale, value) column. For
 *       each column we exercise every {@link GetSink} on both driver result sets and assert the
 *       outcomes match. The {@link Profile} dimension is the full Cartesian product of the boolean
 *       session parameters that affect that type's getters.
 *   <li>{@code write<Type>Parity} - one bind-only SELECT with every (scale, value, setSink)
 *       combination as a parameter. Each parameter is bound on both drivers; the round-tripped
 *       string is compared. Profiles are NOT varied here today: the bind-relevant flags are
 *       connection-time only (see below).
 * </ul>
 *
 * <p>Per-cell mismatches are accumulated and reported together at the end of each test method, so a
 * single run surfaces every divergence rather than aborting on the first.
 *
 * <p>Run via {@code ./gradlew :jdbc:parityTest}. Requires {@code PARAMETER_PATH} pointing at a
 * Snowflake test-account credentials JSON. Both connections are forced onto Arrow result format.
 *
 * <h2>Connection-time params (covered via dedicated tests)</h2>
 *
 * The following params affect date/time/timestamp handling but are read once from the JDBC {@code
 * Properties} bag at connect time and never refreshed from server responses — an {@code ALTER
 * SESSION SET} of these is silently ignored by the legacy driver. They therefore cannot be varied
 * by the shared {@code readParity}/{@code writeParity} matrix; each gets its own focused test that
 * opens a dedicated connection pair via {@link ParityHarness#sessionsFor(java.util.Map)} for the
 * {@code true} and {@code false} value, over the type(s) and timezones the param actually affects:
 *
 * <ul>
 *   <li>{@code JDBC_GET_DATE_USE_NULL_TIMEZONE} (Boolean, default {@code true}) — controls how
 *       {@code getDate(int[, Calendar])} interprets a null/JVM-default Calendar. Covered by {@link
 *       #getDateNullTimezoneParity}.
 *   <li>{@code CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME} (Boolean, default {@code false}) — when true,
 *       {@code setTime(...)} binds the JVM-local wall-clock fields instead of treating {@code
 *       Time.getTime()} as UTC ms reduced modulo a day. Bind path for TIME. Covered by {@link
 *       #treatTimeAsWallClockParity}.
 *   <li>{@code JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE} (Boolean, default {@code false}) —
 *       connect-time fallback default for {@code JDBC_FORMAT_DATE_WITH_TIMEZONE}, consulted by the
 *       legacy driver's {@code DateConverter.getUseDateFormat(...)} when the server has not pushed
 *       the runtime value. Covered by {@link #defaultFormatDateWithTimezoneParity} (which keeps
 *       {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} unset so the fallback is actually reached).
 * </ul>
 *
 * <h2>Trap values</h2>
 *
 * The {@code VALUES} matrix below mixes happy-path values with dates known to diverge between the
 * server's proleptic-Gregorian calendar and {@code java.util.GregorianCalendar} (which the legacy
 * driver's date math runs through): the Julian↔Gregorian cutover (1582-10-04/15, 1752-09-14),
 * pre-1883 local-mean-time offsets for named zones (1800-01-01), the year-0001 and year-9999
 * boundaries, the day before the Unix epoch (negative-ms sign bugs), and 1900 (a non-leap year that
 * several legacy date libraries mishandle).
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@Execution(ExecutionMode.SAME_THREAD)
public class DateTimeParityTest {

  private static final List<GetSink> GET_SINKS = Arrays.asList(GetSink.values());
  private static final List<SetSink> SET_SINKS = Arrays.asList(SetSink.values());

  /**
   * Snowflake types whose parity is currently disabled. The per-type {@code read<Type>Parity} and
   * {@code write<Type>Parity} methods carry an equivalent {@code @Disabled} annotation; this set
   * additionally gates the connection-time focused tests below ({@code getDateNullTimezoneParity}
   * etc.), whose type is a runtime parameter and so cannot be disabled per-type via an annotation.
   * Keep the two in sync: re-enabling a type means both dropping its {@code @Disabled} and removing
   * it here. All three timestamp flavours are now fully enabled (read: P1–P3, write: P4), so the
   * set is empty; add a type back here (and re-add its {@code @Disabled}) to disable it again.
   */
  private static final Set<SfType> DISABLED_TYPES = EnumSet.noneOf(SfType.class);

  private ParityHarness harness;

  @BeforeAll
  void setUp() throws Exception {
    harness = ParityHarness.open();
  }

  @AfterAll
  void tearDown() {
    if (harness != null) {
      harness.close();
      harness = null;
    }
  }

  @ParameterizedTest(name = "READ DATE tz={0} fmt={1} profile={2}")
  @MethodSource("dateReadCells")
  void readDateParity(String tz, String fmt, Profile profile) throws Exception {
    runReadParity(SfType.DATE, tz, fmt, profile);
  }

  @ParameterizedTest(name = "READ TIME tz={0} fmt={1} profile={2}")
  @MethodSource("timeReadCells")
  void readTimeParity(String tz, String fmt, Profile profile) throws Exception {
    runReadParity(SfType.TIME, tz, fmt, profile);
  }

  @ParameterizedTest(name = "READ TIMESTAMP_NTZ tz={0} fmt={1} profile={2}")
  @MethodSource("timestampNtzReadCells")
  void readTimestampNtzParity(String tz, String fmt, Profile profile) throws Exception {
    runReadParity(SfType.TIMESTAMP_NTZ, tz, fmt, profile);
  }

  @ParameterizedTest(name = "READ TIMESTAMP_LTZ tz={0} fmt={1} profile={2}")
  @MethodSource("timestampLtzReadCells")
  void readTimestampLtzParity(String tz, String fmt, Profile profile) throws Exception {
    runReadParity(SfType.TIMESTAMP_LTZ, tz, fmt, profile);
  }

  @ParameterizedTest(name = "READ TIMESTAMP_TZ tz={0} fmt={1} profile={2}")
  @MethodSource("timestampTzReadCells")
  void readTimestampTzParity(String tz, String fmt, Profile profile) throws Exception {
    runReadParity(SfType.TIMESTAMP_TZ, tz, fmt, profile);
  }

  /** Shared read-parity body for the per-type {@code read<Type>Parity} methods. */
  private void runReadParity(SfType type, String tz, String fmt, Profile profile) throws Exception {
    assumeTrue(typeEnabled(type), () -> type + " disabled via parity.types");
    List<String> failures = new ArrayList<>();
    runReadCells(
        harness.sessionsFor(Collections.emptyMap()),
        type,
        tz,
        fmt,
        profile.overlay(),
        GET_SINKS,
        "profile=" + profile,
        failures);
    if (!failures.isEmpty()) {
      fail(buildReport(failures));
    }
  }

  /**
   * Run a single multi-column read query on the given pair under (tz, output-format, overlay), and
   * compare every (cell, sink) outcome between drivers. {@code variant} is a free-form token (e.g.
   * {@code "profile=..."} or {@code "nullTz=true"}) spliced into each failure line to identify the
   * dimension under test.
   */
  private void runReadCells(
      ParityHarness.SessionPair pair,
      SfType type,
      String tz,
      String fmt,
      Map<String, String> overlay,
      List<GetSink> sinks,
      String variant,
      List<String> failures)
      throws Exception {
    pair.applyBoth(tz, type.outputFormatParam(), fmt, overlay);
    ReadLayout layout = ReadLayout.build(type, SCALES.get(type), VALUES.get(type));

    Connection newConn = pair.newSession().connection();
    Connection oldConn = pair.oldSession().connection();
    try (Statement sNew = newConn.createStatement();
        Statement sOld = oldConn.createStatement();
        ResultSet rsNew = sNew.executeQuery(layout.sql);
        ResultSet rsOld = sOld.executeQuery(layout.sql)) {
      assertTrue(rsNew.next(), "no row from new driver");
      assertTrue(rsOld.next(), "no row from old driver");

      for (ReadLayout.Cell cell : layout.cells) {
        for (GetSink sink : sinks) {
          Outcome newOutcome = readOnce(rsNew, cell.columnIdx, sink);
          Outcome oldOutcome = readOnce(rsOld, cell.columnIdx, sink);
          if (!oldOutcome.equals(newOutcome)) {
            failures.add(
                "READ "
                    + type
                    + " tz="
                    + tz
                    + " fmt="
                    + fmt
                    + " "
                    + variant
                    + " scale="
                    + cell.scale
                    + " val='"
                    + cell.value
                    + "' sink="
                    + sink
                    + "\n  legacy: "
                    + oldOutcome
                    + "\n  new:    "
                    + newOutcome);
          }
        }
      }
    }
  }

  @ParameterizedTest(name = "WRITE DATE tz={0} fmt={1}")
  @MethodSource("dateWriteCells")
  void writeDateParity(String tz, String fmt) throws Exception {
    runWriteParity(SfType.DATE, tz, fmt);
  }

  @ParameterizedTest(name = "WRITE TIME tz={0} fmt={1}")
  @MethodSource("timeWriteCells")
  void writeTimeParity(String tz, String fmt) throws Exception {
    runWriteParity(SfType.TIME, tz, fmt);
  }

  @ParameterizedTest(name = "WRITE TIMESTAMP_NTZ tz={0} fmt={1}")
  @MethodSource("timestampNtzWriteCells")
  void writeTimestampNtzParity(String tz, String fmt) throws Exception {
    runWriteParity(SfType.TIMESTAMP_NTZ, tz, fmt);
  }

  @ParameterizedTest(name = "WRITE TIMESTAMP_LTZ tz={0} fmt={1}")
  @MethodSource("timestampLtzWriteCells")
  void writeTimestampLtzParity(String tz, String fmt) throws Exception {
    runWriteParity(SfType.TIMESTAMP_LTZ, tz, fmt);
  }

  @ParameterizedTest(name = "WRITE TIMESTAMP_TZ tz={0} fmt={1}")
  @MethodSource("timestampTzWriteCells")
  void writeTimestampTzParity(String tz, String fmt) throws Exception {
    runWriteParity(SfType.TIMESTAMP_TZ, tz, fmt);
  }

  /** Shared write-parity body for the per-type {@code write<Type>Parity} methods. */
  private void runWriteParity(SfType type, String tz, String fmt) throws Exception {
    assumeTrue(typeEnabled(type), () -> type + " disabled via parity.types");
    ParityHarness.SessionPair pair = harness.sessionsFor(Collections.emptyMap());
    pair.applyBoth(tz, type.outputFormatParam(), fmt, Profile.DEFAULT.overlay());

    // Issue one query per setSink instead of cramming every (value x scale x sink) bind into one
    // request. Universal-driver's JNI bridge has tripped a stack overflow on the Rust handler
    // when the protobuf payload carries hundreds of binds; per-sink chunking keeps requests
    // small and gives clearer per-sink failure attribution.
    List<String> failures = new ArrayList<>();
    for (SetSink sink : SET_SINKS) {
      runWriteChunk(
          pair,
          type,
          tz,
          fmt,
          "",
          WriteLayout.build(
              type, SCALES.get(type), VALUES.get(type), Collections.singletonList(sink)),
          failures);
    }
    if (!failures.isEmpty()) {
      fail(buildReport(failures));
    }
  }

  private void runWriteChunk(
      ParityHarness.SessionPair pair,
      SfType type,
      String tz,
      String fmt,
      String variant,
      WriteLayout layout,
      List<String> failures)
      throws Exception {
    Connection newConn = pair.newSession().connection();
    Connection oldConn = pair.oldSession().connection();

    Outcome[] newBindOutcomes = new Outcome[layout.cells.size()];
    Outcome[] oldBindOutcomes = new Outcome[layout.cells.size()];
    Outcome[] newReadOutcomes = new Outcome[layout.cells.size()];
    Outcome[] oldReadOutcomes = new Outcome[layout.cells.size()];

    PreparedStatement psNew = null;
    PreparedStatement psOld = null;
    try {
      // Prepare each driver independently. A prepare failure on one side (e.g. a legacy-driver
      // limitation on a given cast) must not abort the chunk: stamp that side's binds with the
      // error so it surfaces as a per-cell parity row (old: ERR vs new: <value>) instead of an
      // exception that hides the other driver's behavior entirely.
      try {
        psNew = newConn.prepareStatement(layout.sql);
      } catch (Exception e) {
        stampAll(newBindOutcomes, Outcome.error(e));
      }
      try {
        psOld = oldConn.prepareStatement(layout.sql);
      } catch (Exception e) {
        stampAll(oldBindOutcomes, Outcome.error(e));
      }

      for (int i = 0; i < layout.cells.size(); i++) {
        WriteLayout.Cell cell = layout.cells.get(i);
        if (psNew != null) {
          newBindOutcomes[i] = bindOnce(psNew, type, cell);
        }
        if (psOld != null) {
          oldBindOutcomes[i] = bindOnce(psOld, type, cell);
        }
      }

      ResultSet rsNew = null;
      ResultSet rsOld = null;
      try {
        if (psNew != null) {
          rsNew = psNew.executeQuery();
        }
        if (psOld != null) {
          rsOld = psOld.executeQuery();
        }
        if (rsNew != null && rsOld != null && rsNew.next() && rsOld.next()) {
          for (int i = 0; i < layout.cells.size(); i++) {
            WriteLayout.Cell cell = layout.cells.get(i);
            newReadOutcomes[i] = readOnce(rsNew, cell.columnIdx, GetSink.GET_STRING);
            oldReadOutcomes[i] = readOnce(rsOld, cell.columnIdx, GetSink.GET_STRING);
          }
        }
      } catch (Exception e) {
        // Whole-query failure: stamp every cell with the error so per-cell parity still surfaces.
        for (int i = 0; i < layout.cells.size(); i++) {
          if (newReadOutcomes[i] == null) {
            newReadOutcomes[i] = Outcome.error(e);
          }
          if (oldReadOutcomes[i] == null) {
            oldReadOutcomes[i] = Outcome.error(e);
          }
        }
      } finally {
        closeQuietly(rsNew);
        closeQuietly(rsOld);
      }
    } finally {
      closeQuietly(psNew);
      closeQuietly(psOld);
    }

    for (int i = 0; i < layout.cells.size(); i++) {
      WriteLayout.Cell cell = layout.cells.get(i);
      if (!oldBindOutcomes[i].equals(newBindOutcomes[i])) {
        failures.add(
            "WRITE-BIND "
                + type
                + " tz="
                + tz
                + " fmt="
                + fmt
                + variantSuffix(variant)
                + " scale="
                + cell.scale
                + " val='"
                + cell.value
                + "' setSink="
                + cell.sink
                + "\n  legacy: "
                + oldBindOutcomes[i]
                + "\n  new:    "
                + newBindOutcomes[i]);
        continue;
      }
      Outcome newRead = newReadOutcomes[i];
      Outcome oldRead = oldReadOutcomes[i];
      if (newRead == null || oldRead == null) {
        // Bind succeeded on both but query was not even run on at least one (paired execute
        // failure already recorded above); skip.
        continue;
      }
      if (!oldRead.equals(newRead)) {
        failures.add(
            "WRITE-READ "
                + type
                + " tz="
                + tz
                + " fmt="
                + fmt
                + variantSuffix(variant)
                + " scale="
                + cell.scale
                + " val='"
                + cell.value
                + "' setSink="
                + cell.sink
                + "\n  legacy: "
                + oldRead
                + "\n  new:    "
                + newRead);
      }
    }
  }

  // --------------------------------------------------------------------------------
  // Connection-time params (each needs its own connection pair; see class Javadoc)
  // --------------------------------------------------------------------------------

  /**
   * {@code JDBC_GET_DATE_USE_NULL_TIMEZONE} governs how {@code getDate(int[, Calendar])} interprets
   * a null/JVM-default Calendar. Read-side; exercised on DATE and every TIMESTAMP flavour against
   * the full timezone axis, for both flag values.
   */
  @ParameterizedTest(name = "GET_DATE_NULL_TZ {0} nullTz={1} tz={2}")
  @MethodSource("getDateNullTzCells")
  void getDateNullTimezoneParity(SfType type, boolean nullTz, String tz) throws Exception {
    ParityHarness.SessionPair pair =
        harness.sessionsFor(
            Collections.singletonMap("JDBC_GET_DATE_USE_NULL_TIMEZONE", Boolean.toString(nullTz)));
    List<String> failures = new ArrayList<>();
    runReadCells(
        pair,
        type,
        tz,
        representativeFormat(type),
        Profile.DEFAULT.overlay(),
        Arrays.asList(GetSink.GET_STRING, GetSink.GET_DATE, GetSink.GET_DATE_CAL_UTC),
        "nullTz=" + nullTz,
        failures);
    if (!failures.isEmpty()) {
      fail(buildReport(failures));
    }
  }

  /**
   * {@code JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE} is the connect-time fallback default for {@code
   * JDBC_FORMAT_DATE_WITH_TIMEZONE}, consulted only when the runtime value was not pushed by the
   * server. We therefore keep that runtime flag OUT of the session overlay (default profile) so the
   * fallback can actually be reached. Read-side; DATE and the timezone-formatted timestamps.
   */
  @ParameterizedTest(name = "DEFAULT_FMT_DATE_TZ {0} defaultFmtTz={1} tz={2}")
  @MethodSource("defaultFormatDateWithTzCells")
  void defaultFormatDateWithTimezoneParity(SfType type, boolean defaultFmtTz, String tz)
      throws Exception {
    ParityHarness.SessionPair pair =
        harness.sessionsFor(
            Collections.singletonMap(
                "JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE", Boolean.toString(defaultFmtTz)));
    List<String> failures = new ArrayList<>();
    runReadCells(
        pair,
        type,
        tz,
        representativeFormat(type),
        Profile.DEFAULT.overlay(),
        Arrays.asList(GetSink.GET_STRING, GetSink.GET_DATE),
        "defaultFmtTz=" + defaultFmtTz,
        failures);
    if (!failures.isEmpty()) {
      fail(buildReport(failures));
    }
  }

  /**
   * {@code CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME} changes how {@code setTime(...)} serializes its
   * argument. Write-side, TIME only; exercises the two {@code setTime} sinks against the full
   * timezone axis for both flag values.
   */
  @ParameterizedTest(name = "TREAT_TIME_WALL_CLOCK wallClock={1} tz={2}")
  @MethodSource("treatTimeWallClockCells")
  void treatTimeAsWallClockParity(SfType type, boolean wallClock, String tz) throws Exception {
    ParityHarness.SessionPair pair =
        harness.sessionsFor(
            Collections.singletonMap(
                "CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME", Boolean.toString(wallClock)));
    String fmt = representativeFormat(type);
    pair.applyBoth(tz, type.outputFormatParam(), fmt, Profile.DEFAULT.overlay());

    List<String> failures = new ArrayList<>();
    for (SetSink sink : Arrays.asList(SetSink.SET_TIME, SetSink.SET_TIME_CAL_UTC)) {
      runWriteChunk(
          pair,
          type,
          tz,
          fmt,
          "wallClock=" + wallClock,
          WriteLayout.build(
              type, SCALES.get(type), VALUES.get(type), Collections.singletonList(sink)),
          failures);
    }
    if (!failures.isEmpty()) {
      fail(buildReport(failures));
    }
  }

  // --------------------------------------------------------------------------------
  // Helpers
  // --------------------------------------------------------------------------------

  /** Fill every slot of an outcome array with the same value (used when a prepare fails). */
  private static void stampAll(Outcome[] outcomes, Outcome value) {
    for (int i = 0; i < outcomes.length; i++) {
      outcomes[i] = value;
    }
  }

  private static Outcome readOnce(ResultSet rs, int col, GetSink sink) {
    try {
      return Outcome.value(sink.read(rs, col));
    } catch (Throwable t) {
      return Outcome.error(t);
    }
  }

  private static Outcome bindOnce(PreparedStatement ps, SfType type, WriteLayout.Cell cell) {
    try {
      cell.sink.bind(ps, cell.paramIdx, type, cell.value);
      return Outcome.value("BOUND");
    } catch (Throwable t) {
      return Outcome.error(t);
    }
  }

  private static void closeQuietly(AutoCloseable c) {
    if (c == null) {
      return;
    }
    try {
      c.close();
    } catch (Exception ignore) {
      // ignore
    }
  }

  private static String buildReport(List<String> failures) {
    StringBuilder sb = new StringBuilder("PARITY MISMATCHES (");
    sb.append(failures.size()).append("):\n\n");
    for (String f : failures) {
      sb.append(f).append("\n\n");
    }
    return sb.toString();
  }

  /** Render an optional variant token as a leading-space suffix, or "" when blank. */
  private static String variantSuffix(String variant) {
    return (variant == null || variant.isEmpty()) ? "" : " " + variant;
  }

  // --------------------------------------------------------------------------------
  // Matrix
  // --------------------------------------------------------------------------------

  private static final Map<SfType, List<String>> TIMEZONES = new EnumMap<>(SfType.class);
  private static final Map<SfType, List<String>> FORMATS = new EnumMap<>(SfType.class);
  private static final Map<SfType, List<Integer>> SCALES = new EnumMap<>(SfType.class);
  private static final Map<SfType, List<String>> VALUES = new EnumMap<>(SfType.class);

  /**
   * Session-timezone axis. Picked to surface different shapes of legacy/new divergence:
   *
   * <ul>
   *   <li>{@code UTC} — baseline, exercises identity-shift code paths
   *   <li>{@code America/New_York} — DST in the northern hemisphere, negative offset
   *   <li>{@code Asia/Tokyo} — no DST, large positive offset
   *   <li>{@code Asia/Kolkata} — half-hour offset (+05:30), surfaces sub-hour arithmetic bugs
   *   <li>{@code Australia/Sydney} — DST in the southern hemisphere (transitions reversed vs. NA)
   * </ul>
   */
  private static final List<String> TZ_FULL =
      Arrays.asList("UTC", "America/New_York", "Asia/Tokyo", "Asia/Kolkata", "Australia/Sydney");

  static {
    // ---- DATE ----
    TIMEZONES.put(SfType.DATE, TZ_FULL);
    FORMATS.put(
        SfType.DATE, Arrays.asList("YYYY-MM-DD", "DD-MON-YYYY", "DY, DD MON YYYY", "MM/DD/YYYY"));
    SCALES.put(SfType.DATE, Collections.singletonList(0));
    VALUES.put(
        SfType.DATE,
        Arrays.asList(
            // happy path
            "2024-01-15",
            "1970-01-01",
            "1999-12-31",
            "2000-02-29",
            // boundaries of the supported range
            "0001-01-01",
            "9999-12-31",
            // Julian -> Gregorian cutover: the proleptic-Gregorian server keeps these distinct,
            // but java.util.GregorianCalendar applies the 1582 cutover (10 absent days).
            "1582-10-04",
            "1582-10-15",
            // British/American adoption of the Gregorian calendar (1752 cutover, 11 absent days).
            "1752-09-14",
            // pre-1883 dates carry local-mean-time offsets for named zones (e.g. -04:56 for NY).
            "1800-01-01",
            // US Standard Time introduction (railway time) -- offset changes around this instant.
            "1883-11-18",
            // 1900 is NOT a leap year; legacy 2-digit-epoch math sometimes treats it as one.
            "1899-12-31",
            "1900-01-01",
            // day before the Unix epoch -- negative epoch-ms, a classic sign-handling trap.
            "1969-12-31"));

    // ---- TIME ----
    TIMEZONES.put(SfType.TIME, TZ_FULL);
    FORMATS.put(
        SfType.TIME,
        Arrays.asList(
            "HH24:MI:SS",
            "HH24:MI:SS.FF",
            "HH24:MI:SS.FF0",
            "HH24:MI:SS.FF3",
            "HH24:MI:SS.FF9",
            "HH12:MI:SS AM",
            "HH12:MI:SS.FF3 AM"));
    SCALES.put(SfType.TIME, Arrays.asList(0, 3, 6, 9));
    VALUES.put(
        SfType.TIME,
        Arrays.asList(
            "00:00:00",
            "12:34:56.789012345",
            "23:59:59.999999999",
            "12:00:00.000000001",
            // smallest non-zero nanosecond at the start of the day -- sub-ms truncation trap.
            "00:00:00.000000001",
            // afternoon value -- exercises the 12h/AM-PM (HH12 ... AM) output formats.
            "13:00:00"));

    // ---- TIMESTAMP_NTZ ----
    TIMEZONES.put(SfType.TIMESTAMP_NTZ, TZ_FULL);
    FORMATS.put(
        SfType.TIMESTAMP_NTZ,
        Arrays.asList(
            "YYYY-MM-DD HH24:MI:SS",
            "YYYY-MM-DD HH24:MI:SS.FF",
            "YYYY-MM-DD HH24:MI:SS.FF3",
            "DY, DD MON YYYY HH24:MI:SS.FF",
            "YYYY-MM-DD\"T\"HH24:MI:SS.FF"));
    SCALES.put(SfType.TIMESTAMP_NTZ, Arrays.asList(0, 3, 6, 9));
    VALUES.put(
        SfType.TIMESTAMP_NTZ,
        Arrays.asList(
            "2024-01-15 12:34:56.789012345",
            "1970-01-01 00:00:00",
            "9999-12-31 23:59:59.999999999",
            // Gregorian-cutover day and the instant just before it (proleptic vs.
            // GregorianCalendar).
            "1582-10-15 00:00:00",
            "1582-10-04 23:59:59.999999999",
            // pre-1883 local-mean-time offset territory for named session zones.
            "1800-01-01 00:00:00",
            // US Standard Time introduction.
            "1883-11-18 12:00:00",
            // day before the epoch -- negative epoch-ms with sub-second nanos.
            "1969-12-31 23:59:59.999999999",
            // lower boundary of the supported range.
            "0001-01-01 00:00:00",
            // DST spring-forward gap (02:30 does not exist) and fall-back overlap (01:30 twice):
            // NTZ is the type whose honor-client-TZ re-anchoring routes through
            // ArrowResultUtil.moveToTimeZone, so these exercise the offset math at a transition
            // under a DST-observing session zone (P5 DST-correction decision).
            "2024-03-10 02:30:00",
            "2024-11-03 01:30:00",
            // far-future (>2500): confirms tz-offset resolution without the legacy 400-year-cycle
            // remap (P5 far-future decision).
            "2600-01-01 12:00:00",
            "3000-06-15 08:30:00.123456789"));

    // ---- TIMESTAMP_LTZ ----
    TIMEZONES.put(SfType.TIMESTAMP_LTZ, TZ_FULL);
    FORMATS.put(
        SfType.TIMESTAMP_LTZ,
        Arrays.asList(
            "YYYY-MM-DD HH24:MI:SS TZHTZM",
            "YYYY-MM-DD HH24:MI:SS.FF TZHTZM",
            "YYYY-MM-DD HH24:MI:SS TZH:TZM",
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM"));
    SCALES.put(SfType.TIMESTAMP_LTZ, Arrays.asList(0, 3, 9));
    VALUES.put(
        SfType.TIMESTAMP_LTZ,
        Arrays.asList(
            // spring-forward gap (02:30 does not exist) and fall-back overlap (01:30 twice) in NY.
            "2024-03-10 02:30:00",
            "2024-11-03 01:30:00",
            "2024-01-15 12:34:56.789",
            "1970-01-01 00:00:00",
            // Gregorian cutover under a session timezone.
            "1582-10-15 00:00:00",
            // pre-1883 local-mean-time offsets.
            "1800-01-01 00:00:00",
            // day before the epoch.
            "1969-12-31 23:59:59.999999999",
            // supported-range boundaries.
            "0001-01-01 00:00:00",
            "9999-12-31 23:59:59",
            // far-future (>2500) under a DST-observing session zone: confirms LTZ rendering in the
            // session zone without the legacy 400-year-cycle tz remap (P5 far-future decision).
            "2600-06-15 12:00:00",
            "2999-01-01 00:00:00"));

    // ---- TIMESTAMP_TZ ----
    TIMEZONES.put(SfType.TIMESTAMP_TZ, TZ_FULL);
    FORMATS.put(
        SfType.TIMESTAMP_TZ,
        Arrays.asList(
            "YYYY-MM-DD HH24:MI:SS TZHTZM",
            "YYYY-MM-DD HH24:MI:SS.FF TZHTZM",
            "YYYY-MM-DD HH24:MI:SS TZH:TZM",
            "YYYY-MM-DD HH24:MI:SS.FF3 TZHTZM"));
    SCALES.put(SfType.TIMESTAMP_TZ, Arrays.asList(0, 3, 9));
    VALUES.put(
        SfType.TIMESTAMP_TZ,
        Arrays.asList(
            "2024-01-15 12:34:56.789 -05:00",
            "2024-01-15 12:34:56.789 +09:00",
            // half-hour offset -- exercises the non-zero TZM minute component of the tz-index
            // decode and the TZH:TZM / TZHTZM suffix rendering.
            "2024-06-01 12:00:00 +05:30",
            "1970-01-01 00:00:00 +00:00",
            "2024-03-10 02:30:00 -05:00",
            // supported-range boundaries with explicit offsets.
            "0001-01-01 00:00:00 +00:00",
            "9999-12-31 23:59:59.999999999 +14:00",
            // Gregorian cutover carried with an explicit offset.
            "1582-10-15 00:00:00 +00:00",
            // day before the epoch with negative-zero offset.
            "1969-12-31 23:59:59.999999999 -00:00",
            // extreme negative offset (min supported is -12:00).
            "2024-01-15 12:34:56.789 -12:00",
            // far-future (>2500) with explicit offsets: confirms the stored-offset render path
            // without the legacy 400-year-cycle tz remap (P5 far-future decision).
            "2600-01-01 12:00:00 +05:30",
            "3000-06-15 08:30:00.123456789 -08:00"));
  }

  // ---- Profiles ----
  // Per-type Cartesian over the boolean session params that affect that type's getters. The
  // matrix iterates the full 2^N product so any drift in flag handling surfaces; non-boolean
  // params (output formats, TIMEZONE) are kept as separate axes above.
  private static final Map<SfType, List<Profile>> PROFILES = new EnumMap<>(SfType.class);

  static {
    PROFILES.put(
        SfType.DATE,
        Profile.booleanCartesian(
            Arrays.asList("JDBC_FORMAT_DATE_WITH_TIMEZONE", "JDBC_USE_SESSION_TIMEZONE")));
    PROFILES.put(SfType.TIME, Profile.booleanCartesian(Arrays.asList("JDBC_USE_SESSION_TIMEZONE")));
    PROFILES.put(
        SfType.TIMESTAMP_NTZ,
        Profile.booleanCartesian(
            Arrays.asList(
                "JDBC_USE_SESSION_TIMEZONE",
                "JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC",
                // CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ is intentionally omitted: it can't be
                // toggled against the test backend, so only its default (true) is exercised. See
                // TIMESTAMP_MIGRATION_PLAN.md (Phase 1).
                "JDBC_FORMAT_DATE_WITH_TIMEZONE")));
    PROFILES.put(
        SfType.TIMESTAMP_LTZ,
        Profile.booleanCartesian(
            Arrays.asList("JDBC_USE_SESSION_TIMEZONE", "JDBC_FORMAT_DATE_WITH_TIMEZONE")));
    PROFILES.put(
        SfType.TIMESTAMP_TZ,
        Profile.booleanCartesian(
            Arrays.asList("JDBC_USE_SESSION_TIMEZONE", "JDBC_FORMAT_DATE_WITH_TIMEZONE")));
  }

  static Stream<Arguments> dateReadCells() {
    return readCells(SfType.DATE);
  }

  static Stream<Arguments> timeReadCells() {
    return readCells(SfType.TIME);
  }

  static Stream<Arguments> timestampNtzReadCells() {
    return readCells(SfType.TIMESTAMP_NTZ);
  }

  static Stream<Arguments> timestampLtzReadCells() {
    return readCells(SfType.TIMESTAMP_LTZ);
  }

  static Stream<Arguments> timestampTzReadCells() {
    return readCells(SfType.TIMESTAMP_TZ);
  }

  /**
   * Read-cell generator for a single type: (tz, fmt, profile) over that type's axes. The {@code
   * parity.types} filter is applied per-invocation via {@link #typeEnabled} in the test body (not
   * here) so a filtered-out type yields skipped invocations rather than an empty argument source.
   */
  private static Stream<Arguments> readCells(SfType type) {
    Stream.Builder<Arguments> out = Stream.builder();
    for (String tz : TIMEZONES.get(type)) {
      for (String fmt : FORMATS.get(type)) {
        for (Profile profile : PROFILES.get(type)) {
          out.add(Arguments.of(tz, fmt, profile));
        }
      }
    }
    return out.build();
  }

  /**
   * Optional type filter, controlled by the {@code parity.types} system property (comma-separated
   * {@link SfType} names, case-insensitive). When unset or empty, all types run. Lets a run target
   * a single type, e.g. {@code -Dparity.types=DATE}.
   */
  private static boolean typeEnabled(SfType type) {
    String only = System.getProperty("parity.types");
    if (only == null || only.trim().isEmpty()) {
      return true;
    }
    for (String name : only.split(",")) {
      if (name.trim().equalsIgnoreCase(type.name())) {
        return true;
      }
    }
    return false;
  }

  /**
   * Write parity uses only the default profile; the boolean session flags affect getter behaviour,
   * not bind-side serialization. The one bind-relevant flag (CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME)
   * is connection-time only and needs a separate harness pair, tracked as a follow-up.
   *
   * <p>DATE write cells whose output format contains the {@code MON} element are skipped: the old
   * driver throws {@code IllegalArgumentException("Illegal pattern character 'O'")} at {@code
   * prepareStatement} time for a DATE result column under such a format, so every cell would diff
   * only because the old driver crashes before binding. This is an accepted behavior difference
   * (see {@code BehaviorDifferences.yaml} #7 — the universal driver correctly tolerates the valid
   * MON format). The read matrix still exercises those formats.
   */
  static Stream<Arguments> dateWriteCells() {
    return writeCells(SfType.DATE);
  }

  static Stream<Arguments> timeWriteCells() {
    return writeCells(SfType.TIME);
  }

  static Stream<Arguments> timestampNtzWriteCells() {
    return writeCells(SfType.TIMESTAMP_NTZ);
  }

  static Stream<Arguments> timestampLtzWriteCells() {
    return writeCells(SfType.TIMESTAMP_LTZ);
  }

  static Stream<Arguments> timestampTzWriteCells() {
    return writeCells(SfType.TIMESTAMP_TZ);
  }

  /**
   * Write-cell generator for a single type: (tz, fmt) over that type's axes. The {@code
   * parity.types} filter is applied per-invocation via {@link #typeEnabled} in the test body (not
   * here) so a filtered-out type yields skipped invocations rather than an empty argument source.
   */
  private static Stream<Arguments> writeCells(SfType type) {
    Stream.Builder<Arguments> out = Stream.builder();
    for (String tz : TIMEZONES.get(type)) {
      for (String fmt : FORMATS.get(type)) {
        if (type == SfType.DATE && fmt.contains("MON")) {
          continue;
        }
        out.add(Arguments.of(tz, fmt));
      }
    }
    return out.build();
  }

  /** Representative output format for a type: the first entry of its FORMATS list. */
  private static String representativeFormat(SfType type) {
    return FORMATS.get(type).get(0);
  }

  /**
   * Focused matrix for {@code JDBC_GET_DATE_USE_NULL_TIMEZONE}: DATE plus every TIMESTAMP flavour
   * (the param drives the date-extracting getters), both flag values, full timezone axis.
   */
  static Stream<Arguments> getDateNullTzCells() {
    return connectTimeCells(
        Arrays.asList(
            SfType.DATE, SfType.TIMESTAMP_NTZ, SfType.TIMESTAMP_LTZ, SfType.TIMESTAMP_TZ));
  }

  /**
   * Focused matrix for {@code JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE}: DATE and the
   * timezone-formatted timestamps, both flag values, full timezone axis.
   */
  static Stream<Arguments> defaultFormatDateWithTzCells() {
    return connectTimeCells(Arrays.asList(SfType.DATE, SfType.TIMESTAMP_LTZ, SfType.TIMESTAMP_TZ));
  }

  /** Focused matrix for {@code CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME}: TIME only (bind path). */
  static Stream<Arguments> treatTimeWallClockCells() {
    return connectTimeCells(Collections.singletonList(SfType.TIME));
  }

  /**
   * Shared (type, flag, tz) generator for the connection-time focused tests: each given type, both
   * boolean flag values, every timezone. Skips types in {@link #DISABLED_TYPES} (so the timestamp
   * flavours run nowhere while disabled) and honours the {@code parity.types} filter.
   */
  private static Stream<Arguments> connectTimeCells(List<SfType> types) {
    Stream.Builder<Arguments> out = Stream.builder();
    for (SfType type : types) {
      if (DISABLED_TYPES.contains(type) || !typeEnabled(type)) {
        continue;
      }
      for (boolean flag : new boolean[] {false, true}) {
        for (String tz : TIMEZONES.get(type)) {
          out.add(Arguments.of(type, flag, tz));
        }
      }
    }
    return out.build();
  }
}
