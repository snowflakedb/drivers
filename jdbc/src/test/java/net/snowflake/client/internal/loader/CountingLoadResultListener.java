package net.snowflake.client.internal.loader;

import net.snowflake.client.api.loader.LoadResultListener;
import net.snowflake.client.api.loader.LoadingError;
import net.snowflake.client.api.loader.Operation;

class CountingLoadResultListener implements LoadResultListener {
  private final boolean throwOnError;
  private int errorCount;
  private int errorRecordCount;
  private int submittedRowCount;

  CountingLoadResultListener(boolean throwOnError) {
    this.throwOnError = throwOnError;
  }

  @Override
  public boolean needErrors() {
    return true;
  }

  @Override
  public boolean needSuccessRecords() {
    return false;
  }

  @Override
  public void addError(LoadingError error) {}

  @Override
  public boolean throwOnError() {
    return throwOnError;
  }

  @Override
  public void recordProvided(Operation op, Object[] record) {}

  @Override
  public void addProcessedRecordCount(Operation op, int i) {}

  @Override
  public void addOperationRecordCount(Operation op, int i) {}

  @Override
  public synchronized int getErrorCount() {
    return errorCount;
  }

  @Override
  public synchronized int getErrorRecordCount() {
    return errorRecordCount;
  }

  @Override
  public synchronized void resetErrorCount() {
    errorCount = 0;
  }

  @Override
  public synchronized void resetErrorRecordCount() {
    errorRecordCount = 0;
  }

  @Override
  public synchronized void addErrorCount(int count) {
    errorCount += count;
  }

  @Override
  public synchronized void addErrorRecordCount(int count) {
    errorRecordCount += count;
  }

  @Override
  public synchronized void resetSubmittedRowCount() {
    submittedRowCount = 0;
  }

  @Override
  public synchronized void addSubmittedRowCount(int count) {
    submittedRowCount += count;
  }

  @Override
  public synchronized int getSubmittedRowCount() {
    return submittedRowCount;
  }
}
