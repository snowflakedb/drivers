#pragma once

#include <map>
#include <string>
#include <vector>

std::string get_env_required(const char* name);
int get_env_int(const char* name, int default_value);
std::string get_driver_type();
std::string get_driver_path();
std::map<std::string, std::string> parse_parameters_json();
std::vector<std::string> parse_setup_queries();
