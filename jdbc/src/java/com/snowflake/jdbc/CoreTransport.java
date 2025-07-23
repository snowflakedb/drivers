package com.snowflake.jdbc;

import org.apache.thrift.TConfiguration;
import org.apache.thrift.transport.TTransport;
import org.apache.thrift.transport.TTransportException;

/**
 * CoreTransport provides a Thrift transport implementation that integrates with the sf_core library.
 * It uses native sf_core functions through JNI to handle read, write, and flush operations.
 * 
 * This transport is designed to work with the sf_core's ThriftTransport implementation,
 * providing a bridge between Java JDBC and the Rust-based core functionality.
 */
public class CoreTransport extends TTransport {
    
    // Handle to the native sf_core API instance
    private final long[] apiHandle;
    
    // Transport state
    private boolean open;
    
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
    
    /**
     * Creates a new CoreTransport and initializes the sf_core API.
     * This constructor automatically initializes a DatabaseDriverV1 API instance.
     */
    public CoreTransport(CoreApi.ApiType apiType) {
        this.apiHandle = nativeInit(apiType.id);
        this.open = false;
    }
    
    @Override
    public boolean isOpen() {
        return open;
    }
    
    @Override
    public void open() throws TTransportException {
        if (!isOpen()) {
            open = true;
        }
    }
    
    @Override
    public void close() {
        if (isOpen()) {
            open = false;
            // Clean up native resources
            nativeDestroy(apiHandle);
        }
    }
    
    @Override
    public int read(byte[] buf, int off, int len) throws TTransportException {
        if (!isOpen()) {
            throw new TTransportException(TTransportException.NOT_OPEN, "Transport is not open");
        }
        
        if (buf == null) {
            throw new TTransportException(TTransportException.UNKNOWN, "Buffer cannot be null");
        }
        
        if (off < 0 || len < 0 || off + len > buf.length) {
            throw new TTransportException(TTransportException.UNKNOWN, "Invalid buffer parameters");
        }
        
        if (len == 0) {
            return 0;
        }
        
        try {
            // Create a temporary buffer for the native call
            byte[] tempBuffer = new byte[len];
            int bytesRead = nativeRead(apiHandle, tempBuffer, len);
            
            // Copy the read data to the target buffer
            System.arraycopy(tempBuffer, 0, buf, off, bytesRead);
            
            return bytesRead;
        } catch (Exception e) {
            throw new TTransportException(TTransportException.UNKNOWN, 
                "Failed to read from transport: " + e.getMessage(), e);
        }
    }
    
    @Override
    public void write(byte[] buf, int off, int len) throws TTransportException {
        if (!isOpen()) {
            throw new TTransportException(TTransportException.NOT_OPEN, "Transport is not open");
        }
        
        if (buf == null) {
            throw new TTransportException(TTransportException.UNKNOWN, "Buffer cannot be null");
        }
        
        if (off < 0 || len < 0 || off + len > buf.length) {
            throw new TTransportException(TTransportException.UNKNOWN, "Invalid buffer parameters");
        }
        
        if (len == 0) {
            return;
        }
        
        try {
            // Create a buffer with just the data to write
            byte[] writeBuffer = new byte[len];
            System.arraycopy(buf, off, writeBuffer, 0, len);
            
            int bytesWritten = nativeWrite(apiHandle, writeBuffer, len);
            
            if (bytesWritten != len) {
                throw new TTransportException(TTransportException.UNKNOWN, 
                    String.format("Incomplete write: expected %d bytes, wrote %d bytes", len, bytesWritten));
            }
        } catch (Exception e) {
            throw new TTransportException(TTransportException.UNKNOWN, 
                "Failed to write to transport: " + e.getMessage(), e);
        }
    }
    
    @Override
    public void flush() throws TTransportException {
        if (!isOpen()) {
            throw new TTransportException(TTransportException.NOT_OPEN, "Transport is not open");
        }
        
        try {
            nativeFlush(apiHandle);
        } catch (Exception e) {
            throw new TTransportException(TTransportException.UNKNOWN, 
                "Failed to flush transport: " + e.getMessage(), e);
        }
    }

    @Override
    public TConfiguration getConfiguration() {
        return null;
    }

    @Override
    public void updateKnownMessageSize(long l) throws TTransportException {

    }

    @Override
    public void checkReadBytesAvailable(long l) throws TTransportException {

    }

    /**
     * Gets the native API handle for this transport.
     * 
     * @return The native sf_core API handle
     */
    public long[] getApiHandle() {
        return apiHandle;
    }
    
    // Native method declarations - these will be implemented in the JNI layer
    
    /**
     * Initialize the sf_core API.
     * 
     * @param apiType The API type (1 for DatabaseDriverApiV1)
     * @return Native handle to the API instance
     */
    private static native long[] nativeInit(int apiType);
    
    /**
     * Destroy the sf_core API instance.
     * 
     * @param handle The API handle to destroy
     */
    private static native void nativeDestroy(long[] handle);
    
    /**
     * Write data to the transport.
     * 
     * @param handle The API handle
     * @param buffer The data to write
     * @param length The number of bytes to write
     * @return The number of bytes written
     */
    private static native int nativeWrite(long[] handle, byte[] buffer, int length);
    
    /**
     * Read data from the transport.
     * 
     * @param handle The API handle
     * @param buffer The buffer to read into
     * @param length The maximum number of bytes to read
     * @return The number of bytes read
     */
    private static native int nativeRead(long[] handle, byte[] buffer, int length);
    
    /**
     * Flush the transport.
     * 
     * @param handle The API handle
     */
    private static native void nativeFlush(long[] handle);
}

