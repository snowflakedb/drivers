package net.snowflake.jdbc.e2e.parity;

import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.Statement;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.EnumMap;
import java.util.List;
import java.util.Map;
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
 * universal-driver and snowflake-jdbc 4.0.1 in the same JVM via isolated classloaders, and asserts
 * byte-identical output cell-by-cell.
 *
 * <p>Two test methods per cell:
 *
 * <ul>
 *   <li>{@code readParity} - one multi-column SELECT bakes every (scale, value) column. For each
 *       column we exercise every {@link GetSink} on both driver result sets and assert the outcomes
 *       match. The {@link Profile} dimension is the full Cartesian product of the boolean session
 *       parameters that affect that type's getters.
 *   <li>{@code writeParity} - one bind-only SELECT with every (scale, value, setSink) combination
 *       as a parameter. Each parameter is bound on both drivers; the round-tripped string is
 *       compared. Profiles are NOT varied here today: the bind-relevant flags are connection-time
 *       only (see below).
 * </ul>
 *
 * <p>Per-cell mismatches are accumulated and reported together at the end of each test method, so a
 * single run surfaces every divergence rather than aborting on the first.
 *
 * <p>Run via {@code ./gradlew :jdbc:parityTest}. Requires {@code PARAMETER_PATH} pointing at a
 * Snowflake test-account credentials JSON. Both connections are forced onto Arrow result format.
 *
 * <h2>Session params NOT covered by this matrix</h2>
 *
 * The following parameters affect date/time/timestamp handling but cannot be exercised by an {@code
 * ALTER SESSION SET} from a running connection — they are read once at connect time and never
 * refreshed from server responses. Varying them requires opening a fresh pair of driver
 * connections, which the current {@link ParityHarness} does not support. Track as a follow-up.
 *
 * <ul>
 *   <li>{@code CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME} (Boolean, default {@code false}) — when true,
 *       {@code setTime(...)} reads {@code Time.toLocalTime().toNanoOfDay()} so the JVM-local wall
 *       clock fields are bound, instead of treating {@code Time.getTime()} as UTC ms reduced modulo
 *       a day. Affects only the bind path for TIME.
 *   <li>{@code JDBC_DEFAULT_FORMAT_DATE_WITH_TIMEZONE} (Boolean, default {@code false}) —
 *       connection-time fallback default for {@code JDBC_FORMAT_DATE_WITH_TIMEZONE}, used by legacy
 *       driver's {@code DateConverter.getUseDateFormat(...)} when the server has not pushed the
 *       runtime value. Interacts with {@code JDBC_FORMAT_DATE_WITH_TIMEZONE} (which IS varied above
 *       by the DATE profile).
 *   <li>{@code JDBC_GET_DATE_USE_NULL_TIMEZONE} (Boolean, default {@code true}) — connection-time
 *       toggle that controls how {@code getDate(int, Calendar)} interprets a null/JVM-default
 *       Calendar. Server-side {@code ALTER SESSION SET} of this name is silently ignored by the
 *       legacy driver (not in {@code SessionUtil.BOOLEAN_PARAMS}); only the value passed in the
 *       JDBC {@link java.util.Properties} bag at connect time takes effect.
 * </ul>
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
@Execution(ExecutionMode.SAME_THREAD)
public class DateTimeParityTest {

  private static final List<GetSink> GET_SINKS = Arrays.asList(GetSink.values());
  private static final List<SetSink> SET_SINKS = Arrays.asList(SetSink.values());

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

