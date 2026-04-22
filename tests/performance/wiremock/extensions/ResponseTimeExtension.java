package extensions;

import com.github.tomakehurst.wiremock.extension.Extension;
import com.github.tomakehurst.wiremock.extension.ServeEventListener;
import com.github.tomakehurst.wiremock.extension.Parameters;
import com.github.tomakehurst.wiremock.stubbing.ServeEvent;

import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.ConcurrentHashMap;
import java.util.List;
import java.util.ArrayList;
import java.util.UUID;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

/**
 * WireMock extension that tracks response times split into two phases:
 * <ul>
 *   <li><b>Serve time</b> — from request received to response ready
 *       (stub matching, body lookup). Measured as request arrival → {@code beforeResponseSent}.</li>
 *   <li><b>Send time</b> — from response ready to fully written to the client socket
 *       (includes TLS encryption and TCP back-pressure from a slow receiver).
 *       Measured as {@code beforeResponseSent} → {@code afterComplete}.</li>
 * </ul>
 *
 * No file I/O happens per-request for maximum replay performance.
 * Python triggers a single flush by sending a request to {@value #FLUSH_PATH};
 * {@link #afterComplete} detects it and calls {@link #writeStatsToFile()}.
 */
public class ResponseTimeExtension implements ServeEventListener, Extension {

    private static final ConcurrentLinkedQueue<long[]> responseTimes = new ConcurrentLinkedQueue<>();

    /**
     * Per-request nanoTime captured in {@code beforeMatch} — the earliest hook.
     * Used as the monotonic start-of-request reference so all phase durations
     * are derived from the same clock source.
     */
    private static final ConcurrentHashMap<UUID, Long> requestStartTimestamps = new ConcurrentHashMap<>();

    /**
     * Per-request nanoTime captured in {@code beforeResponseSent}.
     * Entries are consumed in {@code afterComplete}.
     */
    private static final ConcurrentHashMap<UUID, Long> beforeSendTimestamps = new ConcurrentHashMap<>();

    private static final long MAX_RESPONSE_TIME_MS = 300_000;

    static final String FLUSH_PATH = "/__perf/flush-stats";

    private static final String STATS_FILE;
    static {
        String suffix = System.getProperty("response.time.stats.suffix", "");
        STATS_FILE = suffix.isEmpty()
            ? "/wiremock/response-time-stats.json"
            : "/wiremock/response-time-stats-" + suffix + ".json";
    }

    @Override
    public String getName() {
        return "response-time-tracker";
    }

    @Override
    public void beforeMatch(ServeEvent serveEvent, Parameters parameters) {
        String url = serveEvent.getRequest().getUrl();
        if (url != null && url.startsWith(FLUSH_PATH)) {
            return;
        }
        requestStartTimestamps.put(serveEvent.getId(), System.nanoTime());
    }

    @Override
    public void afterMatch(ServeEvent serveEvent, Parameters parameters) {
    }

    @Override
    public void beforeResponseSent(ServeEvent serveEvent, Parameters parameters) {
        String url = serveEvent.getRequest().getUrl();
        if (url != null && url.startsWith(FLUSH_PATH)) {
            return;
        }
        beforeSendTimestamps.put(serveEvent.getId(), System.nanoTime());
    }

    @Override
    public void afterComplete(ServeEvent serveEvent, Parameters parameters) {
        long afterCompleteNanos = System.nanoTime();

        String url = serveEvent.getRequest().getUrl();
        if (url != null && url.startsWith(FLUSH_PATH)) {
            writeStatsToFile();
            return;
        }

        UUID eventId = serveEvent.getId();
        Long startNanos = requestStartTimestamps.remove(eventId);
        Long beforeSendNanos = beforeSendTimestamps.remove(eventId);

        try {
            if (startNanos == null) {
                return;
            }

            long totalMs = (afterCompleteNanos - startNanos) / 1_000_000;
            if (totalMs < 0 || totalMs > MAX_RESPONSE_TIME_MS) {
                return;
            }

            if (beforeSendNanos == null) {
                responseTimes.add(new long[]{Math.max(totalMs, 1), 0});
                return;
            }

            long sendMs = Math.max((afterCompleteNanos - beforeSendNanos) / 1_000_000, 0);
            long serveMs = Math.max(totalMs - sendMs, 0);

            responseTimes.add(new long[]{serveMs, sendMs});
        } catch (Exception e) {
            return;
        }
    }

    /**
     * Write statistics to file atomically (temp file + rename) to avoid partial reads.
     * Triggered by a request to {@value #FLUSH_PATH}.
     */
    static synchronized void writeStatsToFile() {
        File target = new File(STATS_FILE);
        File tmp = new File(STATS_FILE + ".tmp");
        try (FileWriter writer = new FileWriter(tmp)) {
            List<long[]> entries = new ArrayList<>(responseTimes);

            writer.write("{\n");
            writer.write("  \"total_requests\": " + entries.size() + ",\n");

            writer.write("  \"serve_times\": [");
            for (int i = 0; i < entries.size(); i++) {
                if (i > 0) writer.write(", ");
                writer.write(Long.toString(entries.get(i)[0]));
            }
            writer.write("],\n");

            writer.write("  \"send_times\": [");
            for (int i = 0; i < entries.size(); i++) {
                if (i > 0) writer.write(", ");
                writer.write(Long.toString(entries.get(i)[1]));
            }
            writer.write("]\n}\n");

            writer.flush();
        } catch (IOException e) {
            System.err.println("ResponseTimeExtension: Error writing stats file to " + STATS_FILE + ": " + e.getMessage());
            return;
        }
        try {
            Files.move(tmp.toPath(), target.toPath(), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (IOException e) {
            System.err.println("ResponseTimeExtension: Error renaming tmp stats file: " + e.getMessage());
        }
    }

    /**
     * Reset all statistics.
     */
    public static void reset() {
        responseTimes.clear();
        requestStartTimestamps.clear();
        beforeSendTimestamps.clear();
        writeStatsToFile();
    }
}
