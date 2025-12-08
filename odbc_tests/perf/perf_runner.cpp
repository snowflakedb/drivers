#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <map>
#include <numeric>
#include <optional>
#include <set>
#include <sstream>
#include <string>
#include <vector>

#include <sql.h>
#include <sqlext.h>

#include "picojson.h"

namespace {

using Clock = std::chrono::steady_clock;

struct Config {
    std::string account;
    std::string user;
    std::string password;
    std::string database;
    std::string schema;
    std::string warehouse;
    std::string role;
};

struct Options {
    std::string params_path;
    int iterations = 3;
    std::set<std::string> scenarios;
};

struct ScenarioSample {
    long long duration_ms;
    double aux_value = 0.0; // e.g., rows/sec
};

void logPerfValue(const std::string& metric, const std::string& key, double value) {
    std::ostringstream oss;
    oss.setf(std::ios::fixed);
    oss.precision(2);
    oss << value;
    std::cout << "PERF|" << metric << "|" << key << "=" << oss.str() << "\n";
}

void logPerfValue(const std::string& metric, const std::string& key, long long value) {
    std::cout << "PERF|" << metric << "|" << key << "=" << value << "\n";
}

double percentile(const std::vector<long long>& sorted, double pct) {
    if (sorted.empty()) {
        return 0.0;
    }
    double idx = (pct / 100.0) * (sorted.size() - 1);
    size_t lower = static_cast<size_t>(std::floor(idx));
    size_t upper = static_cast<size_t>(std::ceil(idx));
    double weight = idx - lower;
    if (upper >= sorted.size()) {
        return static_cast<double>(sorted.back());
    }
    return sorted[lower] + (sorted[upper] - sorted[lower]) * weight;
}

void summarizeSamples(const std::string& metric, const std::vector<long long>& samples) {
    if (samples.empty()) {
        return;
    }
    std::vector<long long> sorted = samples;
    std::sort(sorted.begin(), sorted.end());
    long long min_v = sorted.front();
    long long max_v = sorted.back();
    double avg = static_cast<double>(std::accumulate(sorted.begin(), sorted.end(), 0LL)) / sorted.size();
    double p50 = percentile(sorted, 50.0);
    double p95 = percentile(sorted, 95.0);
    double p99 = percentile(sorted, 99.0);

    logPerfValue(metric, "count", static_cast<long long>(sorted.size()));
    logPerfValue(metric, "min_ms", min_v);
    logPerfValue(metric, "max_ms", max_v);
    logPerfValue(metric, "avg_ms", avg);
    logPerfValue(metric, "p50_ms", p50);
    logPerfValue(metric, "p95_ms", p95);
    logPerfValue(metric, "p99_ms", p99);
}

void summarizeAux(const std::string& metric, const std::vector<double>& samples) {
    if (samples.empty()) {
        return;
    }
    std::vector<double> sorted = samples;
    std::sort(sorted.begin(), sorted.end());
    auto perc = [&](double pct) -> double {
        if (sorted.empty()) {
            return 0.0;
        }
        double idx = (pct / 100.0) * (sorted.size() - 1);
        size_t lower = static_cast<size_t>(std::floor(idx));
        size_t upper = static_cast<size_t>(std::ceil(idx));
        double weight = idx - lower;
        if (upper >= sorted.size()) {
            return sorted.back();
        }
        return sorted[lower] + (sorted[upper] - sorted[lower]) * weight;
    };
    double avg = std::accumulate(sorted.begin(), sorted.end(), 0.0) / sorted.size();
    logPerfValue(metric, "count", static_cast<long long>(sorted.size()));
    logPerfValue(metric, "min", sorted.front());
    logPerfValue(metric, "max", sorted.back());
    logPerfValue(metric, "avg", avg);
    logPerfValue(metric, "p50", perc(50.0));
    logPerfValue(metric, "p95", perc(95.0));
    logPerfValue(metric, "p99", perc(99.0));
}

bool readFile(const std::string& path, std::string& out) {
    std::ifstream in(path);
    if (!in.is_open()) {
        return false;
    }
    std::ostringstream buffer;
    buffer << in.rdbuf();
    out = buffer.str();
    return true;
}

bool loadParameters(const std::string& path, Config& cfg) {
    std::string data;
    if (!readFile(path, data)) {
        std::cerr << "Failed to read parameters file: " << path << "\n";
        return false;
    }
    picojson::value json;
    std::string err = picojson::parse(json, data);
    if (!err.empty()) {
        std::cerr << "Failed to parse parameters.json: " << err << "\n";
        return false;
    }
    picojson::object obj = json.get<picojson::object>();
    auto nested = obj.find("testconnection");
    if (nested != obj.end() && nested->second.is<picojson::object>()) {
        picojson::object nestedObj = nested->second.get<picojson::object>();
        obj = std::move(nestedObj);
    }
    auto getStr = [&](const std::string& key) -> std::string {
        auto it = obj.find(key);
        if (it == obj.end() || !it->second.is<std::string>()) {
            return "";
        }
        return it->second.get<std::string>();
    };
    cfg.account = getStr("SNOWFLAKE_TEST_ACCOUNT");
    cfg.user = getStr("SNOWFLAKE_TEST_USER");
    cfg.password = getStr("SNOWFLAKE_TEST_PASSWORD");
    cfg.database = getStr("SNOWFLAKE_TEST_DATABASE");
    cfg.schema = getStr("SNOWFLAKE_TEST_SCHEMA");
    cfg.warehouse = getStr("SNOWFLAKE_TEST_WAREHOUSE");
    cfg.role = getStr("SNOWFLAKE_TEST_ROLE");

    if (cfg.account.empty() || cfg.user.empty() || cfg.password.empty()) {
        std::cerr << "Missing required Snowflake parameters in " << path << "\n";
        return false;
    }
    return true;
}

std::string buildConnectionString(const Config& cfg, const std::string& driverPath) {
    std::ostringstream conn;
    conn << "DRIVER=" << driverPath << ";"
         << "SERVER=" << cfg.account << ".snowflakecomputing.com;"
         << "ACCOUNT=" << cfg.account << ";"
         << "UID=" << cfg.user << ";"
         << "PWD=" << cfg.password << ";";
    if (!cfg.database.empty()) conn << "DATABASE=" << cfg.database << ";";
    if (!cfg.schema.empty()) conn << "SCHEMA=" << cfg.schema << ";";
    if (!cfg.warehouse.empty()) conn << "WAREHOUSE=" << cfg.warehouse << ";";
    if (!cfg.role.empty()) conn << "ROLE=" << cfg.role << ";";
    return conn.str();
}

std::string diagInfo(SQLSMALLINT handleType, SQLHANDLE handle) {
    SQLCHAR sqlState[6];
    SQLCHAR message[SQL_MAX_MESSAGE_LENGTH];
    SQLINTEGER nativeError = 0;
    SQLSMALLINT textLength = 0;
    std::ostringstream oss;
    SQLSMALLINT index = 1;
    while (SQLGetDiagRec(handleType, handle, index, sqlState, &nativeError, message,
                         sizeof(message), &textLength) == SQL_SUCCESS) {
        oss << "[" << sqlState << "] (" << nativeError << ") "
            << reinterpret_cast<char*>(message) << "\n";
        index++;
    }
    return oss.str();
}

bool checkRC(SQLRETURN rc, SQLSMALLINT type, SQLHANDLE handle, const std::string& op) {
    if (rc == SQL_SUCCESS || rc == SQL_SUCCESS_WITH_INFO) {
        if (rc == SQL_SUCCESS_WITH_INFO) {
            std::cerr << "INFO: " << op << " returned SQL_SUCCESS_WITH_INFO\n"
                      << diagInfo(type, handle);
        }
        return true;
    }
    std::cerr << "ODBC error during " << op << ":\n" << diagInfo(type, handle);
    return false;
}

class PerfRunner {
public:
    explicit PerfRunner(const Config& cfg, const std::string& driverPath)
        : cfg_(cfg), driverPath_(driverPath) {}

