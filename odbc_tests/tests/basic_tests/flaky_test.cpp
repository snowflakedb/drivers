#include <random>

#include <catch2/catch_test_macros.hpp>

TEST_CASE("Flaky test example - intentionally unreliable", "[odbc][flaky]") {
  std::random_device rd;
  std::mt19937 gen(rd());
  std::bernoulli_distribution coin(0.5);
  REQUIRE(coin(gen));
}
