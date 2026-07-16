cmake_minimum_required(VERSION 4.0)

# Replay tests generated from Excel VBA + ADO traces.
#
# Excel reaches the driver here through MSDASQL (ADO's OLE DB → ODBC
# bridge), which surfaces several ODBC 2.x APIs that don't appear in
# the Power Query traces — SQLBindCol, SQLBindParameter,
# SQLExtendedFetch, SQLTransact, etc. See `docs/design/excel-odbc-trace-test-plan`
# for the per-scenario driver-axis mapping.
add_odbc_test(replay_excel_ado_sqlbindcol sqlbindcol/test.cpp)
add_odbc_test(replay_excel_ado_prepared_exec prepared_exec/test.cpp)
