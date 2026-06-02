package net.snowflake.client.internal.api.implementation.connection;

import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import net.snowflake.client.api.resultset.QueryStatus;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetQueryStatusResponse;

/** Maps Core driver query status responses to the public {@link QueryStatus} API object. */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
class QueryStatusMapper {

  static QueryStatus fromCoreResponse(ConnectionGetQueryStatusResponse response) {
    // Proto3 defaults match QueryStatus defaults, so no "hasField" checks.
    return new QueryStatus(
        response.getEndTime(),
        response.getErrorCode(),
        response.getErrorMessage(),
        response.getQueryId(),
        response.getStatusName(),
        response.getSessionId(),
        response.getSqlText(),
        response.getStartTime(),
        response.getState(),
        response.getTotalDuration(),
        response.getWarehouseExternalSize(),
        (int) response.getWarehouseId(),
        response.getWarehouseName(),
        response.getWarehouseServerType());
  }
}
