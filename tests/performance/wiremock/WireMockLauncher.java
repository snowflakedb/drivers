import wiremock.com.fasterxml.jackson.core.StreamReadConstraints;

/**
 * Launcher for WireMock that increases StreamReadConstraints before starting.
 * 
 * This is necessary because:
 * 1. WireMock 3.x uses shaded Jackson and ignores system properties
 * 2. We need to load large mapping files from disk (50M+ rows with base64 response bodies)
 */
public class WireMockLauncher {
    public static void main(String[] args) throws Exception {
        // handle large datasets
        int maxStringLength = 500_000_000;
        StreamReadConstraints.overrideDefaultStreamReadConstraints(
            StreamReadConstraints.builder()
                .maxStringLength(maxStringLength)
                .build()
        );
        wiremock.Run.main(args);
    }
}
