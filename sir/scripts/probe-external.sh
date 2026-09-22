#!/usr/bin/env bash
# Run the held-out Embench probe set with the support objects each corpus
# needs, and print clean/refused/mismatch totals.
#
# Usage: sir/scripts/probe-external.sh [cases] [external-dir]
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CASES="${1:-2}"
EXTERNAL="${2:-/tmp/external}"
BIN="$ROOT/target/release/emit_c_diff"

if [ ! -x "$BIN" ]; then
    (cd "$ROOT" && cargo build --release -p sir_benchmarks --bin emit_c_diff)
fi

if [ ! -f "$EXTERNAL/wikisort.ll" ]; then
    "$ROOT/scripts/fetch-external.sh" "$EXTERNAL"
fi

corpora=(
    matmult-int crc32 edn statemate
    aha-mont64 depthconv huffbench md5sum nettle-aes nettle-sha256
    nsichneu picojpeg qrduino sglib-combined slre tarfind ud wikisort xgboost
)

clean=0
refused=0
errors=0
mismatched=0
for name in "${corpora[@]}"; do
    support=()
    case "$name" in
        crc32|wikisort) support=(--ffi-support "$ROOT/support/beebs_rng.c") ;;
        huffbench|md5sum|tarfind)
            support=(--ffi-support "$EXTERNAL/embench/support/beebsc.c")
            ;;
    esac
    out=$(TMPDIR="${TMPDIR:-/tmp}" "$BIN" "$EXTERNAL/$name.ll" \
        --cases "$CASES" "${support[@]}" --workdir "$EXTERNAL/probe/$name" 2>&1) || true
    line=$(printf '%s\n' "$out" | grep 'EMITC_SUMMARY' | tail -1)
    printf '%-16s %s\n' "$name" "$line"
    value() { printf '%s\n' "$line" | sed -n "s/.*$1=\\([0-9]*\\).*/\\1/p"; }
    clean=$((clean + $(value clean)))
    refused=$((refused + $(value lower_refused)))
    errors=$((errors + $(value harness_errors)))
    mismatched=$((mismatched + $(value mismatched)))
done
printf 'TOTAL clean=%d refused=%d harness_errors=%d mismatched=%d\n' \
    "$clean" "$refused" "$errors" "$mismatched"
[ "$mismatched" -eq 0 ]
