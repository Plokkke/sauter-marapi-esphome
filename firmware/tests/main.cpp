#include "check.h"

int main() {
  for (const TestCase &test : tests()) {
    const int before = failures();
    test.body();
    std::printf("%s %s\n", failures() == before ? "ok    " : "FAILED", test.name);
  }
  std::printf("%zu tests, %d failed checks\n", tests().size(), failures());
  return failures() == 0 ? 0 : 1;
}