    ~PerfRunner() { disconnect(); cleanEnv(); }

    bool initialize() {
        SQLRETURN rc = SQLAllocHandle(SQL_HANDLE_ENV, SQL_NULL_HANDLE, &hEnv_);
        if (!checkRC(rc, SQL_HANDLE_ENV, hEnv_, "SQLAllocHandle ENV")) {
            return false;
        }
        rc = SQLSetEnvAttr(hEnv_, SQL_ATTR_ODBC_VERSION, (SQLPOINTER)SQL_OV_ODBC3, 0);
        return checkRC(rc, SQL_HANDLE_ENV, hEnv_, "SQLSetEnvAttr ODBC3");
    }

    bool connectPersistent() {
        if (connected_) {
            return true;
        }
        std::string connStr = buildConnectionString(cfg_, driverPath_);
        SQLRETURN rc = SQLAllocHandle(SQL_HANDLE_DBC, hEnv_, &hDbc_);
        if (!checkRC(rc, SQL_HANDLE_ENV, hEnv_, "SQLAllocHandle DBC")) {
            return false;
        }
        SQLCHAR outConn[1024];
        SQLSMALLINT outLen = 0;
        rc = SQLDriverConnect(hDbc_, nullptr,
                              (SQLCHAR*)connStr.c_str(), SQL_NTS,
                              outConn, sizeof(outConn), &outLen,
                              SQL_DRIVER_NOPROMPT);
        if (!checkRC(rc, SQL_HANDLE_DBC, hDbc_, "SQLDriverConnect")) {
            return false;
        }
        connected_ = true;
        return true;
    }

