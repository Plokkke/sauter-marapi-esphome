#pragma once
// A minimal test runner: TEST(name) { CHECK(condition); }, run by firmware/tests/run.sh.
#include <cstdio>
#include <functional>
#include <vector>

struct TestCase {
  const char *name;
  std::function<void()> body;
};

inline std::vector<TestCase> &tests() {
  static std::vector<TestCase> all;
  return all;
}
inline int &failures() {
  static int count = 0;
  return count;
}

struct Register {
  Register(const char *name, std::function<void()> body) { tests().push_back({name, std::move(body)}); }
};

#define TEST(name)                                  \
  static void name();                               \
  static const Register register_##name(#name, name); \
  static void name()

#define CHECK(condition)                                                        \
  do {                                                                          \
    if (!(condition)) {                                                         \
      std::printf("  FAILED %s:%d: %s\n", __FILE__, __LINE__, #condition);      \
      failures()++;                                                             \
    }                                                                           \
  } while (0)
