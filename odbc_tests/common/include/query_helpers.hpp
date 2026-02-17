#ifndef QUERY_HELPERS_HPP
#define QUERY_HELPERS_HPP

#include <sql.h>

#include <string>

// Query the current database name using a temporary statement on the given connection.
// Allocates and frees its own statement handle so the caller's statement state is not mutated.
// Throws std::runtime_error if any ODBC call fails.
std::string get_current_database(SQLHDBC dbc);

#endif  // QUERY_HELPERS_HPP
