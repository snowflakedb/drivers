package extensions;

import com.github.tomakehurst.wiremock.extension.Extension;
import com.github.tomakehurst.wiremock.extension.ServeEventListener;
import com.github.tomakehurst.wiremock.extension.Parameters;
import com.github.tomakehurst.wiremock.stubbing.ServeEvent;

import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.List;
import java.util.ArrayList;
import java.util.Map;
import java.util.HashMap;
import java.io.FileWriter;
import java.io.IOException;

/**
 * Lightweight WireMock extension that tracks only response times.
 * All calculations (avg, percentiles, etc.) are done in Python.
 */
public class ResponseTimeExtension implements ServeEventListener, Extension {
    private static final ConcurrentLinkedQueue<Long> responseTimes = new ConcurrentLinkedQueue<>();
    private static final String STATS_FILE = "/wiremock/response-time-stats.json";
    /** Maximum acceptable response time (5 minutes). Responses exceeding this are discarded as outliers. */
    private static final long MAX_RESPONSE_TIME_MS = 300_000;
    
    public ResponseTimeExtension() {
        // Extension initialized
    }
    
    @Override
    public String getName() {
        return "response-time-tracker";
    }
    
    @Override
    public void beforeMatch(ServeEvent serveEvent, Parameters parameters) {
        // Not needed for response time tracking
    }
    
    @Override
    public void afterMatch(ServeEvent serveEvent, Parameters parameters) {
        // Not needed for response time tracking  
    }
    
    @Override
    public void beforeResponseSent(ServeEvent serveEvent, Parameters parameters) {
        // Not needed for response time tracking
    }
    
    @Override
    public void afterComplete(ServeEvent serveEvent, Parameters parameters) {
        long responseTime = 0;
        
        try {
            // Calculate actual response time from request timestamp
            long requestTime = serveEvent.getRequest().getLoggedDate().getTime();
            long currentTime = System.currentTimeMillis();
            responseTime = currentTime - requestTime;
        } catch (Exception e) {
            // Silently skip requests with timing errors
            return;
        }
        
        // Validate response time
        if (responseTime < 0) {
            return; // Skip negative times
        }
        
        if (responseTime == 0) {
            // This happens when request timestamp equals current time (same millisecond)
            // Use 1ms as minimum to avoid division by zero and indicate request was processed
            responseTime = 1;
        }
        
        if (responseTime > MAX_RESPONSE_TIME_MS) {
            return;
        }
        
        // Store response time
        responseTimes.add(responseTime);
        
        // Write stats to file periodically (every 10 requests for responsiveness)
        int count = responseTimes.size();
        if (count % 10 == 0 || count == 1) {
            writeStatsToFile();
        }
    }
    
    /**
     * Get all response times (calculations done in Python)
     */
    public static Map<String, Object> getStats() {
        Map<String, Object> stats = new HashMap<>();
        
        // Return all response times for Python to process
        List<Long> times = new ArrayList<>(responseTimes);
        stats.put("response_times", times);
        stats.put("total_requests", times.size());
        
        return stats;
    }
    
    /**
     * Write statistics to file for external access
     */
    private static void writeStatsToFile() {
        try (FileWriter writer = new FileWriter(STATS_FILE)) {
            Map<String, Object> stats = getStats();
            // Simple JSON writing (no external dependencies)
            writer.write("{\n");
            writer.write("  \"total_requests\": " + stats.get("total_requests") + ",\n");
            writer.write("  \"response_times\": [");
            
            @SuppressWarnings("unchecked")
            List<Long> times = (List<Long>) stats.get("response_times");
            for (int i = 0; i < times.size(); i++) {
                if (i > 0) writer.write(", ");
                writer.write(times.get(i).toString());
            }
            
            writer.write("]\n}\n");
            writer.flush();
        } catch (IOException e) {
            System.err.println("ResponseTimeExtension: Error writing stats file to " + STATS_FILE + ": " + e.getMessage());
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
