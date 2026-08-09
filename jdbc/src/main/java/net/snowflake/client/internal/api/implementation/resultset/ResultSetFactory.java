package net.snowflake.client.internal.api.implementation.resultset;

import java.util.List;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.api.implementation.resultset.metadata.SnowflakeResultSetMetaDataImpl;
import net.snowflake.client.internal.api.implementation.statement.SnowflakeStatementImpl;
import net.snowflake.client.internal.core.arrow.ArrowStreamFactory;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.core.arrow.converters.SessionDataConversionContext;
import net.snowflake.client.internal.core.arrow.cursor.ArrowResources;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ArrowArrayStreamPtr;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseFetchChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetDescriptor;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetGetStreamResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultSetResponse;

/**
 * Central factory for all ResultSet creation.
 *
 * <p>For synchronous results, fetches the prebuilt Arrow stream via {@code resultSetGetStream} and
 * hands the core result set handle to the constructed {@link SnowflakeResultSetImpl}, which
 * releases it on close. Keeping the handle alive lets {@code getResultSetSerializables} slice its
 * chunk metadata without re-fetching from the backend. The handle is released here only when result
 * set construction does not take ownership (stream fetch fails, or there is no usable stream).
 *
 * <p>For asynchronous results, creates an {@link SnowflakeAsyncResultSetImpl} that lazily
 * materializes on first data access.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class ResultSetFactory {

  public static InternalResultSet create(
      CoreDriverApi coreDriverApi,
      SnowflakeStatementImpl statement,
      String queryId,
      ResultSetResponse rs) {
    CoreResultSetProvider coreResultSet =
        new CoreResultSetProvider(
            coreDriverApi, rs.getResultSetHandle(), queryId, parametersOf(statement));
    try {
      ResultSetGetStreamResponse response = coreResultSet.getStream();
      return resultSetFromResponse(
          coreResultSet, statement, queryId, response.getStream(), rs.getResultDescriptor());
    } catch (Throwable e) {
      coreResultSet.release();
      throw e;
    }
  }

  public static InternalAsyncResultSet createAsync(
      String queryId,
      InternalSnowflakeConnection connection,
      SnowflakeStatementImpl statement,
      boolean ownsStatement) {
    return new SnowflakeAsyncResultSetImpl(queryId, connection, statement, ownsStatement);
  }

  public static InternalResultSet createIfHasStream(
      CoreDriverApi coreDriverApi,
      SnowflakeStatementImpl statement,
      String queryId,
      ResultSetResponse rs) {
    CoreResultSetProvider coreResultSet =
        new CoreResultSetProvider(
            coreDriverApi, rs.getResultSetHandle(), queryId, parametersOf(statement));
    try {
      ResultSetGetStreamResponse response = coreResultSet.getStream();
      if (response.hasStream() && !response.getStream().getValue().isEmpty()) {
        return resultSetFromResponse(
            coreResultSet, statement, queryId, response.getStream(), rs.getResultDescriptor());
      }
      coreResultSet.release();
      return null;
    } catch (Throwable e) {
      coreResultSet.release();
      throw e;
    }
  }

  static InternalResultSet createFromChunks(
      CoreDriverApi coreDriverApi,
      List<ResultChunk> chunks,
      List<ColumnMetadata> columnMetadata,
      String queryId,
      DatabaseFetchChunkResponse chunk,
      long rowCount,
      ParametersRegistry parameters) {
    ResultSetChunksProvider chunkSource =
        new InMemoryResultSetChunksProvider(
            coreDriverApi, chunks, columnMetadata, queryId, parameters);
    return resultSetFromResponse(
        chunkSource,
        null,
        queryId,
        chunk.getStream(),
        columnMetadata,
        rowCount,
        SessionDataConversionContext.from(parameters));
  }

  /**
   * Wraps an existing result set with a {@link ConvertingRowReader} that projects/filters rows into
   * a new column layout. The source result set is detached (marked closed and unregistered from the
   * statement) and its {@link RowReader} ownership transfers to the new result set.
   */
  public static InternalResultSet wrapWithConverter(
      SnowflakeStatementImpl statement,
      SnowflakeResultSetImpl resultSet,
      SnowflakeResultSetMetaDataImpl metaData,
      RowConverter converter) {
    String queryID = metaData.getQueryID();
    RowReader sourceReader = resultSet.detachRowReader();
    String[] names = metaData.getColumnNames().toArray(new String[metaData.getColumnCount()]);
    ConvertingRowReader convertingReader = new ConvertingRowReader(sourceReader, names, converter);

    return new SnowflakeResultSetImpl(statement, queryID, convertingReader, metaData, true);
  }

  /** Creates a result set backed by pre-built in-memory rows. */
  public static InternalResultSet createFromRows(
      SnowflakeStatementImpl statement,
      SnowflakeResultSetMetaDataImpl metaData,
      Object[][] rows,
      boolean ownsStatement) {
    String queryId = metaData.getQueryID();
    String[] names = metaData.getColumnNames().toArray(new String[metaData.getColumnCount()]);
    InMemoryRowReader rowReader = new InMemoryRowReader(names, rows);
    return new SnowflakeResultSetImpl(statement, queryId, rowReader, metaData, ownsStatement);
  }

  /** Creates a result set with the given metadata and no rows. */
  public static InternalResultSet createEmpty(
      SnowflakeStatementImpl statement,
      SnowflakeResultSetMetaDataImpl metaData,
      boolean ownsStatement) {
    String queryId = metaData.getQueryID();
    String[] names = metaData.getColumnNames().toArray(new String[metaData.getColumnCount()]);
    InMemoryRowReader rowReader = new InMemoryRowReader(names, new Object[][] {});
    return new SnowflakeResultSetImpl(statement, queryId, rowReader, metaData, ownsStatement);
  }

  private static InternalResultSet resultSetFromResponse(
      ResultSetChunksProvider resultSetChunksProvider,
      SnowflakeStatementImpl statement,
      String queryId,
      ArrowArrayStreamPtr arrayStreamPtr,
      ResultSetDescriptor descriptor) {
    long totalRowCount = descriptor.hasRowCount() ? descriptor.getRowCount() : -1;
    return resultSetFromResponse(
        resultSetChunksProvider,
        statement,
        queryId,
        arrayStreamPtr,
        descriptor.getColumnsList(),
        totalRowCount,
        SessionDataConversionContext.from(parametersOf(statement)));
  }

  private static InternalResultSet resultSetFromResponse(
      ResultSetChunksProvider resultSetChunksProvider,
      SnowflakeStatementImpl statement,
      String queryId,
      ArrowArrayStreamPtr arrayStreamPtr,
      List<ColumnMetadata> columns,
      long totalRowCount,
      DataConversionContext conversionContext) {
    byte[] streamPointerBytes = arrayStreamPtr.getValue().toByteArray();
    long pointer = ArrowStreamFactory.pointerFromBytes(streamPointerBytes);
    ArrowResources arrowResources = ArrowStreamFactory.createFromPointer(pointer);
    SnowflakeResultSetMetaDataImpl metaData =
        SnowflakeResultSetMetaDataImpl.from(queryId, columns, conversionContext);
    ArrowRowReader rowReader = new ArrowRowReader(arrowResources, conversionContext, totalRowCount);
    return new SnowflakeResultSetImpl(
        statement, queryId, rowReader, metaData, false, resultSetChunksProvider);
  }

  /**
   * The connection's live parameter registry, or an empty registry for a statement-less caller. An
   * empty registry yields the same interface-default formatting a context-free caller would get.
   *
   * <p>Borrows the statement's connection without a closed-state check: ResultSet construction must
   * succeed even if the statement was concurrently closed after execute() returned (parity with
   * legacy snowflake-jdbc). getConnectionInternal() returns the connection directly rather than
   * owning it, so it must not be closed here.
   */
  private static ParametersRegistry parametersOf(SnowflakeStatementImpl statement) {
    return statement == null
        ? ParametersRegistry.EMPTY
        : statement.getConnectionInternal().getParameters();
  }
}
