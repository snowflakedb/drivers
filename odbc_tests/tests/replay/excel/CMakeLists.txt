cmake_minimum_required(VERSION 4.0)

# Replay tests generated from Excel ODBC traces.
#
# Layout:
#   excel/
#     powerquery/      - Power Query (Get Data) connector
#       raw_sql/       - "Advanced options" > custom SQL text
#       navigator/     - Navigator-driven catalog/schema/table browsing
#     ado/             - Excel's older ADO/MSQuery path
add_subdirectory(powerquery)