  @ParameterizedTest(name = "READ {0} tz={1} fmt={2} profile={3}")
  @MethodSource("readCells")
  void readParity(SfType type, String tz, String fmt, Profile profile) throws Exception {
    harness.applyBoth(tz, type.outputFormatParam(), fmt, profile.overlay());
    ReadLayout layout = ReadLayout.build(type, SCALES.get(type), VALUES.get(type));

    List<String> failures = new ArrayList<>();
    Connection newConn = harness.newSession().connection();
    Connection oldConn = harness.oldSession().connection();
    try (Statement sNew = newConn.createStatement();
        Statement sOld = oldConn.createStatement();
        ResultSet rsNew = sNew.executeQuery(layout.sql);
        ResultSet rsOld = sOld.executeQuery(layout.sql)) {
      assertTrue(rsNew.next(), "no row from new driver");
      assertTrue(rsOld.next(), "no row from old driver");

      for (ReadLayout.Cell cell : layout.cells) {
        for (GetSink sink : GET_SINKS) {
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
                    + " profile="
                    + profile
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
    if (!failures.isEmpty()) {
      fail(buildReport(failures));
    }
  }

  @ParameterizedTest(name = "WRITE {0} tz={1} fmt={2}")
  @MethodSource("writeCells")
  void writeParity(SfType type, String tz, String fmt) throws Exception {
    harness.applyBoth(tz, type.outputFormatParam(), fmt, Profile.DEFAULT.overlay());

    // Issue one query per setSink instead of cramming every (value x scale x sink) bind into one
    // request. Universal-driver's JNI bridge has tripped a stack overflow on the Rust handler
    // when the protobuf payload carries hundreds of binds; per-sink chunking keeps requests
    // small and gives clearer per-sink failure attribution.
    List<String> failures = new ArrayList<>();
    for (SetSink sink : SET_SINKS) {
      runWriteChunk(
          type,
          tz,
          fmt,
          WriteLayout.build(
              type, SCALES.get(type), VALUES.get(type), Collections.singletonList(sink)),
          failures);
    }
    if (!failures.isEmpty()) {
      fail(buildReport(failures));
    }
  }

  private void runWriteChunk(
      SfType type, String tz, String fmt, WriteLayout layout, List<String> failures)
      throws Exception {
    Connection newConn = harness.newSession().connection();
    Connection oldConn = harness.oldSession().connection();

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
            "2024-01-15", "1970-01-01", "1999-12-31", "2000-02-29", "0001-01-01", "9999-12-31"));

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
            "00:00:00", "12:34:56.789012345", "23:59:59.999999999", "12:00:00.000000001"));

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
            "1582-10-15 00:00:00"));

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
            "2024-03-10 02:30:00",
            "2024-11-03 01:30:00",
            "2024-01-15 12:34:56.789",
            "1970-01-01 00:00:00"));

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
            "1970-01-01 00:00:00 +00:00",
            "2024-03-10 02:30:00 -05:00"));
  }

  // ---- Profiles ----
  // Per-type Cartesian over the boolean session params that affect that type's getters. The
  // matrix iterates the full 2^N product so any drift in flag handling surfaces; non-boolean
  // params (output formats, TIMEZONE) are kept as separate axes above.
  private static final Map<SfType, List<Profile>> PROFILES = new EnumMap<>(SfType.class);

  static {
    PROFILES.put(
        SfType.DATE, Profile.booleanCartesian(Arrays.asList("JDBC_FORMAT_DATE_WITH_TIMEZONE")));
    PROFILES.put(SfType.TIME, Profile.booleanCartesian(Arrays.asList("JDBC_USE_SESSION_TIMEZONE")));
    PROFILES.put(
        SfType.TIMESTAMP_NTZ,
        Profile.booleanCartesian(
            Arrays.asList(
                "JDBC_USE_SESSION_TIMEZONE",
                "JDBC_TREAT_TIMESTAMP_NTZ_AS_UTC",
                "CLIENT_HONOR_CLIENT_TZ_FOR_TIMESTAMP_NTZ",
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

  static Stream<Arguments> readCells() {
    Stream.Builder<Arguments> out = Stream.builder();
    for (SfType type : SfType.values()) {
      for (String tz : TIMEZONES.get(type)) {
        for (String fmt : FORMATS.get(type)) {
          for (Profile profile : PROFILES.get(type)) {
            out.add(Arguments.of(type, tz, fmt, profile));
          }
        }
      }
    }
    return out.build();
  }

  /**
   * Write parity uses only the default profile; the boolean session flags affect getter behaviour,
   * not bind-side serialization. The one bind-relevant flag (CLIENT_TREAT_TIME_AS_WALL_CLOCK_TIME)
   * is connection-time only and needs a separate harness pair, tracked as a follow-up.
   */
  static Stream<Arguments> writeCells() {
    Stream.Builder<Arguments> out = Stream.builder();
    for (SfType type : SfType.values()) {
      for (String tz : TIMEZONES.get(type)) {
        for (String fmt : FORMATS.get(type)) {
          out.add(Arguments.of(type, tz, fmt));
        }
      }
    }
    return out.build();
  }
}
