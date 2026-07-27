package net.snowflake.client.internal.unicore;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.File;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

/** JNI round-trip probe; needs {@code jdbc_bridge} on {@code CORE_PATH}. */
public class CoreLoggingBridgeTest {

  @Test
  public void shouldReturnDeliveredWhenNativeBridgeIsLoaded() {
    String corePath = System.getenv("CORE_PATH");
    Assumptions.assumeTrue(
        corePath != null && new File(corePath).exists(),
        "jdbc_bridge native library not built; skipping JNI round-trip probe");

    try {
      int status =
          CoreLoggingBridge.logEvent(
              2,
              "jni round-trip probe",
              "CoreLoggingBridgeTest.java",
              1,
              "shouldReturnDeliveredWhenNativeBridgeIsLoaded",
              "net.snowflake.client.internal.unicore.CoreLoggingBridgeTest");
      assertEquals(CoreLoggingBridge.CORE_DELIVERED, status);
    } catch (UnsatisfiedLinkError e) {
      Assumptions.assumeTrue(
          false, "jdbc_bridge missing nativeLogEvent symbol; rebuild jdbc_bridge");
    }
  }
}
