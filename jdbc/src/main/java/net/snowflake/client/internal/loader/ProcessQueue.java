package net.snowflake.client.internal.loader;

import static java.lang.Math.toIntExact;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.LinkedList;
import java.util.List;
import net.snowflake.client.api.loader.LoadResultListener;
import net.snowflake.client.api.loader.Loader;
import net.snowflake.client.api.loader.LoadingError;
import net.snowflake.client.api.loader.Operation;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

/**
 * This class is responsible for processing a collection of uploaded data files represented by
 * BufferStage class
 */
public class ProcessQueue implements Runnable {
  private static final SFLogger logger = SFLoggerFactory.getLogger(ProcessQueue.class);

  private final Thread _thread;

  private final StreamLoader _loader;

  // Read and written only by the processing thread; they name the step a failure happened in.
  private State _currentState;
  private String _currentCommand;

  public ProcessQueue(StreamLoader loader) {
    logger.debug("Creating new ProcessQueue");

    _loader = loader;
    _thread = new Thread(this);
    _thread.setName("ProcessQueueThread");
    _thread.start();
  }

  @Override
  public void run() {
    while (true) {
      BufferStage stage = null;
      _currentState = State.INITIALIZE;
      _currentCommand = null;
      try {
        stage = _loader.takeProcess();
        if (stage.getRowCount() > 0) {
          processStage(stage);
        }
        if (stage.isTerminate()) {
          break;
        }
      } catch (InterruptedException ex) {
        logger.error("Interrupted", ex);
        break;
      } catch (Exception ex) {
        String msg =
            String.format("State: %s, %s, %s", _currentState, _currentCommand, ex.getMessage());
        _loader.abort(new Loader.ConnectionError(msg, Utils.getCause(ex)));
        logger.error("Processing failed in state {}: {}", _currentState, ex.getClass().getName());
        if (stage == null || stage.isTerminate()) {
          break;
        }
      }
    }
  }

  private void processStage(BufferStage stage) throws SQLException {
    Connection conn = _loader.getProcessConnection();
    // No double quote is added _loader.getRemoteStage(), since
    // it is mostly likely to be "~". If not, we may need to double quote
    // them.
    String remoteStage = "@" + _loader.getRemoteStage() + "/" + stage.getRemoteLocation();
    // Loader.abort() and finish() are also synchronized on this
    synchronized (_loader) {
      String updateKeys = getOn(_loader.getKeys(), "T", "S");
      if (stage.getOp() != Operation.INSERT && updateKeys.isEmpty()) {
        _loader.abort(new RuntimeException("No update key column is specified for the job."));
      }

      if (_loader.isAborted()) {
        // Everything was rolled back upon abort() call
        if (!_loader._preserveStageFile) {
          _currentCommand = "RM '" + remoteStage + "'";
          logger.debug("Removing uploaded files from the stage: {}", remoteStage);
          StreamLoader.execute(conn, _currentCommand);
        } else {
          logger.debug(
              "Error occurred. The remote stage is preserved for further investigation: {}",
              remoteStage);
        }
        return;
      }

      createStagingTable(conn, stage);
      if (!copyIntoStagingTable(conn, stage, remoteStage)) {
        return;
      }
      stage.setState(BufferStage.State.VALIDATED);

      int updateCount = ingest(conn, stage, updateKeys);
      stage.setState(BufferStage.State.PROCESSED);
      _currentState = State.FINISH;
      _currentCommand = null;
      reportCounts(stage, updateCount);

      StreamLoader.execute(conn, "RM '" + remoteStage + "'");
    }
  }

  /** Creates a temporary table like the target, reduced to the loaded columns. */
  private void createStagingTable(Connection conn, BufferStage stage) throws SQLException {
    logger.debug("Creating Temporary Table: name={}", stage.getId());
    _currentState = State.CREATE_TEMP_TABLE;
    List<String> allColumns = getAllColumns(conn);

    // use like to make sure columns in temporary table
    // contains properties (e.g., NOT NULL) from the source table
    _currentCommand =
        "CREATE TEMPORARY TABLE "
            + StreamLoader.quoteIdentifier(stage.getId())
            + " LIKE "
            + _loader.getFullTableName();
    List<String> selectedColumns = _loader.getColumns();
    StreamLoader.execute(conn, _currentCommand);

    // In case clustering key exists, drop it from the temporary table so that unused
    // columns can be dropped from the table without errors.
    StreamLoader.execute(
        conn,
        "alter table " + StreamLoader.quoteIdentifier(stage.getId()) + " drop clustering key");

    for (String col : allColumns) {
      if (!selectedColumns.contains(col)) {
        StreamLoader.execute(
            conn,
            "alter table "
                + StreamLoader.quoteIdentifier(stage.getId())
                + " drop column "
                + StreamLoader.quoteIdentifier(col));
      }
    }
  }