    bool disconnect() {
        if (hStmt_) {
            SQLFreeHandle(SQL_HANDLE_STMT, hStmt_);
            hStmt_ = nullptr;
        }
        if (connected_) {
            SQLDisconnect(hDbc_);
            connected_ = false;
        }
        if (hDbc_) {
            SQLFreeHandle(SQL_HANDLE_DBC, hDbc_);
            hDbc_ = nullptr;
        }
        return true;
    }

    bool connectOnceForMeasurement(long long& duration_ms) {
        SQLHDBC tempDbc = nullptr;
        SQLRETURN rc = SQLAllocHandle(SQL_HANDLE_DBC, hEnv_, &tempDbc);
        if (!checkRC(rc, SQL_HANDLE_ENV, hEnv_, "SQLAllocHandle DBC (temp)")) {
            return false;
        }
        std::string conn = buildConnectionString(cfg_, driverPath_);
        auto start = Clock::now();
        SQLCHAR outConn[1024];
        SQLSMALLINT outLen = 0;
        rc = SQLDriverConnect(tempDbc, nullptr,
                              (SQLCHAR*)conn.c_str(), SQL_NTS,
                              outConn, sizeof(outConn), &outLen,
                              SQL_DRIVER_NOPROMPT);
        if (!checkRC(rc, SQL_HANDLE_DBC, tempDbc, "SQLDriverConnect (temp)")) {
            SQLFreeHandle(SQL_HANDLE_DBC, tempDbc);
            return false;
        }
        SQLDisconnect(tempDbc);
        SQLFreeHandle(SQL_HANDLE_DBC, tempDbc);
        duration_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                          Clock::now() - start)
                          .count();
        return true;
    }

    SQLHSTMT resetStatement() {
        if (hStmt_) {
            SQLFreeHandle(SQL_HANDLE_STMT, hStmt_);
            hStmt_ = nullptr;
        }
        SQLRETURN rc = SQLAllocHandle(SQL_HANDLE_STMT, hDbc_, &hStmt_);
        if (!checkRC(rc, SQL_HANDLE_DBC, hDbc_, "SQLAllocHandle STMT")) {
            return nullptr;
        }
        return hStmt_;
    }

    SQLHSTMT stmt() const { return hStmt_; }
    SQLHDBC connection() const { return hDbc_; }

private:
    void cleanEnv() {
        if (hEnv_) {
            SQLFreeHandle(SQL_HANDLE_ENV, hEnv_);
            hEnv_ = nullptr;
        }
    }

    Config cfg_;
    std::string driverPath_;
    SQLHENV hEnv_ = nullptr;
    SQLHDBC hDbc_ = nullptr;
    SQLHSTMT hStmt_ = nullptr;
    bool connected_ = false;
};

bool executeSimpleQuery(SQLHSTMT stmt, const std::string& query) {
    SQLRETURN rc = SQLExecDirect(stmt, (SQLCHAR*)query.c_str(), SQL_NTS);
    if (!checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLExecDirect")) {
        return false;
    }
    SQLSMALLINT numCols = 0;
    rc = SQLNumResultCols(stmt, &numCols);
    if (!checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLNumResultCols")) {
        SQLCloseCursor(stmt);
        return false;
    }
    if (numCols > 0) {
        while ((rc = SQLFetch(stmt)) == SQL_SUCCESS) {
            // consume rows
        }
        if (rc != SQL_NO_DATA) {
            checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLFetch");
            SQLCloseCursor(stmt);
            return false;
        }
        SQLCloseCursor(stmt);
    }
    return true;
}

