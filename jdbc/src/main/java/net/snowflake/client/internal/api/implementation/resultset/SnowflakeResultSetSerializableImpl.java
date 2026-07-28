package net.snowflake.client.internal.api.implementation.resultset;

import com.google.protobuf.MessageLite;
import com.google.protobuf.Parser;
import java.io.IOException;
import java.io.ObjectInputStream;
import java.io.ObjectOutputStream;
import java.io.Serializable;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import lombok.AccessLevel;
import lombok.Getter;
import net.snowflake.client.api.resultset.SnowflakeResultSetSerializable;
import net.snowflake.client.internal.api.implementation.parameters.FrozenParametersRegistry;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.ProtobufApis;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.DatabaseFetchChunkResponse;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ResultChunk;

class SnowflakeResultSetSerializableImpl implements SnowflakeResultSetSerializable, Serializable {

  private static final long serialVersionUID = 1L;

  private transient CoreDriverApi coreDriverApi;

  private final String queryId;

  // Snapshot of the originating session's parameters, used to rebuild the DataConversionContext so
  // the sessionless result set formats values exactly as the live session did.
  @Getter(AccessLevel.PACKAGE)
  private final FrozenParametersRegistry parameters;

  // Protobuf types aren't Serializable, so these are written/read manually as bytes.
  private transient List<ResultChunk> chunks;
  private transient List<ColumnMetadata> columnMetadata;

  SnowflakeResultSetSerializableImpl(
      CoreDriverApi coreDriverApi,
      String queryId,
      List<ResultChunk> chunks,
      List<ColumnMetadata> columnMetadata,
      FrozenParametersRegistry parameters) {
    this.coreDriverApi = coreDriverApi;
    this.queryId = queryId;
    this.chunks = Collections.unmodifiableList(new ArrayList<>(chunks));
    this.columnMetadata = Collections.unmodifiableList(new ArrayList<>(columnMetadata));
    this.parameters = parameters;
  }

  static List<SnowflakeResultSetSerializable> splitBySize(
      CoreDriverApi coreDriverApi,
      List<ResultChunk> allChunks,
      List<ColumnMetadata> columnMetadata,
      String queryId,
      ParametersRegistry parameters,
      long maxSizeInBytes)
      throws SQLException {
    if (allChunks.isEmpty()) {
      throw new SQLException("The Result Set serializable is invalid.");
    }

    ResultChunk inlineChunk = null;
    List<ResultChunk> remoteChunks = new ArrayList<>();
    for (ResultChunk chunk : allChunks) {
      if (chunk.hasInline()) {
        // there can be only one inline chunk
        inlineChunk = chunk;
      } else {
        remoteChunks.add(chunk);
      }
    }

    if (inlineChunk == null && remoteChunks.stream().noneMatch(ResultChunk::hasRemote)) {
      throw new SQLException("The Result Set serializable is invalid.");
    }

    FrozenParametersRegistry frozenParameters = parameters.freeze();
    List<SnowflakeResultSetSerializable> resultSetSerializables = new ArrayList<>();
    List<ResultChunk> currentChunks = new ArrayList<>();
    if (inlineChunk != null) {
      currentChunks.add(inlineChunk);
    }
    for (ResultChunk remoteChunk : remoteChunks) {
      long currentSize = uncompressedDataSizeInBytes(currentChunks);
      long remoteSize = uncompressedDataSizeInBytes(remoteChunk);
      if (currentSize > 0 && maxSizeInBytes < currentSize + remoteSize) {
        resultSetSerializables.add(
            new SnowflakeResultSetSerializableImpl(
                coreDriverApi, queryId, currentChunks, columnMetadata, frozenParameters));
        currentChunks = new ArrayList<>();
      }
      currentChunks.add(remoteChunk);
    }
    resultSetSerializables.add(
        new SnowflakeResultSetSerializableImpl(
            coreDriverApi, queryId, currentChunks, columnMetadata, frozenParameters));

    return resultSetSerializables;
  }

  @Override
  public ResultSet getResultSet(ResultSetRetrieveConfig resultSetRetrieveConfig)
      throws SQLException {
    if (chunks.isEmpty()) {
      throw new SQLException("The Result Set serializable is invalid.");
    }
    // TODO: use url and proxy setting from ResultSetRetrieveConfig

    DatabaseFetchChunkResponse response = coreDriverApi.databaseFetchChunk(chunks, columnMetadata);
    // The factory rebuilds the originating session's conversion context from this frozen parameter
    // snapshot so formatting matches the live result set. Pass the chunks too, so the derived
    // (sessionless) ResultSet can be serialized again without re-fetch.
    return ResultSetFactory.createFromChunks(
        coreDriverApi, chunks, columnMetadata, queryId, response, getRowCount(), parameters);
  }

  @Override
  public long getRowCount() throws SQLException {
    return chunks.stream().mapToInt(ResultChunk::getRowCount).sum();
  }

  @Override
  public long getCompressedDataSizeInBytes() {
    return chunks.stream()
        .mapToLong(SnowflakeResultSetSerializableImpl::compressedDataSizeInBytes)
        .sum();
  }

  @Override
  public long getUncompressedDataSizeInBytes() {
    return chunks.stream()
        .mapToLong(SnowflakeResultSetSerializableImpl::uncompressedDataSizeInBytes)
        .sum();
  }

  private static long compressedDataSizeInBytes(ResultChunk chunk) {
    if (chunk.hasInline()) {
      return chunk.getInline().length();
    }
    if (chunk.hasRemote()) {
      return chunk.getRemote().getCompressedSize();
    }
    return 0;
  }

  private static long uncompressedDataSizeInBytes(ResultChunk chunk) {
    if (chunk.hasInline()) {
      return chunk.getInline().length();
    }
    if (chunk.hasRemote()) {
      return chunk.getRemote().getUncompressedSize();
    }
    return 0;
  }

  private static long uncompressedDataSizeInBytes(List<ResultChunk> chunkList) {
    return chunkList.stream()
        .mapToLong(SnowflakeResultSetSerializableImpl::uncompressedDataSizeInBytes)
        .sum();
  }

  private void writeObject(ObjectOutputStream out) throws IOException {
    out.defaultWriteObject();
    serializeProtoObjects(out, chunks);
    serializeProtoObjects(out, columnMetadata);
  }

  private void readObject(ObjectInputStream in) throws IOException, ClassNotFoundException {
    in.defaultReadObject();
    chunks = deserializeProtoObjects(in, ResultChunk.parser());
    columnMetadata = deserializeProtoObjects(in, ColumnMetadata.parser());

    coreDriverApi = ProtobufApis.coreDriverApi;
  }

  private static <T extends MessageLite> void serializeProtoObjects(
      ObjectOutputStream out, List<T> objects) throws IOException {
    out.writeInt(objects.size());
    for (T object : objects) {
      byte[] bytes = object.toByteArray();
      out.writeInt(bytes.length);
      out.write(bytes);
    }
  }

  private static <T> List<T> deserializeProtoObjects(ObjectInputStream in, Parser<T> parser)
      throws IOException {
    int objectsCount = in.readInt();
    List<T> deserializedObjects = new ArrayList<>(objectsCount);
    for (int i = 0; i < objectsCount; i++) {
      byte[] bytes = new byte[in.readInt()];
      in.readFully(bytes);
      deserializedObjects.add(parser.parseFrom(bytes));
    }
    return Collections.unmodifiableList(deserializedObjects);
  }
}
