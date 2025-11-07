#pragma once

#include <map>
#include <string>
#include <vector>

#include "test_types.h"

std::string get_env_required(const char* name);
std::string get_env_optional(const char* name, const std::string& default_value);
int get_env_int(const char* name, int default_value);
std::string get_driver_type();
std::string get_driver_path();
TestType get_test_type();
std::map<std::string, std::string> parse_parameters_json();
std::vector<std::string> parse_setup_queries();
