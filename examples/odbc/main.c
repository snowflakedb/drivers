#include <stdio.h>
#include <string.h>

void select_1(const char* connection_string);
void put_file(const char* connection_string);
void get_file(const char* connection_string);

void print_usage(const char* program_name) {
  printf("Usage: %s <test_name> <connection_string>\n", program_name);
}

#define RUN_TEST(test_name)               \
  if (strcmp(argv[1], #test_name) == 0) { \
    test_name(argv[2]);                   \
    return 0;                             \
  }

int main(int argc, char* argv[]) {
  if (argc != 3) {
    print_usage(argv[0]);
    return 1;
  }
  RUN_TEST(select_1);
  RUN_TEST(put_file);
  RUN_TEST(get_file);
  print_usage(argv[0]);
  return 1;
}

#undef RUN_TEST
