package net.snowflake.client.internal.loader;

import static org.junit.jupiter.api.Assertions.assertSame;

import java.io.IOException;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.exception.CoreException;
import org.junit.jupiter.api.Test;

class UtilsTest {

  @Test
  void shouldReturnSameThrowableWhenThereIsNoCause() {
    RuntimeException e = new RuntimeException("top");

    assertSame(e, Utils.getCause(e));
  }

  @Test
  void shouldReturnDeepestCauseOfForeignChain() {
    IOException root = new IOException("root");
    RuntimeException e = new RuntimeException("top", new IllegalStateException("middle", root));

    assertSame(root, Utils.getCause(e));
  }

  @Test
  void shouldStopAtSqlExceptionWhoseCauseIsInternalCarrier() {
    CoreException carrier = new CoreException("SQL compilation error", new IOException("io"));
    SnowflakeSQLException sqlException =
        new SnowflakeSQLException("SQL compilation error", carrier);
    RuntimeException e = new RuntimeException("wrapper", sqlException);

    assertSame(sqlException, Utils.getCause(e));
  }

  @Test
  void shouldReturnSqlExceptionWhenItIsTheTopLevelThrowable() {
    SnowflakeSQLException sqlException =
        new SnowflakeSQLException("SQL compilation error", new CoreException("carrier"));

    assertSame(sqlException, Utils.getCause(sqlException));
  }
}
