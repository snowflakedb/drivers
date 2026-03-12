package extensions;

import com.github.tomakehurst.wiremock.extension.Extension;
import com.github.tomakehurst.wiremock.extension.ServeEventListener;
import com.github.tomakehurst.wiremock.extension.Parameters;
import com.github.tomakehurst.wiremock.stubbing.ServeEvent;

import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.List;
import java.util.ArrayList;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

/**
 * Lightweight WireMock extension that tracks only response times.
 * All calculations (avg, percentiles, etc.) are done in Python.
 *
 * No file I/O happens per-request for maximum replay performance.
 * Python triggers a single flush by sending a request to the
 * {@value #FLUSH_PATH} stub after the test run finishes;
 * {@link #afterComplete} detects it and calls {@link #writeStatsToFile()}.
 */
public class ResponseTimeExtension implements ServeEventListener, Extension {
    private static final ConcurrentLinkedQueue<Long> responseTimes = new ConcurrentLinkedQueue<>();
    /** Maximum acceptable response time (5 minutes). Responses exceeding this are discarded as outliers. */
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
    }
    
    @Override
    public void afterMatch(ServeEvent serveEvent, Parameters parameters) {
    }
    
    @Override
    public void beforeResponseSent(ServeEvent serveEvent, Parameters parameters) {
    }
    
    @Override
    public void afterComplete(ServeEvent serveEvent, Parameters parameters) {
        String url = serveEvent.getRequest().getUrl();
        if (url != null && url.startsWith(FLUSH_PATH)) {
            writeStatsToFile();
            return;
        }

        long responseTime;
        try {
            long requestTime = serveEvent.getRequest().getLoggedDate().getTime();
            responseTime = System.currentTimeMillis() - requestTime;
        } catch (Exception e) {
            return;
        }

        if (responseTime < 0 || responseTime > MAX_RESPONSE_TIME_MS) {
            return;
        }

        responseTimes.add(Math.max(responseTime, 1));
    }
    
    /**
     * Write statistics to file atomically (temp file + rename) to avoid partial reads.
     * Triggered by a request to {@value #FLUSH_PATH} or from {@link #reset()}.
     */
    static synchronized void writeStatsToFile() {
        File target = new File(STATS_FILE);
        File tmp = new File(STATS_FILE + ".tmp");
        try (FileWriter writer = new FileWriter(tmp)) {
            List<Long> times = new ArrayList<>(responseTimes);
            writer.write("{\n");
            writer.write("  \"total_requests\": " + times.size() + ",\n");
            writer.write("  \"response_times\": [");

            for (int i = 0; i < times.size(); i++) {
                if (i > 0) writer.write(", ");
                writer.write(times.get(i).toString());
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
     * Reset all statistics
     */
    public static void reset() {
        responseTimes.clear();
        writeStatsToFile();
    }
}
