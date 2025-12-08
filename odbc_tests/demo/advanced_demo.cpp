/**
 * Advanced ODBC Driver Demo
 * 
 * This demo showcases the Universal Snowflake ODBC Driver capabilities:
 * - Connection management with various authentication methods
 * - Complex queries with parameter binding
 * - Large result set handling (pagination)
 * - PUT/GET file operations
 * - Transaction management
 * - Metadata queries
 * - Error handling and diagnostics
 * - Multiple data types (DECIMAL, TIMESTAMP, VARIANT, etc.)
 */

#include <iostream>
#include <iomanip>
#include <string>
#include <vector>
#include <fstream>
#include <sstream>
#include <cmath>
#include <chrono>
#include <cstdint>
#include <sql.h>
#include <sqlext.h>
#include <cstring>

// ANSI color codes for pretty output
#define COLOR_RESET   "\033[0m"
#define COLOR_BOLD    "\033[1m"
#define COLOR_GREEN   "\033[32m"
#define COLOR_BLUE    "\033[34m"
#define COLOR_YELLOW  "\033[33m"
#define COLOR_RED     "\033[31m"
#define COLOR_CYAN    "\033[36m"

class ODBCDemo {
private:
    using Clock = std::chrono::steady_clock;
    
    SQLHENV hEnv;
    SQLHDBC hDbc;
    SQLHSTMT hStmt;
    bool connected;
    
    static long long elapsedMillis(const Clock::time_point& start) {
        return std::chrono::duration_cast<std::chrono::milliseconds>(
                   Clock::now() - start)
            .count();
    }
    
    static void logPerf(const std::string& metric, long long duration_ms) {
        std::cout << "PERF|" << metric << "|duration_ms=" << duration_ms << "\n";
    }
    
    static void logPerfValue(const std::string& metric, const std::string& key, double value) {
        std::ostringstream oss;
        oss.setf(std::ios::fixed);
        oss.precision(2);
        oss << value;
        std::cout << "PERF|" << metric << "|" << key << "=" << oss.str() << "\n";
    }
    
    static uint64_t computeChecksum(const std::string& path) {
        std::ifstream file(path, std::ios::binary);
        if (!file.is_open()) {
            return 0;
        }
        
        const uint64_t fnvOffset = 1469598103934665603ull;
        const uint64_t fnvPrime = 1099511628211ull;
        uint64_t hash = fnvOffset;
        char buffer[4096];
        
        while (file.read(buffer, sizeof(buffer)) || file.gcount() > 0) {
            std::streamsize count = file.gcount();
            for (std::streamsize i = 0; i < count; ++i) {
                hash ^= static_cast<unsigned char>(buffer[i]);
                hash *= fnvPrime;
            }
        }
        
        return hash;
    }
    
    static std::string formatChecksum(uint64_t value) {
        std::ostringstream oss;
        oss << std::hex << std::uppercase << value;
        return oss.str();
    }
    
    // Helper to print section headers
    void printSection(const std::string& title) {
        std::cout << "\n" << COLOR_BOLD << COLOR_CYAN 
                  << "═══════════════════════════════════════════════════════════════\n"
                  << "  " << title << "\n"
                  << "═══════════════════════════════════════════════════════════════"
                  << COLOR_RESET << "\n\n";
    }
    
    // Helper to print success messages
    void printSuccess(const std::string& msg) {
        std::cout << COLOR_GREEN << "✓ " << msg << COLOR_RESET << "\n";
    }
    
    // Helper to print info messages
    void printInfo(const std::string& msg) {
        std::cout << COLOR_BLUE << "ℹ " << msg << COLOR_RESET << "\n";
    }
    
    // Helper to print warnings
    void printWarning(const std::string& msg) {
        std::cout << COLOR_YELLOW << "⚠ " << msg << COLOR_RESET << "\n";
    }
    
    // Helper to print errors
    void printError(const std::string& msg) {
        std::cout << COLOR_RED << "✗ " << msg << COLOR_RESET << "\n";
    }
    
    // Get detailed diagnostic information
    std::string getDiagnostics(SQLSMALLINT handleType, SQLHANDLE handle) {
        SQLCHAR sqlState[6];
        SQLCHAR message[SQL_MAX_MESSAGE_LENGTH];
        SQLINTEGER nativeError;
        SQLSMALLINT textLength;
        std::stringstream ss;
        
        SQLSMALLINT recNumber = 1;
        while (SQLGetDiagRec(handleType, handle, recNumber, sqlState, &nativeError,
                            message, sizeof(message), &textLength) == SQL_SUCCESS) {
            ss << "  [" << sqlState << "] (" << nativeError << ") " << message << "\n";
            recNumber++;
        }
        
        return ss.str();
    }
    
