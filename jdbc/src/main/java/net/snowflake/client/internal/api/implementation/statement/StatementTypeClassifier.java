package net.snowflake.client.internal.api.implementation.statement;

import lombok.experimental.UtilityClass;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ExecuteResult;

@UtilityClass
class StatementTypeClassifier {
  public static final long NO_UPDATE_COUNT = -1L;

  static boolean producesResultSet(ExecuteResult executeResult) {
    return lookupType(executeResult).producesResultSet();
  }

  static long getUpdateCount(ExecuteResult executeResult) {
    StatementType statementType = lookupType(executeResult);
    if (statementType.producesResultSet()) {
      return NO_UPDATE_COUNT;
    }

    return statementType.isDml() && executeResult.hasRowsAffected()
        ? executeResult.getRowsAffected()
        : 0L;
  }

  private static StatementType lookupType(ExecuteResult executeResult) {
    if (!executeResult.hasStatementTypeId()) {
      return StatementType.UNKNOWN;
    }
    return StatementType.lookupById(executeResult.getStatementTypeId());
  }

  private enum StatementType {
    UNKNOWN(0x0000L, true),
    SELECT(0x1000L, true),
    DML(0x3000L, false),
    INSERT(0x3000L + 0x100L, false),
    UPDATE(0x3000L + 0x200L, false),
    DELETE(0x3000L + 0x300L, false),
    MERGE(0x3000L + 0x400L, false),
    MULTI_INSERT(0x3000L + 0x500L, false),
    COPY(0x3000L + 0x600L, false),
    UNLOAD(0x3000L + 0x700L, false),
    RECLUSTER(0x3000L + 0x800L, false),
    SCL(0x4000L, false),
    ALTER_SESSION(0x4000L + 0x100L, false),
    USE(0x4000L + 0x300L, false),
    USE_DATABASE(0x4000L + 0x300L + 0x01L, false),
    USE_SCHEMA(0x4000L + 0x300L + 0x02L, false),
    USE_WAREHOUSE(0x4000L + 0x300L + 0x03L, false),
    SHOW(0x4000L + 0x400L, true),
    DESCRIBE(0x4000L + 0x500L, true),
    LIST(0x4000L + 0x700L + 0x01L, true),
    TCL(0x5000L, false),
    DDL(0x6000L, false),
    ALTER_USER_MANAGE_PATS(0x6000L + 0x200L + 0x44L, true),
    GET(0x7000L + 0x100L + 0x01L, true),
    PUT(0x7000L + 0x100L + 0x02L, true),
    REMOVE(0x7000L + 0x100L + 0x03L, true);

    private static final long CATEGORY_RANGE = 0x1000L;

    private final long statementTypeId;
    private final boolean producesResultSet;

    StatementType(long statementTypeId, boolean producesResultSet) {
      this.statementTypeId = statementTypeId;
      this.producesResultSet = producesResultSet;
    }

    static StatementType lookupById(long statementTypeId) {
      for (StatementType type : values()) {
        if (type.statementTypeId == statementTypeId) {
          return type;
        }
      }

      if (statementTypeId >= SCL.statementTypeId
          && statementTypeId < SCL.statementTypeId + CATEGORY_RANGE) {
        return SCL;
      } else if (statementTypeId >= TCL.statementTypeId
          && statementTypeId < TCL.statementTypeId + CATEGORY_RANGE) {
        return TCL;
      } else if (statementTypeId >= DDL.statementTypeId
          && statementTypeId < DDL.statementTypeId + CATEGORY_RANGE) {
        return DDL;
      } else {
        return UNKNOWN;
      }
    }

    boolean producesResultSet() {
      return producesResultSet;
    }

    boolean isDml() {
      return statementTypeId >= DML.statementTypeId
          && statementTypeId < DML.statementTypeId + CATEGORY_RANGE;
    }
  }
}
