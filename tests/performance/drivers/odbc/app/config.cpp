#include "config.h"

#include <cstdlib>
#include <iostream>
#include <regex>

std::string get_env_required(const char* name) {
  const char* value = std::getenv(name);
  if (!value) {
    std::cerr << "ERROR: Required environment variable " << name << " not set\n";
    exit(1);
  }
  return std::string(value);
}

int get_env_int(const char* name, int default_value) {
  const char* value = std::getenv(name);
  return value ? std::atoi(value) : default_value;
}

std::string get_driver_type() {
  const char* driver_type = std::getenv("DRIVER_TYPE");
  return driver_type ? driver_type : "universal";
}

std::string get_driver_path() {
  std::string driver_type_str = get_driver_type();

  if (driver_type_str == "old") {
    return "/usr/lib/snowflake/odbc/lib/libSnowflake.so";
  } else {
    return "/usr/lib/libsfodbc.so";
  }
}

std::map<std::string, std::string> parse_parameters_json() {
  std::map<std::string, std::string> params;

  const char* params_json = std::getenv("PARAMETERS_JSON");
  if (!params_json) {
    std::cerr << "ERROR: PARAMETERS_JSON environment variable not set\n";
    exit(1);
  }

  std::string json_str(params_json);

  std::string search_str = json_str;

  std::vector<std::pair<std::string, std::vector<std::string>>> key_mappings = {
      {"account", {"SNOWFLAKE_TEST_ACCOUNT", "account"}},
      {"host", {"SNOWFLAKE_TEST_HOST", "host"}},
      {"user", {"SNOWFLAKE_TEST_USER", "user"}},
      {"database", {"SNOWFLAKE_TEST_DATABASE", "database"}},
      {"schema", {"SNOWFLAKE_TEST_SCHEMA", "schema"}},
      {"warehouse", {"SNOWFLAKE_TEST_WAREHOUSE", "warehouse"}},
      {"role", {"SNOWFLAKE_TEST_ROLE", "role"}}};

  for (const auto& [param_name, json_keys] : key_mappings) {
    for (const auto& json_key : json_keys) {
      std::regex pattern("\"" + json_key + "\"\\s*:\\s*\"([^\"]*)\"");
      std::smatch match;
      if (std::regex_search(search_str, match, pattern)) {
        params[param_name] = match[1].str();
        break;
      }
    }
  }

  // Parse private key contents (array of strings)
  // Find the start of SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS array
  size_t key_start = json_str.find("\"SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS\"");
  if (key_start != std::string::npos) {
    size_t array_start = json_str.find("[", key_start);
    size_t array_end = json_str.find("]", array_start);

    if (array_start != std::string::npos && array_end != std::string::npos) {
      std::string key_array = json_str.substr(array_start + 1, array_end - array_start - 1);
      std::string private_key;

      // Extract all quoted strings from the array
      std::regex line_pattern(R"xxx("([^"]*)")xxx");
      auto lines_begin = std::sregex_iterator(key_array.begin(), key_array.end(), line_pattern);
      auto lines_end = std::sregex_iterator();

      for (auto it = lines_begin; it != lines_end; ++it) {
        if (!private_key.empty()) {
          private_key += "\n";
        }
        private_key += (*it)[1].str();
      }

      params["private_key"] = private_key;
    }
  }

  return params;
}

std::vector<std::string> parse_setup_queries() {
  std::vector<std::string> setup_queries;

  const char* setup_queries_json = std::getenv("SETUP_QUERIES");
  if (!setup_queries_json) {
    return setup_queries;  // No setup queries
  }

  std::string json_str(setup_queries_json);

  // Remove leading/trailing whitespace and brackets
  size_t start = json_str.find('[');
  size_t end = json_str.rfind(']');
  if (start == std::string::npos || end == std::string::npos) {
    return setup_queries;
  }

  std::string queries_str = json_str.substr(start + 1, end - start - 1);

  // Parse quoted strings
  size_t pos = 0;
  while (pos < queries_str.length()) {
    size_t quote_start = queries_str.find('"', pos);
    if (quote_start == std::string::npos) break;
    size_t quote_end = queries_str.find('"', quote_start + 1);
    if (quote_end == std::string::npos) break;
    std::string query = queries_str.substr(quote_start + 1, quote_end - quote_start - 1);
    if (!query.empty()) {
      setup_queries.push_back(query);
    }
    pos = quote_end + 1;
  }

  return setup_queries;
}
