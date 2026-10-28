cmake_minimum_required(VERSION 4.0)

# Replay tests generated from Tableau ODBC traces.
#
# Layout:
#   tableau/
#     generic_odbc/    - Tableau's generic ODBC connector
#     official/        - Tableau's official Snowflake connector
add_subdirectory(generic_odbc)
add_subdirectory(official)
