package net.snowflake.client.internal.api.implementation.connection;

import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;

/**
 * Maps Core driver query status responses to the public {@link QueryStatus} API object.
 *
 * <p>The Core driver returns a status name string (e.g. "RUNNING", "SUCCESS") and optional error
 * fields. This mapper converts those into a fully populated {@link QueryStatus} instance.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
class QueryStatusMapper {

  static QueryStatus fromCoreResponse(ConnectionGetQueryStatusResponse response) {
    String statusName = response.getStatusName();
    int errorCode = response.hasErrorCode() ? response.getErrorCode() : 0;
    String errorMessage = response.hasErrorMessage() ? response.getErrorMessage() : "";

    // TODO: ConnectionGetQueryStatusResponse currently only provides statusName, errorCode, and
    //  errorMessage. The following fields are available in the old JDBC driver's query status
    //  endpoint but not yet surfaced by the core driver proto:
    //  - endTime, startTime, totalDuration (query timing)
    //  - id (query ID — the caller already knows it, but the response should echo it)
    //  - sessionId
    //  - sqlText (the original SQL)
    //  - warehouseId, warehouseName, warehouseExternalSize, warehouseServerType
    //  Once the core driver proto is extended, populate these from the response instead of
    // defaults.
    return new QueryStatus(
        /* endTime= */ 0,
        errorCode,
        errorMessage,
        /* id= */ "",
        /* name= */ statusName,
        /* sessionId= */ 0,
        /* sqlText= */ "",
        /* startTime= */ 0,
        /* state= */ statusName,
        /* totalDuration= */ 0,
        /* warehouseExternalSize= */ "",
        /* warehouseId= */ 0,
        /* warehouseName= */ "",
        /* warehouseServerType= */ "");
  }
}