bool scenarioSimpleQuery(PerfRunner& runner) {
    SQLHSTMT stmt = runner.resetStatement();
    if (!stmt) return false;
    return executeSimpleQuery(stmt,
        "SELECT CURRENT_USER(), CURRENT_ROLE(), CURRENT_DATABASE(), CURRENT_SCHEMA()");
}

bool scenarioAggregation(PerfRunner& runner) {
    SQLHSTMT stmt = runner.resetStatement();
    if (!stmt) return false;

    const char* createTable =
        "CREATE OR REPLACE TEMPORARY TABLE demo_perf_sales ("
        " sale_id INTEGER, product VARCHAR(50), category VARCHAR(50),"
        " quantity INTEGER, price DECIMAL(10,2), sale_date DATE)";
    if (!executeSimpleQuery(stmt, createTable)) return false;

    const char* insertData =
        "INSERT INTO demo_perf_sales VALUES "
        "(1,'Laptop','Electronics',5,1200.00,'2024-01-15'),"
        "(2,'Mouse','Electronics',50,25.50,'2024-01-16'),"
        "(3,'Desk','Furniture',10,450.00,'2024-01-17'),"
        "(4,'Chair','Furniture',20,200.00,'2024-01-18'),"
        "(5,'Monitor','Electronics',15,350.00,'2024-01-19'),"
        "(6,'Keyboard','Electronics',30,75.00,'2024-01-20'),"
        "(7,'Bookshelf','Furniture',8,180.00,'2024-01-21')";
    if (!executeSimpleQuery(stmt, insertData)) return false;

    const char* aggQuery =
        "SELECT category, COUNT(*), SUM(quantity), AVG(price), SUM(quantity*price)"
        " FROM demo_perf_sales GROUP BY category";
    if (!executeSimpleQuery(stmt, aggQuery)) return false;

    return true;
}