  /**
   * Copies the staged files into the staging table and reports rejected rows to the listener.
   *
   * @return false when the listener asked to stop on the first error and the loader was aborted
   */
  private boolean copyIntoStagingTable(Connection conn, BufferStage stage, String remoteStage)
      throws SQLException {
    logger.debug("COPY data in the stage to table: stage={}, name={}", remoteStage, stage.getId());
    _currentState = State.COPY_INTO_TABLE;
    _currentCommand =
        "COPY INTO "
            + StreamLoader.quoteIdentifier(stage.getId())
            + " FROM '"
            + remoteStage
            + "' on_error='"
            + _loader._onError
            + "'"
            + " file_format=("
            + " field_optionally_enclosed_by='\"'"
            + " empty_field_as_null="
            + Boolean.toString(!_loader._copyEmptyFieldAsEmpty)
            + ")";
    long loaded = 0;
    long parsed = 0;
    try (Statement copyStatement = conn.createStatement();
        ResultSet rs = copyStatement.executeQuery(_currentCommand)) {
      while (rs.next()) {
        loaded += rs.getLong("rows_loaded");
        parsed += rs.getLong("rows_parsed");
      }
    }

    int errorRecordCount = toIntExact(parsed - loaded);
    logger.debug(
        "errorRecordCount=[{}], parsed=[{}], loaded=[{}]", errorRecordCount, parsed, loaded);

    LoadResultListener listener = _loader.getListener();
    listener.addErrorRecordCount(errorRecordCount);

    if (loaded == stage.getRowCount()) {
      logger.debug(
          "COPY command successfully finished: stage={}, name={}", remoteStage, stage.getId());
      listener.addErrorCount(0);
      return true;
    }
    logger.debug("Found errors in COPY command: stage={}, name={}", remoteStage, stage.getId());
    if (!listener.needErrors()) {
      return true;
    }
    Loader.DataError dataError = reportLoadingErrors(conn, stage, remoteStage, listener);
    if (!listener.throwOnError()) {
      return true;
    }

    _loader.abort(dataError);
    if (!_loader._preserveStageFile) {
      logger.debug("RM: {}", remoteStage);
      StreamLoader.execute(conn, "RM '" + remoteStage + "'");
    } else {
      logger.error(
          "Error occurred. The remote stage is preserved for further investigation: {}",
          remoteStage);
    }
    return false;
  }

  /** Passes every validation error to the listener and returns the first one as a DataError. */
  private Loader.DataError reportLoadingErrors(
      Connection conn, BufferStage stage, String remoteStage, LoadResultListener listener)
      throws SQLException {
    _currentState = State.COPY_INTO_TABLE_ERROR;
    _currentCommand =
        "COPY INTO "
            + StreamLoader.quoteIdentifier(stage.getId())
            + " FROM '"
            + remoteStage
            + "' validation_mode='return_all_errors'"
            + " file_format=("
            + "field_optionally_enclosed_by='\"'"
            + "empty_field_as_null="
            + Boolean.toString(!_loader._copyEmptyFieldAsEmpty)
            + ")";

    int errorCount = 0;
    String lastErrorRow = "";
    Loader.DataError dataError = null;
    try (Statement validateStatement = conn.createStatement();
        ResultSet errorsSet = validateStatement.executeQuery(_currentCommand)) {
      while (errorsSet.next()) {
        errorCount++;
        String rn = errorsSet.getString(LoadingError.ErrorProperty.ROW_NUMBER.name());
        if (rn != null && !lastErrorRow.equals(rn)) {
          // de-duping records with multiple errors
          lastErrorRow = rn;
        }
        LoadingError loadError = new LoadingError(errorsSet, stage, _loader);

        listener.addError(loadError);
        if (dataError == null) {
          dataError = loadError.getException();
        }
      }
    }
    logger.debug("errorCount: {}", errorCount);

    listener.addErrorCount(errorCount);
    return dataError;
  }

  /** Applies the staging table to the target table and returns the update count. */
  private int ingest(Connection conn, BufferStage stage, String updateKeys) throws SQLException {
    _currentState = State.INGEST_DATA;
    String loadStatement = loadStatement(stage, updateKeys);
    _currentCommand = loadStatement;

    logger.debug("Running load statement: operation={}", stage.getOp());
    try (Statement s = conn.createStatement()) {
      s.execute(loadStatement);
      return s.getUpdateCount();
    }
  }

