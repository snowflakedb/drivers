namespace Snowflake.Data.Telemetry;

/// <summary>
/// TODO SNOW-4382577
/// </summary>
internal static class TelemetryActivities
{
    internal const string CommandCancel = "CommandCancel";
    internal const string ExecuteNonQuery = "ExecuteNonQuery";
    internal const string ExecuteNonQueryAsync = "ExecuteNonQueryAsync";
    internal const string ExecuteScalar = "ExecuteScalar";
    internal const string ExecuteScalarAsync = "ExecuteScalarAsync";
    internal const string ExecuteDbDataReader = "ExecuteDbDataReader";
    internal const string ExecuteDbDataReaderAsync = "ExecuteDbDataReaderAsync";
    internal const string ExecuteInAsyncMode = "ExecuteInAsyncMode";
    internal const string ExecuteAsyncInAsyncMode = "ExecuteAsyncInAsyncMode";
    internal const string GetQueryStatus = "GetQueryStatus";
    internal const string GetQueryStatusAsync = "GetQueryStatusAsync";
    internal const string GetResultsFromQueryId = "GetResultsFromQueryId";
    internal const string GetResultsFromQueryIdAsync = "GetResultsFromQueryIdAsync";
    internal const string ExecuteWithCustomUploadStream = "ExecuteWithCustomUploadStream";
}

// TODO SNOW-4382577
internal static class TelemetryEvents
{
    internal const string RowCountOverflow = "RowCountOverflow";
    internal const string RowCountNegative = "RowCountNegative";
    internal const string DqlResultSetSkipped = "DqlResultSetSkipped";
}
