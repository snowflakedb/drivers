package com.snowflake.unicore;

public class JNICoreTransport implements CoreTransport {
    static {
        // Load the native library
        try {
            // Try to load from CORE_PATH environment variable
            String corePath = System.getenv("CORE_PATH");
            if (corePath == null) {
                throw new RuntimeException("CORE_PATH environment variable not set");
            }
            System.load(corePath);
        } catch (UnsatisfiedLinkError e) {
            // Fallback to explicit path if needed
            try {
                String libraryPath = System.getProperty("jdbc.library.path");
                if (libraryPath != null) {
                    System.load(libraryPath);
                } else {
                    throw new RuntimeException("jdbc native library not found. " +
                            "Please ensure the library is available or set the jdbc.library.path system property.", e);
                }
            } catch (UnsatisfiedLinkError e2) {
                throw new RuntimeException("Failed to load jdbc native library", e2);
            }
        }
    }

    @Override
    public TransportResponse handleMessage(String serviceName, String methodName, byte[] requestBytes) throws TransportException {
        TransportResponse response = nativeHandleMessage(serviceName, methodName, requestBytes);
        if (response == null) {
            throw new TransportException("Empty transport response");
        }
        return response;
    }

    private static native TransportResponse nativeHandleMessage(String serviceName, String methodName, byte[] requestBytes);
}