  private String loadStatement(BufferStage stage, String updateKeys) {
    switch (stage.getOp()) {
      case INSERT:
        return "INSERT INTO "
            + _loader.getFullTableName()
            + "("
            + _loader.getColumnsAsString()
            + ")"
            + " SELECT "
            + _loader.getStageColumnsAsString()
            + " FROM "
            + StreamLoader.quoteIdentifier(stage.getId());
      case DELETE:
        return "DELETE FROM "
            + _loader.getFullTableName()
            + " T USING "
            + StreamLoader.quoteIdentifier(stage.getId())
            + " AS S WHERE "
            + updateKeys;
      case MODIFY:
        return "MERGE INTO "
            + _loader.getFullTableName()
            + " T USING "
            + StreamLoader.quoteIdentifier(stage.getId())
            + " AS S ON "
            + updateKeys
            + " WHEN MATCHED THEN UPDATE SET "
            + setClause();
      case UPSERT:
        return "MERGE INTO "
            + _loader.getFullTableName()
            + " T USING "
            + StreamLoader.quoteIdentifier(stage.getId())
            + " AS S ON "
            + updateKeys
            + " WHEN MATCHED THEN UPDATE SET "
            + setClause()
            + " WHEN NOT MATCHED THEN INSERT("
            + _loader.getColumnsAsString()
            + ") VALUES"
            + valuesClause();
      default:
        return "";
    }
  }

  private String setClause() {
    StringBuilder setStatement = new StringBuilder(" ");
    List<String> columns = _loader.getColumns();
    for (int c = 0; c < columns.size(); ++c) {
      if (c > 0) {
        setStatement.append(", ");
      }
      String column = columns.get(c);
      setStatement
          .append("T.")
          .append(StreamLoader.quoteIdentifier(column))
          .append("=S.")
          .append(StreamLoader.quoteIdentifier(column));
    }
    return setStatement.toString();
  }

  private String valuesClause() {
    StringBuilder valueStatement = new StringBuilder("(");
    List<String> columns = _loader.getColumns();
    for (int c = 0; c < columns.size(); ++c) {
      if (c > 0) {
        valueStatement.append(" , ");
      }
      valueStatement.append("S.").append(StreamLoader.quoteIdentifier(columns.get(c)));
    }
    return valueStatement.append(")").toString();
  }

  private void reportCounts(BufferStage stage, int updateCount) {
    LoadResultListener listener = _loader.getListener();
    switch (stage.getOp()) {
      case INSERT:
      case UPSERT:
        listener.addProcessedRecordCount(stage.getOp(), stage.getRowCount());
        listener.addOperationRecordCount(stage.getOp(), updateCount);
        break;
      case DELETE:
      case MODIFY:
        // the number of successful DELETE is the number
        // of processed rows and not the number of given
        // rows.
        listener.addProcessedRecordCount(stage.getOp(), updateCount);
        listener.addOperationRecordCount(stage.getOp(), updateCount);
        break;
      default:
    }
  }

  private List<String> getAllColumns(final Connection conn) throws SQLException {
    List<String> columns = new LinkedList<>();
    try (Statement statement = conn.createStatement();
        ResultSet result =
            statement.executeQuery("show " + "columns" + " in " + _loader.getFullTableName())) {
      while (result.next()) {
        String col = result.getString("column_name");
        columns.add(col);
      }
    }
    return columns;
  }

  private String getOn(List<String> keys, String L, String R) {
    if (keys == null) {
      return "";
    }
    // L and R don't need to be quoted.
    StringBuilder sb = keys.size() > 1 ? new StringBuilder(64) : new StringBuilder();
    for (int i = 0; i < keys.size(); i++) {
      if (i > 0) {
        sb.append("AND ");
      }
      String key = StreamLoader.quoteIdentifier(keys.get(i));
      sb.append(L)
          .append('.')
          .append(key)
          .append(" = ")
          .append(R)
          .append('.')
          .append(key)
          .append(' ');
    }
    return sb.toString();
  }

  public void join() {
    logger.debug("Joining threads");
    try {
      _thread.join(0);
    } catch (InterruptedException ex) {
      logger.debug("Interrupted while joining the ProcessQueue thread");
    }
  }

  private enum State {
    INITIALIZE,
    CREATE_TEMP_TABLE,
    COPY_INTO_TABLE,
    COPY_INTO_TABLE_ERROR,
    INGEST_DATA,
    FINISH
  }
}
