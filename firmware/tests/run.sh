#!/bin/sh
# Builds and runs the tests of the pure logic (firmware/components/linap_mitm/*.h) on the Mac.
set -e
cd "$(dirname "$0")"
mkdir -p build
c++ -std=c++17 -Wall -Wextra -Werror -I ../components/linap_mitm -o build/tests ./*.cpp
build/tests
