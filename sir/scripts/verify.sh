#!/usr/bin/env bash
# ABSAC evidence runner. Runs the differential matrix and prints hashes so
# a third party can reproduce the current claims.
#
#   scripts/verify.sh          # fast: tests, base-14 native, external native
#   scripts/verify.sh full     # adds AArch64, fuzz, sanitizers
set -uo pipefail

MODE="${1:-fast}"
SIR_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="$(cd "$SIR_DIR/.." && pwd)"
CORPUS="$ROOT/gate6a"
EXTERNAL="${ABSAC_EXTERNAL:-/tmp/external}"
BIN="$SIR_DIR/target/release/emit_c_diff"
export TMPDIR="${TMPDIR:-$ROOT/tmp}"
mkdir -p "$TMPDIR/verify"

fail=0

# All 19 held-out Embench benchmarks. Gating only four of them is how the
# sglib dllist miscompile of 2026-09-23 stayed invisible: the failing shape
# was exercised by a corpus that was not in the matrix. Each corpus also
# needs its own support object, so the flags travel with the name.
EXTERNAL_CORPORA="matmult-int crc32 edn statemate aha-mont64 depthconv huffbench
md5sum nettle-aes nettle-sha256 nsichneu picojpeg qrduino sglib-combined slre
tarfind ud wikisort xgboost"
external_missing=0
external_args=()

external_args_for() {
    local support
    external_args=()
    case "$1" in
        crc32|wikisort) support="$SIR_DIR/support/beebs_rng.c" ;;
        huffbench|md5sum|tarfind) support="$EXTERNAL/embench/support/beebsc.c" ;;
        *) support="" ;;
    esac
    [ -n "$support" ] && [ -f "$support" ] && external_args=(--ffi-support "$support")
    return 0
}

run_case() {
    local label="$1"
    shift
    local out
    out=$(timeout "${CASE_TIMEOUT:-180}" "$BIN" "$@" 2>&1)
    local status=$?
    if [ "$status" -eq 124 ]; then
        echo "FAIL $label timeout"
        fail=1
        return
    fi
    local summary
    summary=$(printf '%s\n' "$out" | grep -o 'EMITC_SUMMARY.*' | tail -1)
    if printf '%s\n' "$out" | grep -qE 'mismatched=[1-9]|harness_errors=[1-9]|violations=[1-9]'; then
        echo "FAIL $label"
        printf '%s\n' "$out" | tail -20
        fail=1
    else
        echo "PASS $label ${summary:-no-summary}"
    fi
}

echo "== build"
(cd "$SIR_DIR" && cargo build --release -p sir_benchmarks --bin emit_c_diff >/dev/null) || exit 1

echo "== workspace tests"
test_log="$TMPDIR/verify/tests.log"
(cd "$SIR_DIR" && cargo test --workspace) >"$test_log" 2>&1 || {
    echo "FAIL workspace tests"; tail -20 "$test_log"; exit 1;
}
read -r passed failed ignored < <(
    awk -F'[ ;]' '
        { for (i = 1; i <= NF; i++) {
            if ($i == "passed") p += $(i-1);
            if ($i == "failed") f += $(i-1);
            if ($i == "ignored") g += $(i-1);
        } }
        END { print p+0, f+0, g+0 }
    ' "$test_log"
)
if [ "$failed" -ne 0 ]; then
    echo "FAIL workspace tests passed=$passed failed=$failed"
    fail=1
else
    echo "PASS workspace tests passed=$passed failed=$failed ignored=$ignored"
fi

echo "== base-14 native"
for name in v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v16; do
    run_case "base14/$name" "$CORPUS/${name}_corpus.ll" \
        --cases 2 --workdir "$TMPDIR/verify/base14_$name"
done

echo "== external Embench native (all 19)"
for name in $EXTERNAL_CORPORA; do
    if [ ! -f "$EXTERNAL/$name.ll" ]; then
        echo "SKIP external/$name (no .ll — run scripts/fetch-external.sh)"
        external_missing=$((external_missing + 1))
        continue
    fi
    external_args_for "$name"
    run_case "external/$name" "$EXTERNAL/$name.ll" \
        --cases 3 --workdir "$TMPDIR/verify/external_$name" "${external_args[@]}"
done
if [ "$external_missing" -ne 0 ]; then
    echo "WARN $external_missing/19 external corpora absent — the gate is NOT complete"
fi

if [ "$MODE" = "full" ]; then
    echo "== base-14 AArch64"
    for name in v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v16; do
        run_case "base14-a64/$name" "$CORPUS/${name}_corpus.ll" \
            --cases 2 --target aarch64 --workdir "$TMPDIR/verify/base14_a64_$name"
    done
    echo "== fuzz native"
    for name in v17 v17b v19 v19b; do
        run_case "fuzz/$name" "$CORPUS/${name}_fuzz_corpus.ll" \
            --cases 8 --workdir "$TMPDIR/verify/fuzz_$name"
    done
    echo "== fuzz AArch64 (v17/v19)"
    for name in v17 v19; do
        run_case "fuzz-a64/$name" "$CORPUS/${name}_fuzz_corpus.ll" \
            --cases 8 --target aarch64 --workdir "$TMPDIR/verify/fuzz_a64_$name"
    done
    echo "== sanitizers (all 19)"
    for name in $EXTERNAL_CORPORA; do
        [ -f "$EXTERNAL/$name.ll" ] || continue
        external_args_for "$name"
        run_case "sanitize/$name" "$EXTERNAL/$name.ll" \
            --cases 3 --sanitize --workdir "$TMPDIR/verify/sanitize_$name" "${external_args[@]}"
    done
    echo "== external Embench AArch64 (all 19)"
    for name in $EXTERNAL_CORPORA; do
        [ -f "$EXTERNAL/$name.ll" ] || continue
        external_args_for "$name"
        run_case "external-a64/$name" "$EXTERNAL/$name.ll" \
            --cases 3 --target aarch64 --workdir "$TMPDIR/verify/external_a64_$name" "${external_args[@]}"
    done
fi

echo "== hashes"
manifest="$TMPDIR/verify/manifest.sha256"
{
    find "$SIR_DIR/crates" -name '*.rs' -print0 | sort -z | xargs -0 sha256sum
    find "$SIR_DIR/support" -type f -print0 | sort -z | xargs -0 sha256sum
    find "$CORPUS" -name '*.ll' -print0 | sort -z | xargs -0 sha256sum
    if [ -d "$EXTERNAL" ]; then
        find "$EXTERNAL" -maxdepth 1 -name '*.ll' -print0 | sort -z | xargs -0 sha256sum
    fi
} >"$manifest"
echo "MANIFEST $manifest $(wc -l <"$manifest") files"

if [ "$fail" -ne 0 ]; then
    echo "VERIFY FAIL"
    exit 1
fi
echo "VERIFY PASS mode=$MODE"
