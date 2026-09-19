#!/bin/bash
# bench.sh — compile and run the witness frontier benchmark
#
# Compiles witness_bench.c with both clang and gcc at -O3 -march=native,
# runs both, and saves results.

set -e

DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR"

OUT="$DIR/results"
mkdir -p "$OUT"

echo "=== Compiling with clang -O3 -march=native ==="
clang -O3 -march=native -std=c11 -D_POSIX_C_SOURCE=200809L \
    -mavx2 -msse4.2 -mpopcnt \
    -D_GNU_SOURCE \
    witness_bench.c -o "$OUT/witness_bench_clang" -lm -lpthread 2>&1

echo "=== Compiling with gcc -O3 -march=native ==="
gcc -O3 -march=native -std=c11 -D_POSIX_C_SOURCE=200809L \
    -mavx2 -msse4.2 -mpopcnt \
    -D_GNU_SOURCE \
    witness_bench.c -o "$OUT/witness_bench_gcc" -lm -lpthread 2>&1

echo "=== Running clang build ==="
"$OUT/witness_bench_clang" > "$OUT/results_clang.csv" 2>"$OUT/errors_clang.txt"
echo "Results: $OUT/results_clang.csv"

echo "=== Running gcc build ==="
"$OUT/witness_bench_gcc" > "$OUT/results_gcc.csv" 2>"$OUT/errors_gcc.txt"
echo "Results: $OUT/results_gcc.csv"

echo ""
echo "=== Correctness errors (clang) ==="
cat "$OUT/errors_clang.txt" 2>/dev/null || true
echo "=== Correctness errors (gcc) ==="
cat "$OUT/errors_gcc.txt" 2>/dev/null || true

echo ""
echo "=== Done ==="