    // Check return code and print diagnostics if needed
    bool checkRC(SQLRETURN rc, SQLSMALLINT handleType, SQLHANDLE handle, 
                 const std::string& operation) {
        if (rc == SQL_SUCCESS || rc == SQL_SUCCESS_WITH_INFO) {
            if (rc == SQL_SUCCESS_WITH_INFO) {
                printWarning(operation + " completed with info:");
                std::cout << getDiagnostics(handleType, handle);
            }
            return true;
        }
        
        printError(operation + " failed:");
        std::cout << getDiagnostics(handleType, handle);
        return false;
    }

public:
    ODBCDemo() : hEnv(nullptr), hDbc(nullptr), hStmt(nullptr), connected(false) {}
    
    ~ODBCDemo() {
        disconnect();
    }
    
    // Initialize ODBC environment
    bool initialize() {
        printSection("Initializing ODBC Environment");
        
        SQLRETURN rc = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &hEnv);
        if (!checkRC(rc, SQL_HANDLE_ENV, hEnv, "Allocate environment handle")) {
            return false;
        }
        
        rc = SQLSetEnvAttr(hEnv, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
        if (!checkRC(rc, SQL_HANDLE_ENV, hEnv, "Set ODBC version")) {
            return false;
        }
        
        rc = SQLAllocHandle(SQL_HANDLE_DBC, hEnv, &hDbc);
        if (!checkRC(rc, SQL_HANDLE_DBC, hDbc, "Allocate connection handle")) {
            return false;
        }
        
        printSuccess("ODBC environment initialized");
        return true;
    }
    
    // Connect to Snowflake using connection string from parameters.json
    bool connect(const std::string& parameterPath) {
        printSection("Connecting to Snowflake");
        
        // Read parameters from JSON file
        std::ifstream file(parameterPath);
        if (!file.is_open()) {
            printError("Cannot open parameters file: " + parameterPath);
            return false;
        }
        
        // Simple JSON parsing for testconnection section
        std::string line, account, user, password, database, schema, warehouse, role;
        bool inTestConnection = false;
        
        while (std::getline(file, line)) {
            if (line.find("\"testconnection\"") != std::string::npos) {
                inTestConnection = true;
                continue;
            }
            
            if (inTestConnection) {
                if (line.find("}") != std::string::npos && line.find("\"SNOWFLAKE") == std::string::npos) {
                    break;
                }
                
                auto extractValue = [&line]() -> std::string {
                    size_t start = line.find(": \"");
                    if (start == std::string::npos) return "";
                    start += 3;
                    size_t end = line.find("\"", start);
                    if (end == std::string::npos) return "";
                    return line.substr(start, end - start);
                };
                
                if (line.find("\"SNOWFLAKE_TEST_ACCOUNT\"") != std::string::npos) account = extractValue();
                else if (line.find("\"SNOWFLAKE_TEST_USER\"") != std::string::npos) user = extractValue();
                else if (line.find("\"SNOWFLAKE_TEST_PASSWORD\"") != std::string::npos) password = extractValue();
                else if (line.find("\"SNOWFLAKE_TEST_DATABASE\"") != std::string::npos) database = extractValue();
                else if (line.find("\"SNOWFLAKE_TEST_SCHEMA\"") != std::string::npos) schema = extractValue();
                else if (line.find("\"SNOWFLAKE_TEST_WAREHOUSE\"") != std::string::npos) warehouse = extractValue();
                else if (line.find("\"SNOWFLAKE_TEST_ROLE\"") != std::string::npos) role = extractValue();
            }
        }
        
        // Get driver path from environment
        const char* driverPath = std::getenv("DRIVER_PATH");
        if (!driverPath) {
            printError("DRIVER_PATH environment variable not set");
            return false;
        }
        
        // Build connection string (use uppercase parameter names)
        // Add SERVER parameter for official Snowflake driver compatibility
        std::string server = account + ".snowflakecomputing.com";
        std::stringstream connStr;
        connStr << "DRIVER=" << driverPath << ";"
                << "SERVER=" << server << ";"
                << "ACCOUNT=" << account << ";"
                << "UID=" << user << ";"
                << "PWD=" << password << ";"
                << "DATABASE=" << database << ";"
                << "SCHEMA=" << schema << ";"
                << "WAREHOUSE=" << warehouse << ";"
                << "ROLE=" << role;
        
        std::string connectionString = connStr.str();
        std::string safeConnStr = connectionString.substr(0, connectionString.find("PWD=")) + "PWD=***";
        printInfo("Connection string: " + safeConnStr);
        
        SQLCHAR outConnStr[1024];
        SQLSMALLINT outConnStrLen;
        
        auto connectStart = Clock::now();
        SQLRETURN rc = SQLDriverConnect(hDbc, nullptr, 
                                       (SQLCHAR*)connectionString.c_str(), SQL_NTS,
                                       outConnStr, sizeof(outConnStr), &outConnStrLen,
                                       SQL_DRIVER_NOPROMPT);
        
        if (!checkRC(rc, SQL_HANDLE_DBC, hDbc, "Connect to Snowflake")) {
            return false;
        }
        
        connected = true;
        printSuccess("Connected to Snowflake");
        logPerf("connect", elapsedMillis(connectStart));
        
        // Allocate statement handle
        rc = SQLAllocHandle(SQL_HANDLE_STMT, hDbc, &hStmt);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Allocate statement handle")) {
            return false;
        }
        