bool scenarioParameterized(PerfRunner& runner) {
    SQLHSTMT stmt = runner.resetStatement();
    if (!stmt) return false;

    const char* query =
        "SELECT product, quantity, price, (quantity * price) as total "
        "FROM demo_perf_sales WHERE category = ? AND price > ?";

    SQLRETURN rc = SQLPrepare(stmt, (SQLCHAR*)query, SQL_NTS);
    if (!checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLPrepare")) return false;

    char category[] = "Electronics";
    double minPrice = 50.0;
    SQLLEN catLen = SQL_NTS;
    SQLLEN minPriceLen = 0;

    rc = SQLBindParameter(stmt, 1, SQL_PARAM_INPUT, SQL_C_CHAR, SQL_VARCHAR,
                          sizeof(category), 0, category, sizeof(category), &catLen);
    if (!checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLBindParameter category")) return false;

    rc = SQLBindParameter(stmt, 2, SQL_PARAM_INPUT, SQL_C_DOUBLE, SQL_DOUBLE,
                          0, 0, &minPrice, 0, &minPriceLen);
    if (!checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLBindParameter minPrice")) return false;

    rc = SQLExecute(stmt);
    if (!checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLExecute")) return false;

    while ((rc = SQLFetch(stmt)) == SQL_SUCCESS) {
        // consume rows
    }
    if (rc != SQL_NO_DATA) {
        checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLFetch");
        return false;
    }
    SQLCloseCursor(stmt);
    return true;
}

bool scenarioLargeResultSet(PerfRunner& runner, double& rowsPerSecOut) {
    SQLHSTMT stmt = runner.resetStatement();
    if (!stmt) return false;

    const int TOTAL_ROWS = 50000;
    std::ostringstream query;
    query << "SELECT SEQ8() AS id, MOD(SEQ8()*73,1000)+1 AS value "
          << "FROM TABLE(GENERATOR(ROWCOUNT=>" << TOTAL_ROWS << "))";

    SQLRETURN rc = SQLExecDirect(stmt, (SQLCHAR*)query.str().c_str(), SQL_NTS);
    if (!checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLExecDirect large result")) return false;

    auto fetchStart = Clock::now();
    SQLBIGINT id = 0;
    SQLLEN idLen = 0;
    SQLBindCol(stmt, 1, SQL_C_SBIGINT, &id, 0, &idLen);

    int64_t rowCount = 0;
    while ((rc = SQLFetch(stmt)) == SQL_SUCCESS) {
        rowCount++;
    }
    if (rc != SQL_NO_DATA) {
        checkRC(rc, SQL_HANDLE_STMT, stmt, "SQLFetch large result");
        return false;
    }

    auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
        Clock::now() - fetchStart);
    if (elapsed.count() > 0) {
        rowsPerSecOut = static_cast<double>(rowCount) * 1000.0 / elapsed.count();
    } else {
        rowsPerSecOut = 0.0;
    }
    SQLCloseCursor(stmt);
    return true;
}

bool scenarioPutGet(PerfRunner& runner) {
    SQLHSTMT stmt = runner.resetStatement();
    if (!stmt) return false;

    const char* createStage = "CREATE OR REPLACE TEMPORARY STAGE perf_stage";
    if (!executeSimpleQuery(stmt, createStage)) return false;

    const std::string testFile = "/tmp/perf_demo_data.csv";
    {
        std::ofstream out(testFile);
        if (!out.is_open()) {
            std::cerr << "Failed to create test file " << testFile << "\n";
            return false;
        }
        out << "id,name,value\n";
        for (int i = 1; i <= 100; ++i) {
            out << i << ",Item" << i << "," << (i * 10) << "\n";
        }
    }

    std::string putQuery = "PUT 'file://" + testFile + "' @perf_stage AUTO_COMPRESS=FALSE";
    if (!executeSimpleQuery(stmt, putQuery)) return false;

    SQLCloseCursor(stmt);
    runner.resetStatement();
    stmt = runner.stmt();
    std::string getQuery = "GET @perf_stage 'file:///tmp'";
    if (!executeSimpleQuery(stmt, getQuery)) return false;

    std::filesystem::remove(testFile);
    std::filesystem::remove("/tmp/perf_demo_data.csv.gz");
    return true;
}

bool scenarioTransactions(PerfRunner& runner) {
    SQLHSTMT stmt = runner.resetStatement();
    if (!stmt) return false;

    SQLHDBC dbc = runner.connection();

    SQLRETURN rc = SQLSetConnectAttr(dbc, SQL_ATTR_AUTOCOMMIT,
                                     (SQLPOINTER)SQL_AUTOCOMMIT_OFF, 0);
    if (!checkRC(rc, SQL_HANDLE_DBC, dbc, "SQLSetConnectAttr AUTOCOMMIT OFF")) {
        return false;
    }

    const char* createTable =
        "CREATE OR REPLACE TEMPORARY TABLE perf_accounts (account_id INTEGER, balance DECIMAL(10,2))";
    if (!executeSimpleQuery(stmt, createTable)) return false;

    const char* insertData =
        "INSERT INTO perf_accounts VALUES (1,1000.00),(2,500.00)";
    if (!executeSimpleQuery(stmt, insertData)) return false;

    rc = SQLEndTran(SQL_HANDLE_DBC, dbc, SQL_COMMIT);
    if (!checkRC(rc, SQL_HANDLE_DBC, dbc, "SQLEndTran COMMIT")) return false;

    const char* debit = "UPDATE perf_accounts SET balance = balance - 200 WHERE account_id = 1";
    if (!executeSimpleQuery(stmt, debit)) return false;

    rc = SQLEndTran(SQL_HANDLE_DBC, dbc, SQL_ROLLBACK);
    if (!checkRC(rc, SQL_HANDLE_DBC, dbc, "SQLEndTran ROLLBACK")) return false;

    const char* checkQuery = "SELECT account_id, balance FROM perf_accounts";
    if (!executeSimpleQuery(stmt, checkQuery)) return false;

    SQLSetConnectAttr(dbc, SQL_ATTR_AUTOCOMMIT, (SQLPOINTER)SQL_AUTOCOMMIT_ON, 0);
    return true;
}

Options parseArgs(int argc, char** argv) {
    Options opts;
    if (argc < 2) {
        return opts;
    }
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if ((arg == "--params" || arg == "-p") && i + 1 < argc) {
            opts.params_path = argv[++i];
        } else if ((arg == "--iterations" || arg == "-n") && i + 1 < argc) {
            opts.iterations = std::max(1, std::atoi(argv[++i]));
        } else if ((arg == "--scenarios" || arg == "-s") && i + 1 < argc) {
            std::string list = argv[++i];
            std::istringstream ss(list);
            std::string item;
            while (std::getline(ss, item, ',')) {
                if (!item.empty()) {
                    opts.scenarios.insert(item);
                }
            }
        }
    }
    if (opts.scenarios.empty()) {
        opts.scenarios = {"connect", "simple_query", "aggregation", "parameterized",
                          "large_fetch", "put_get", "transactions"};
    }
    return opts;
}

} // namespace

int main(int argc, char** argv) {
    Options opts = parseArgs(argc, argv);
    if (opts.params_path.empty()) {
        std::cerr << "Usage: perf_runner --params <parameters.json> [--iterations N] [--scenarios list]\n";
        return 1;
    }
    std::cerr << "[perf_runner] Parameters path: " << opts.params_path
              << " iterations=" << opts.iterations << std::endl;
    const char* driverPath = std::getenv("DRIVER_PATH");
    if (!driverPath) {
        std::cerr << "DRIVER_PATH environment variable is required\n";
        return 1;
    }
    Config cfg;
    if (!loadParameters(opts.params_path, cfg)) {
        return 1;
    }

    PerfRunner runner(cfg, driverPath);
    std::cerr << "[perf_runner] Initializing ODBC environment\n";
    if (!runner.initialize()) {
        return 1;
    }

    std::vector<std::pair<std::string, std::vector<long long>>> scenarioSamples;
    std::vector<std::pair<std::string, std::vector<double>>> scenarioAux;

    auto recordScenario = [&](const std::string& name,
                              std::function<bool(double&)> scenarioFunc,
                              bool trackAux) -> bool {
        std::vector<long long> durations;
        std::vector<double> auxValues;
        for (int i = 0; i < opts.iterations; ++i) {
            std::cerr << "[perf_runner] Scenario " << name << " iteration " << (i + 1)
                      << "/" << opts.iterations << std::endl;
            if (!runner.connectPersistent()) return false;
            auto start = Clock::now();
            double aux = 0.0;
            if (!scenarioFunc(aux)) {
                std::cerr << "[perf_runner] Scenario " << name << " failed\n";
                return false;
            }
            long long elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
                Clock::now() - start).count();
            durations.push_back(elapsed);
            if (trackAux) {
                auxValues.push_back(aux);
            }
        }
        scenarioSamples.emplace_back(name, durations);
        if (trackAux) {
            scenarioAux.emplace_back(name + "_aux", auxValues);
        }
        return true;
    };

    if (opts.scenarios.count("connect")) {
        std::vector<long long> durations;
        for (int i = 0; i < opts.iterations; ++i) {
            std::cerr << "[perf_runner] Scenario perf_connect iteration " << (i + 1)
                      << "/" << opts.iterations << std::endl;
            long long duration = 0;
            if (!runner.connectOnceForMeasurement(duration)) {
                return 1;
            }
            durations.push_back(duration);
        }
        scenarioSamples.emplace_back("perf_connect", durations);
    }

    if (opts.scenarios.count("simple_query")) {
        if (!recordScenario("perf_simple_query",
                [&](double&) { return scenarioSimpleQuery(runner); }, false)) {
            return 1;
        }
    }

    if (opts.scenarios.count("aggregation")) {
        if (!recordScenario("perf_aggregation",
                [&](double&) { return scenarioAggregation(runner); }, false)) {
            return 1;
        }
    }

    if (opts.scenarios.count("parameterized")) {
        if (!recordScenario("perf_parameterized",
                [&](double&) { return scenarioParameterized(runner); }, false)) {
            return 1;
        }
    }

    if (opts.scenarios.count("large_fetch")) {
        if (!recordScenario("perf_large_fetch",
                [&](double& aux) { return scenarioLargeResultSet(runner, aux); }, true)) {
            return 1;
        }
    }

    if (opts.scenarios.count("put_get")) {
        if (!recordScenario("perf_put_get",
                [&](double&) { return scenarioPutGet(runner); }, false)) {
            return 1;
        }
    }

    if (opts.scenarios.count("transactions")) {
        if (!recordScenario("perf_transactions",
                [&](double&) { return scenarioTransactions(runner); }, false)) {
            return 1;
        }
    }

    for (const auto& [name, samples] : scenarioSamples) {
        summarizeSamples(name, samples);
    }
    for (const auto& [name, samples] : scenarioAux) {
        summarizeAux(name, samples);
    }

    return 0;
}

