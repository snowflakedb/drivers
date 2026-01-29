package net.snowflake.client.internal.core.arrow.cursor;

public final class CursorState {
  private boolean wasNull = false;
  private int currentRow = -1;
  private boolean afterLast = false;

  public void reset() {
    wasNull = false;
    currentRow = -1;
    afterLast = false;
  }

  public boolean wasNull() {
    return wasNull;
  }

  public void setWasNull(boolean wasNull) {
    this.wasNull = wasNull;
  }

  public int getCurrentRow() {
    return currentRow;
  }

  public void incrementRow() {
    currentRow++;
  }

  public boolean isAfterLast() {
    return afterLast;
  }

  public void setAfterLast() {
    this.afterLast = true;
  }
}