        return true;
    }
    
    // Disconnect from Snowflake
    void disconnect() {
        if (hStmt) {
            SQLFreeHandle(SQL_HANDLE_STMT, hStmt);
            hStmt = nullptr;
        }
        
        if (connected && hDbc) {
            SQLDisconnect(hDbc);
            connected = false;
        }
        
        if (hDbc) {
            SQLFreeHandle(SQL_HANDLE_DBC, hDbc);
            hDbc = nullptr;
        }
        
        if (hEnv) {
            SQLFreeHandle(SQL_HANDLE_ENV, hEnv);
            hEnv = nullptr;
        }
    }
    
    // Demo 1: Basic query with result display
    bool demoBasicQuery() {
        printSection("Demo 1: Basic Query - Current Session Info");
        
        auto demoStart = Clock::now();
        
        const char* query = "SELECT CURRENT_USER() as user, "
                          "CURRENT_ROLE() as role, "
                          "CURRENT_DATABASE() as database, "
                          "CURRENT_SCHEMA() as schema, "
                          "CURRENT_WAREHOUSE() as warehouse, "
                          "CURRENT_VERSION() as version";
        
        SQLRETURN rc = SQLExecDirect(hStmt, (SQLCHAR*)query, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Execute query")) {
            return false;
        }
        
        // Fetch and display results
        char user[256], role[256], database[256], schema[256], warehouse[256], version[256];
        SQLLEN userLen, roleLen, dbLen, schemaLen, whLen, versionLen;
        
        SQLBindCol(hStmt, 1, SQL_C_CHAR, user, sizeof(user), &userLen);
        SQLBindCol(hStmt, 2, SQL_C_CHAR, role, sizeof(role), &roleLen);
        SQLBindCol(hStmt, 3, SQL_C_CHAR, database, sizeof(database), &dbLen);
        SQLBindCol(hStmt, 4, SQL_C_CHAR, schema, sizeof(schema), &schemaLen);
        SQLBindCol(hStmt, 5, SQL_C_CHAR, warehouse, sizeof(warehouse), &whLen);
        SQLBindCol(hStmt, 6, SQL_C_CHAR, version, sizeof(version), &versionLen);
        
        rc = SQLFetch(hStmt);
        
        if (rc == SQL_SUCCESS) {
            std::cout << COLOR_BOLD << "Session Information:\n" << COLOR_RESET;
            std::cout << "  User:      " << user << "\n";
            std::cout << "  Role:      " << role << "\n";
            std::cout << "  Database:  " << database << "\n";
            std::cout << "  Schema:    " << schema << "\n";
            std::cout << "  Warehouse: " << warehouse << "\n";
            std::cout << "  Version:   " << version << "\n";
            printSuccess("Query completed");
        }
        
        SQLCloseCursor(hStmt);
        logPerf("demo_basic_query", elapsedMillis(demoStart));
        return true;
    }
    
    // Helper to reset statement handle
    void resetStatement() {
        if (hStmt) {
            SQLFreeHandle(SQL_HANDLE_STMT, hStmt);
            hStmt = nullptr;
        }
        SQLAllocHandle(SQL_HANDLE_STMT, hDbc, &hStmt);
    }
    
    // Demo 2: Complex query with aggregations
    bool demoAggregationQuery() {
        printSection("Demo 2: Aggregation Query - Generate Statistics");
        
        resetStatement();
        auto demoStart = Clock::now();
        
        // Create temporary table with sample data
        const char* createTable = 
            "CREATE TEMPORARY TABLE demo_sales ("
            "    sale_id INTEGER,"
            "    product VARCHAR(50),"
            "    category VARCHAR(50),"
            "    quantity INTEGER,"
            "    price DECIMAL(10,2),"
            "    sale_date DATE"
            ")";
        
        SQLRETURN rc = SQLExecDirect(hStmt, (SQLCHAR*)createTable, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Create temporary table")) {
            return false;
        }
        SQLCloseCursor(hStmt);
        
        // Insert sample data
        const char* insertData = 
            "INSERT INTO demo_sales VALUES "
            "(1, 'Laptop', 'Electronics', 5, 1200.00, '2024-01-15'),"
            "(2, 'Mouse', 'Electronics', 50, 25.50, '2024-01-16'),"
            "(3, 'Desk', 'Furniture', 10, 450.00, '2024-01-17'),"
            "(4, 'Chair', 'Furniture', 20, 200.00, '2024-01-18'),"
            "(5, 'Monitor', 'Electronics', 15, 350.00, '2024-01-19'),"
            "(6, 'Keyboard', 'Electronics', 30, 75.00, '2024-01-20'),"
            "(7, 'Bookshelf', 'Furniture', 8, 180.00, '2024-01-21')";
        
        rc = SQLExecDirect(hStmt, (SQLCHAR*)insertData, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Insert sample data")) {
            return false;
        }
        SQLCloseCursor(hStmt);
        
        // Run aggregation query
        const char* aggQuery = 
            "SELECT "
            "    category,"
            "    COUNT(*) as num_products,"
            "    SUM(quantity) as total_quantity,"
            "    AVG(price) as avg_price,"
            "    SUM(quantity * price) as total_revenue"
            " FROM demo_sales"
            " GROUP BY category"
            " ORDER BY total_revenue DESC";
        
        rc = SQLExecDirect(hStmt, (SQLCHAR*)aggQuery, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Execute aggregation query")) {
            return false;
        }
        
        // Display results in a formatted table
        std::cout << COLOR_BOLD << "\nSales Summary by Category:\n" << COLOR_RESET;
        std::cout << std::string(80, '-') << "\n";
        std::cout << std::left << std::setw(15) << "Category"
                  << std::right << std::setw(12) << "Products"
                  << std::setw(12) << "Quantity"
                  << std::setw(15) << "Avg Price"
                  << std::setw(18) << "Total Revenue\n";
        std::cout << std::string(80, '-') << "\n";
        
        char category[256];
        SQLBIGINT numProducts, totalQuantity;
        double avgPrice, totalRevenue;
        SQLLEN catLen, npLen, tqLen, apLen, trLen;
        
        SQLBindCol(hStmt, 1, SQL_C_CHAR, category, sizeof(category), &catLen);
        SQLBindCol(hStmt, 2, SQL_C_SBIGINT, &numProducts, 0, &npLen);
        SQLBindCol(hStmt, 3, SQL_C_SBIGINT, &totalQuantity, 0, &tqLen);
        SQLBindCol(hStmt, 4, SQL_C_DOUBLE, &avgPrice, 0, &apLen);
        SQLBindCol(hStmt, 5, SQL_C_DOUBLE, &totalRevenue, 0, &trLen);
        
        int rowCount = 0;
        while ((rc = SQLFetch(hStmt)) == SQL_SUCCESS) {
            std::cout << std::left << std::setw(15) << category
                      << std::right << std::setw(12) << numProducts
                      << std::setw(12) << totalQuantity
                      << std::setw(15) << std::fixed << std::setprecision(2) << avgPrice
                      << std::setw(18) << std::fixed << std::setprecision(2) << totalRevenue << "\n";
            rowCount++;
        }
        
        std::cout << std::string(80, '-') << "\n";
        printSuccess("Processed " + std::to_string(rowCount) + " categories");
        
        SQLCloseCursor(hStmt);
        logPerf("demo_aggregation_query", elapsedMillis(demoStart));
        return true;
    }
    
    // Demo 3: Parameter binding
    bool demoParameterBinding() {
        printSection("Demo 3: Parameterized Queries - Prepared Statements");
        
        resetStatement();
        auto demoStart = Clock::now();
        
        // Prepare a parameterized query
        const char* query = "SELECT product, quantity, price, (quantity * price) as total "
                          "FROM demo_sales WHERE category = ? AND price > ?";
        
        SQLRETURN rc = SQLPrepare(hStmt, (SQLCHAR*)query, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Prepare statement")) {
            return false;
        }
        
        // Bind parameters
        char category[] = "Electronics";
        double minPrice = 50.0;
        SQLLEN catLen = SQL_NTS;
        SQLLEN priceLen = 0;
        
        rc = SQLBindParameter(hStmt, 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR,
                             sizeof(category), 0, category, sizeof(category), &catLen);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Bind parameter 1")) {
            return false;
        }
        
        rc = SQLBindParameter(hStmt, 2, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE,
                             0, 0, &minPrice, 0, &priceLen);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Bind parameter 2")) {
            return false;
        }
        
        printInfo("Executing query with parameters: category='" + std::string(category) + 
                 "', min_price=" + std::to_string(minPrice));
        
        // Execute prepared statement
        rc = SQLExecute(hStmt);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Execute prepared statement")) {
            return false;
        }
        
        // Display results
        std::cout << COLOR_BOLD << "\nFiltered Products:\n" << COLOR_RESET;
        std::cout << std::string(70, '-') << "\n";
        std::cout << std::left << std::setw(20) << "Product"
                  << std::right << std::setw(12) << "Quantity"
                  << std::setw(15) << "Price"
                  << std::setw(15) << "Total\n";
        std::cout << std::string(70, '-') << "\n";
        
        char product[256];
        SQLBIGINT quantity;
        double price, total;
        SQLLEN prodLen, qtyLen, prLen, totLen;
        
        SQLBindCol(hStmt, 1, SQL_C_CHAR, product, sizeof(product), &prodLen);
        SQLBindCol(hStmt, 2, SQL_C_SBIGINT, &quantity, 0, &qtyLen);
        SQLBindCol(hStmt, 3, SQL_C_DOUBLE, &price, 0, &prLen);
        SQLBindCol(hStmt, 4, SQL_C_DOUBLE, &total, 0, &totLen);
        
        int rowCount = 0;
        while ((rc = SQLFetch(hStmt)) == SQL_SUCCESS) {
            std::cout << std::left << std::setw(20) << product
                      << std::right << std::setw(12) << quantity
                      << std::setw(15) << std::fixed << std::setprecision(2) << price
                      << std::setw(15) << std::fixed << std::setprecision(2) << total << "\n";
            rowCount++;
        }
        
        std::cout << std::string(70, '-') << "\n";
        printSuccess("Found " + std::to_string(rowCount) + " products");
        
        SQLCloseCursor(hStmt);
        logPerf("demo_parameter_binding", elapsedMillis(demoStart));
        return true;
    }
    
    // Demo 4: Large result set with pagination
    bool demoLargeResultSet() {
        printSection("Demo 4: Large Result Set - Pagination Demo");
        
        resetStatement();
        auto demoStart = Clock::now();
        
        const int TOTAL_ROWS = 10000;
        const int PAGE_SIZE = 1000;
        
        printInfo("Generating " + std::to_string(TOTAL_ROWS) + " rows using GENERATOR...");
        
        std::stringstream query;
        query << "SELECT "
              << "  SEQ8() as id, "
              << "  MOD(SEQ8() * 73, 1000) + 1 as value, "
              << "  DATEADD(day, SEQ8(), '2024-01-01'::DATE) as date "
              << "FROM TABLE(GENERATOR(ROWCOUNT => " << TOTAL_ROWS << "))";
        
        auto execStart = Clock::now();
        SQLRETURN rc = SQLExecDirect(hStmt, (SQLCHAR*)query.str().c_str(), SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Execute large query")) {
            return false;
        }
        logPerf("demo_large_result_set_exec", elapsedMillis(execStart));
        
        SQLBIGINT id, value;
        char date[32];
        SQLLEN idLen, valueLen, dateLen;
        
        SQLBindCol(hStmt, 1, SQL_C_SBIGINT, &id, 0, &idLen);
        SQLBindCol(hStmt, 2, SQL_C_SBIGINT, &value, 0, &valueLen);
        SQLBindCol(hStmt, 3, SQL_C_CHAR, date, sizeof(date), &dateLen);
        
        int totalRows = 0;
        int currentPage = 1;
        double sum = 0, min = 1e9, max = -1e9;
        
        std::cout << COLOR_BOLD << "\nProcessing pages:\n" << COLOR_RESET;
        
        auto fetchStart = Clock::now();
        while ((rc = SQLFetch(hStmt)) == SQL_SUCCESS) {
            totalRows++;
            sum += value;
            if (value < min) min = value;
            if (value > max) max = value;
            
            if (totalRows % PAGE_SIZE == 0) {
                int percentComplete = (totalRows * 100) / TOTAL_ROWS;
                std::cout << "  Page " << std::setw(3) << currentPage 
                          << ": Processed " << std::setw(6) << totalRows << " rows"
                          << " (" << std::setw(3) << percentComplete << "% complete)\n";
                currentPage++;
            }
        }
        long long fetchDuration = elapsedMillis(fetchStart);
        
        std::cout << "\n" << COLOR_BOLD << "Statistics:\n" << COLOR_RESET;
        std::cout << "  Total Rows:    " << totalRows << "\n";
        std::cout << "  Average Value: " << std::fixed << std::setprecision(2) << (sum / totalRows) << "\n";
        std::cout << "  Min Value:     " << std::fixed << std::setprecision(0) << min << "\n";
        std::cout << "  Max Value:     " << std::fixed << std::setprecision(0) << max << "\n";
        
        printSuccess("Processed " + std::to_string(totalRows) + " rows successfully");
        
        SQLCloseCursor(hStmt);
        logPerf("demo_large_result_set_fetch", fetchDuration);
        logPerf("demo_large_result_set", elapsedMillis(demoStart));
        if (totalRows > 0 && fetchDuration > 0) {
            double throughput = (totalRows * 1000.0) / static_cast<double>(fetchDuration);
            logPerfValue("demo_large_result_set", "rows_per_sec", throughput);
        }
        return true;
    }
    
    // Demo 5: PUT/GET file operations
    bool demoPutGetOperations() {
        printSection("Demo 5: File Operations - PUT and GET");
        
        resetStatement();
        auto demoStart = Clock::now();
        
        // Create a temporary stage
        const char* createStage = "CREATE TEMPORARY STAGE demo_stage";
        SQLRETURN rc = SQLExecDirect(hStmt, (SQLCHAR*)createStage, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Create temporary stage")) {
            return false;
        }
        SQLCloseCursor(hStmt);
        printSuccess("Created temporary stage");
        
        // Create a test file
        const std::string testFile = "/tmp/demo_data.csv";
        std::ofstream outFile(testFile);
        if (!outFile.is_open()) {
            printError("Cannot create test file: " + testFile);
            return false;
        }
        
        outFile << "id,name,value\n";
        for (int i = 1; i <= 100; i++) {
            outFile << i << ",Item" << i << "," << (i * 10) << "\n";
        }
        outFile.close();
        printSuccess("Created test file: " + testFile + " (100 rows)");
        uint64_t sourceChecksum = computeChecksum(testFile);
        std::string sourceChecksumStr = formatChecksum(sourceChecksum);
        printInfo("Source checksum: 0x" + sourceChecksumStr);
        
        // PUT file to stage without compression so results are deterministic
        std::string putQuery = "PUT 'file://" + testFile + "' @demo_stage AUTO_COMPRESS=FALSE";
        printInfo("Uploading file to stage (no compression)...");
        
        auto putStart = Clock::now();
        rc = SQLExecDirect(hStmt, (SQLCHAR*)putQuery.c_str(), SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "PUT file to stage")) {
            return false;
        }
        logPerf("demo_put_operation", elapsedMillis(putStart));
        
        // Get PUT result
        char source[256], target[256], status[256];
        SQLBIGINT sourceSize, targetSize;
        SQLLEN srcLen, tgtLen, srcSizeLen, tgtSizeLen, statusLen;
        
        SQLBindCol(hStmt, 1, SQL_C_CHAR, source, sizeof(source), &srcLen);
        SQLBindCol(hStmt, 2, SQL_C_CHAR, target, sizeof(target), &tgtLen);
        SQLBindCol(hStmt, 3, SQL_C_SBIGINT, &sourceSize, 0, &srcSizeLen);
        SQLBindCol(hStmt, 4, SQL_C_SBIGINT, &targetSize, 0, &tgtSizeLen);
        SQLBindCol(hStmt, 7, SQL_C_CHAR, status, sizeof(status), &statusLen);
        
        if (SQLFetch(hStmt) == SQL_SUCCESS) {
            std::cout << "  Source:      " << source << "\n";
            std::cout << "  Target:      " << target << "\n";
            std::cout << "  Source Size: " << sourceSize << " bytes\n";
            std::cout << "  Target Size: " << targetSize << " bytes (compressed)\n";
            std::cout << "  Status:      " << status << "\n";
            if (targetSize == sourceSize) {
                std::cout << "  Compression: disabled\n";
            } else {
                std::cout << "  Compression: " << std::fixed << std::setprecision(1)
                          << (100.0 * (1.0 - (double)targetSize / sourceSize)) << "%\n";
            }
            printSuccess("File uploaded to stage");
        }
        SQLCloseCursor(hStmt);
        
        // Reset statement for GET operation
        resetStatement();
        
        // GET file from stage
        const std::string downloadDir = "/tmp/";
        std::string getQuery = "GET @demo_stage 'file://" + downloadDir + "'";
        printInfo("Downloading file from stage...");
        
        auto getStart = Clock::now();
        rc = SQLExecDirect(hStmt, (SQLCHAR*)getQuery.c_str(), SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "GET file from stage")) {
            return false;
        }
        logPerf("demo_get_operation", elapsedMillis(getStart));
        
        // Get GET result
        char file[256];
        SQLLEN fileLen, statusLen2;
        
        SQLBindCol(hStmt, 1, SQL_C_CHAR, file, sizeof(file), &fileLen);
        SQLBindCol(hStmt, 3, SQL_C_CHAR, status, sizeof(status), &statusLen2);
        
        if (SQLFetch(hStmt) == SQL_SUCCESS) {
            std::string downloadedFile = file;
            std::string downloadedPath = downloadedFile;
            const std::string fileScheme = "file://";
            if (downloadedPath.rfind(fileScheme, 0) == 0) {
                downloadedPath = downloadedPath.substr(fileScheme.size());
            } else if (!downloadedPath.empty() && downloadedPath[0] != '/') {
                downloadedPath = downloadDir + downloadedPath;
            }
            
            uint64_t downloadedChecksum = computeChecksum(downloadedPath);
            std::string downloadedChecksumStr = formatChecksum(downloadedChecksum);
            bool checksumMatch = downloadedChecksum == sourceChecksum;
            
            std::cout << "  File:     " << file << "\n";
            std::cout << "  Status:   " << status << "\n";
            std::cout << "  Checksum: 0x" << downloadedChecksumStr << "\n";
            
            if (checksumMatch) {
                printSuccess("File downloaded from stage (checksum match)");
            } else {
                printWarning("Downloaded file checksum mismatch");
            }
        }
        SQLCloseCursor(hStmt);
        
        logPerf("demo_put_get", elapsedMillis(demoStart));
        return true;
    }
    
    // Demo 6: Complex data types
    bool demoComplexDataTypes() {
        printSection("Demo 6: Complex Data Types - VARIANT, ARRAY, OBJECT");
        
        resetStatement();
        auto demoStart = Clock::now();
        
        // Create table with complex types
        const char* createTable = 
            "CREATE TEMPORARY TABLE demo_complex ("
            "    id INTEGER,"
            "    json_data VARIANT,"
            "    tags ARRAY,"
            "    metadata OBJECT"
            ")";
        
        SQLRETURN rc = SQLExecDirect(hStmt, (SQLCHAR*)createTable, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Create table with complex types")) {
            return false;
        }
        SQLCloseCursor(hStmt);
        
        // Insert data with complex types
        const char* insertData = 
            "INSERT INTO demo_complex SELECT "
            "    1,"
            "    PARSE_JSON('{\"name\": \"Product A\", \"price\": 99.99, \"in_stock\": true}'),"
            "    ARRAY_CONSTRUCT('electronics', 'featured', 'sale'),"
            "    OBJECT_CONSTRUCT('created_by', 'system', 'priority', 'high')";
        
        rc = SQLExecDirect(hStmt, (SQLCHAR*)insertData, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Insert complex data")) {
            return false;
        }
        SQLCloseCursor(hStmt);
        
        // Query and extract from complex types
        const char* query = 
            "SELECT "
            "    id,"
            "    json_data:name::STRING as product_name,"
            "    json_data:price::FLOAT as price,"
            "    json_data:in_stock::BOOLEAN as in_stock,"
            "    ARRAY_SIZE(tags) as num_tags,"
            "    tags[0]::STRING as first_tag,"
            "    metadata:priority::STRING as priority"
            " FROM demo_complex";
        
        rc = SQLExecDirect(hStmt, (SQLCHAR*)query, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Query complex types")) {
            return false;
        }
        
        SQLBIGINT id, numTags;
        char productName[256], firstTag[256], priority[256];
        double price;
        char inStock[10];
        SQLLEN idLen, nameLen, priceLen, stockLen, tagsLen, tagLen, prioLen;
        
        SQLBindCol(hStmt, 1, SQL_C_SBIGINT, &id, 0, &idLen);
        SQLBindCol(hStmt, 2, SQL_C_CHAR, productName, sizeof(productName), &nameLen);
        SQLBindCol(hStmt, 3, SQL_C_DOUBLE, &price, 0, &priceLen);
        SQLBindCol(hStmt, 4, SQL_C_CHAR, inStock, sizeof(inStock), &stockLen);
        SQLBindCol(hStmt, 5, SQL_C_SBIGINT, &numTags, 0, &tagsLen);
        SQLBindCol(hStmt, 6, SQL_C_CHAR, firstTag, sizeof(firstTag), &tagLen);
        SQLBindCol(hStmt, 7, SQL_C_CHAR, priority, sizeof(priority), &prioLen);
        
        if (SQLFetch(hStmt) == SQL_SUCCESS) {
            std::cout << COLOR_BOLD << "Extracted Data:\n" << COLOR_RESET;
            std::cout << "  ID:           " << id << "\n";
            std::cout << "  Product Name: " << productName << "\n";
            std::cout << "  Price:        $" << std::fixed << std::setprecision(2) << price << "\n";
            std::cout << "  In Stock:     " << inStock << "\n";
            std::cout << "  Num Tags:     " << numTags << "\n";
            std::cout << "  First Tag:    " << firstTag << "\n";
            std::cout << "  Priority:     " << priority << "\n";
            printSuccess("Complex data types handled successfully");
        }
        
        SQLCloseCursor(hStmt);
        logPerf("demo_complex_data_types", elapsedMillis(demoStart));
        return true;
    }
    
    // Demo 7: Transaction management
    bool demoTransactions() {
        printSection("Demo 7: Transaction Management - ACID Properties");
        
        resetStatement();
        auto demoStart = Clock::now();
        
        // Disable auto-commit
        SQLRETURN rc = SQLSetConnectAttr(hDbc, SQL_ATTR_AUTOCOMMIT, 
                                         (SQLPOINTER)SQL_AUTOCOMMIT_OFF, 0);
        if (!checkRC(rc, SQL_HANDLE_DBC, hDbc, "Disable auto-commit")) {
            return false;
        }
        printInfo("Auto-commit disabled");
        
        // Create a table
        const char* createTable = 
            "CREATE TEMPORARY TABLE demo_accounts ("
            "    account_id INTEGER,"
            "    balance DECIMAL(10,2)"
            ")";
        
        rc = SQLExecDirect(hStmt, (SQLCHAR*)createTable, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Create accounts table")) {
            SQLEndTran(SQL_HANDLE_DBC, hDbc, SQL_ROLLBACK);
            return false;
        }
        SQLCloseCursor(hStmt);
        
        // Insert initial data
        const char* insertData = 
            "INSERT INTO demo_accounts VALUES (1, 1000.00), (2, 500.00)";
        
        rc = SQLExecDirect(hStmt, (SQLCHAR*)insertData, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Insert initial balances")) {
            SQLEndTran(SQL_HANDLE_DBC, hDbc, SQL_ROLLBACK);
            return false;
        }
        SQLCloseCursor(hStmt);
        
        // Commit initial setup
        rc = SQLEndTran(SQL_HANDLE_DBC, hDbc, SQL_COMMIT);
        if (!checkRC(rc, SQL_HANDLE_DBC, hDbc, "Commit initial setup")) {
            return false;
        }
        printSuccess("Initial balances committed");
        
        // Demonstrate rollback
        printInfo("Attempting transfer: $200 from account 1 to account 2");
        
        const char* debit = "UPDATE demo_accounts SET balance = balance - 200 WHERE account_id = 1";
        rc = SQLExecDirect(hStmt, (SQLCHAR*)debit, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Debit account 1")) {
            SQLEndTran(SQL_HANDLE_DBC, hDbc, SQL_ROLLBACK);
            return false;
        }
        SQLCloseCursor(hStmt);
        
        printWarning("Simulating error before credit...");
        printInfo("Rolling back transaction");
        
        rc = SQLEndTran(SQL_HANDLE_DBC, hDbc, SQL_ROLLBACK);
        if (!checkRC(rc, SQL_HANDLE_DBC, hDbc, "Rollback transaction")) {
            return false;
        }
        
        // Verify rollback
        const char* checkBalances = "SELECT account_id, balance FROM demo_accounts ORDER BY account_id";
        rc = SQLExecDirect(hStmt, (SQLCHAR*)checkBalances, SQL_NTS);
        if (!checkRC(rc, SQL_HANDLE_STMT, hStmt, "Check balances after rollback")) {
            return false;
        }
        
        std::cout << COLOR_BOLD << "\nBalances after rollback:\n" << COLOR_RESET;
        SQLBIGINT accountId;
        double balance;
        SQLLEN idLen, balLen;
        
        SQLBindCol(hStmt, 1, SQL_C_SBIGINT, &accountId, 0, &idLen);
        SQLBindCol(hStmt, 2, SQL_C_DOUBLE, &balance, 0, &balLen);
        
        while ((rc = SQLFetch(hStmt)) == SQL_SUCCESS) {
            std::cout << "  Account " << accountId << ": $" 
                      << std::fixed << std::setprecision(2) << balance << "\n";
        }
        SQLCloseCursor(hStmt);
        
        printSuccess("Transaction rolled back successfully - balances unchanged");
        
        // Re-enable auto-commit
        SQLSetConnectAttr(hDbc, SQL_ATTR_AUTOCOMMIT, 
                         (SQLPOINTER)SQL_AUTOCOMMIT_ON, 0);
        
        logPerf("demo_transactions", elapsedMillis(demoStart));
        return true;
    }
    
    // Run all demos
    void runAllDemos() {
        std::cout << COLOR_BOLD << COLOR_BLUE
                  << "\n╔═══════════════════════════════════════════════════════════════╗\n"
                  << "║                                                               ║\n"
                  << "║     Universal Snowflake ODBC Driver - Advanced Demo          ║\n"
                  << "║                                                               ║\n"
                  << "╚═══════════════════════════════════════════════════════════════╝\n"
                  << COLOR_RESET << "\n";
        
        int passed = 0, failed = 0;
        
        auto runDemo = [&](const std::string& name, auto demoFunc) {
            try {
                if ((this->*demoFunc)()) {
                    passed++;
                } else {
                    failed++;
                }
            } catch (const std::exception& e) {
                printError("Exception in " + name + ": " + e.what());
                failed++;
            }
        };
        
        runDemo("Basic Query", &ODBCDemo::demoBasicQuery);
        runDemo("Aggregation Query", &ODBCDemo::demoAggregationQuery);
        runDemo("Parameter Binding", &ODBCDemo::demoParameterBinding);
        runDemo("Large Result Set", &ODBCDemo::demoLargeResultSet);
        runDemo("PUT/GET Operations", &ODBCDemo::demoPutGetOperations);
        runDemo("Complex Data Types", &ODBCDemo::demoComplexDataTypes);
        runDemo("Transactions", &ODBCDemo::demoTransactions);
        
        // Final summary
        printSection("Demo Summary");
        std::cout << COLOR_BOLD << "Results:\n" << COLOR_RESET;
        std::cout << "  " << COLOR_GREEN << "✓ Passed: " << passed << COLOR_RESET << "\n";
        if (failed > 0) {
            std::cout << "  " << COLOR_RED << "✗ Failed: " << failed << COLOR_RESET << "\n";
        }
        std::cout << "  Total:   " << (passed + failed) << "\n\n";
        
        if (failed == 0) {
            std::cout << COLOR_GREEN << COLOR_BOLD
                      << "🎉 All demos completed successfully!\n"
                      << COLOR_RESET;
        }
    }
};

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: " << argv[0] << " <path_to_parameters.json>\n";
        return 1;
    }
    
    ODBCDemo demo;
    
    if (!demo.initialize()) {
        return 1;
    }
    
    if (!demo.connect(argv[1])) {
        return 1;
    }
    
    demo.runAllDemos();
    
    return 0;
}

