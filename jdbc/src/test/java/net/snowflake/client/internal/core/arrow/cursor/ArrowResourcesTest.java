package net.snowflake.client.internal.core.arrow.cursor;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.IOException;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.core.arrow.vector.Column;
import net.snowflake.client.internal.core.arrow.vector.ColumnBatch;
import net.snowflake.client.internal.core.arrow.vector.ColumnBuilder;
import org.junit.jupiter.api.Test;

public class ArrowResourcesTest {

  @Test
  public void shouldTranslateSchemaReadFailureToInternalError() {
    IOException schemaError = new IOException("schema read failed");
    ArrowResources resources = new ArrowResources(new FailingReader(schemaError, null));

    SFSQLException thrown = assertThrows(SFSQLException.class, resources::getActiveRoot);

    assertEquals(ErrorCode.INTERNAL_ERROR, thrown.getErrorCode());
    assertEquals(
        "INTERNAL_ERROR: [Unable to read Arrow schema, java.io.IOException: schema read failed]",
        thrown.getMessage());
  }

  @Test
  public void shouldPreferCurrentRootOverReaderBatch() {
    ArrowResources resources =
        new ArrowResources(new FailingReader(new IOException("must not be read"), null));
    try (Column col = ColumnBuilder.of("c", Column.PhysicalKind.INT32, null);
        ColumnBatch root = ColumnBatch.of(col)) {
      resources.setCurrentRoot(root);

      assertSame(root, resources.getActiveRoot());
    }
  }

  @Test
  public void shouldWrapReaderCloseFailureInCloseAll() {
    IOException closeError = new IOException("reader close failed");
    ArrowResources resources = new ArrowResources(new FailingReader(null, closeError));

    SFSQLException thrown = assertThrows(SFSQLException.class, resources::closeAll);

    assertSame(closeError, thrown.getCause());
  }

  @Test
  public void shouldDropRootAndReaderOnReset() {
    ArrowResources resources =
        new ArrowResources(new FailingReader(null, new IOException("must not be closed")));
    try (Column col = ColumnBuilder.of("c", Column.PhysicalKind.INT32, null);
        ColumnBatch root = ColumnBatch.of(col)) {
      resources.setCurrentRoot(root);

      resources.reset();

      assertNull(resources.getCurrentRoot());
      assertDoesNotThrow(resources::closeAll);
    }
  }

  private static final class FailingReader implements ArrowBatchReader {
    private final IOException batchError;
    private final IOException closeError;

    private FailingReader(IOException batchError, IOException closeError) {
      this.batchError = batchError;
      this.closeError = closeError;
    }

    @Override
    public boolean loadNextBatch() {
      return false;
    }

    @Override
    public ColumnBatch getBatch() throws IOException {
      if (batchError != null) {
        throw batchError;
      }
      return null;
    }

    @Override
    public void close() throws IOException {
      if (closeError != null) {
        throw closeError;
      }
    }
  }
}
