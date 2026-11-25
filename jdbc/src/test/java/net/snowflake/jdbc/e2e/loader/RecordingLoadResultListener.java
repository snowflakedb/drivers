package net.snowflake.jdbc.e2e.loader;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import net.snowflake.client.api.loader.LoadResultListener;
import net.snowflake.client.api.loader.LoadingError;
import net.snowflake.client.api.loader.Operation;

/** Listener that records every callback so tests can assert the counts the loader reports. */
class RecordingLoadResultListener implements LoadResultListener {

  private final List<LoadingError> errors = Collections.synchronizedList(new ArrayList<>());

  private final AtomicInteger errorCount = new AtomicInteger(0);
  private final AtomicInteger errorRecordCount = new AtomicInteger(0);
  private final AtomicInteger submittedRowCount = new AtomicInteger(0);

  final AtomicInteger operationCount = new AtomicInteger(0);
  final AtomicInteger processed = new AtomicInteger(0);
  final AtomicInteger deleted = new AtomicInteger(0);
  final AtomicInteger updated = new AtomicInteger(0);

  private volatile Object[] lastRecord;

  private final boolean throwOnError;

  RecordingLoadResultListener() {
    this(false);
  }

  RecordingLoadResultListener(boolean throwOnError) {
    this.throwOnError = throwOnError;
  }

  List<LoadingError> getErrors() {
    return errors;
  }

  Object[] getLastRecord() {
    return lastRecord;
  }

  @Override
  public boolean needErrors() {
    return true;
  }

  @Override
  public boolean needSuccessRecords() {
    return true;
  }

  @Override
  public void addError(LoadingError error) {
    errors.add(error);
  }

  @Override
  public boolean throwOnError() {
    return throwOnError;
  }

  @Override
  public void recordProvided(Operation op, Object[] record) {
    lastRecord = record;
  }

  @Override
  public void addProcessedRecordCount(Operation op, int i) {
    processed.addAndGet(i);
  }

  @Override
  public void addOperationRecordCount(Operation op, int i) {
    operationCount.addAndGet(i);
    if (op == Operation.DELETE) {
      deleted.addAndGet(i);
    } else if (op == Operation.MODIFY || op == Operation.UPSERT) {
      updated.addAndGet(i);
    }
  }

  @Override
  public int getErrorCount() {
    return errorCount.get();
  }

  @Override
  public int getErrorRecordCount() {
    return errorRecordCount.get();
  }

  @Override
  public void resetErrorCount() {
    errorCount.set(0);
  }

  @Override
  public void resetErrorRecordCount() {
    errorRecordCount.set(0);
  }

  @Override
  public void addErrorCount(int count) {
    errorCount.addAndGet(count);
  }

  @Override
  public void addErrorRecordCount(int count) {
    errorRecordCount.addAndGet(count);
  }

  @Override
  public void resetSubmittedRowCount() {
    submittedRowCount.set(0);
  }

  @Override
  public void addSubmittedRowCount(int count) {
    submittedRowCount.addAndGet(count);
  }

  @Override
  public int getSubmittedRowCount() {
    return submittedRowCount.get();
  }
}
