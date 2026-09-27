#!/usr/bin/env bash
# Opportunity audit — workload driver and noise floor.
#
#   scripts/audit-run.sh noise [reps]        # baseline vs baseline: the noise floor
#   scripts/audit-run.sh run <APP> [reps]    # one application's workload
#
# The noise floor is measured before any result is interpreted. This is a 2-core
# WSL2 box with no `perf`, and the spread between two runs of the *same* binary
# decides whether a 5% end-to-end claim can be evaluated here at all
# (AUDIT_PREREGISTRATION.md, "Methodology").
#
# Each case below is the frozen workload for that application. Changing one is a
# dated correction to the pre-registration, not an edit.
set -uo pipefail

MODE="${1:-noise}"; shift || true
A=/tmp/audit; B="$A/build"; W="$A/work"
mkdir -p "$A/out"

# Program stdout goes to a *regular file*, never to /dev/null. GNU grep checks
# `stat("/dev/null")` at startup and sets `dev_null_output`, after which it
# behaves as if `-q` were given (grep.c:2896, `exit_on_match | dev_null_output`)
# and exits at the first match. Redirecting the scan workload to /dev/null
# silently turned a 69 MB scan into a first-match early exit — 0.075 s became
# 0.002 s and the instruction count fell 82x. A harness that redirects to
# /dev/null is not running the workload it says it is.
STDOUT_SINK="$A/out/current.stdout"

# run_one APP   — the frozen workload command, stdout discarded
run_one() {
    case "$1" in
        A1-grep)   "$B/A1-grep" -F -c "fn " "$W/corpus.txt" ;;
        A2-zstd)   "$B/A2-zstd" -3 -c "$W/corpus.txt" ;;
        A3-jq)     "$B/A3-jq" '[.[] | select(.v > 50000) | {k, t}] | length' "$W/records.json" ;;
        A4-sqlite) "$B/A4-sqlite3" :memory: < "$W/bench.sql" ;;
        A5-pngtest) "$B/A5-pngtest" "$W/image.png" ;;
        A6-lua)    "$B/A6-lua" "$W/bench.lua" ;;
        *) echo "unknown app $1" >&2; return 2 ;;
    esac
}

APPS="A1-grep A2-zstd A3-jq A4-sqlite A5-pngtest A6-lua"

measure() { # label reps
    local label="$1" reps="$2" i t0 t1
    local -a t=()
    for ((i = 0; i < reps; i++)); do
        t0=$(date +%s%N)
        run_one "$label" >"$STDOUT_SINK" 2>/dev/null
        t1=$(date +%s%N)
        t+=($(( (t1 - t0) / 1000 )))
    done
    printf '%s\n' "${t[@]}" | sort -n | awk -v l="$label" '
        { a[NR] = $1 }
        END {
            med = (NR % 2) ? a[(NR+1)/2] : (a[NR/2] + a[NR/2+1]) / 2
            printf "%-12s min=%9.2f median=%9.2f max=%9.2f spread=%6.2f%% n=%d\n",
                   l, a[1]/1000, med/1000, a[NR]/1000,
                   (a[NR] - a[1]) * 100.0 / med, NR
        }'
}

case "$MODE" in
    cg)
        # Profile one application under callgrind. This machine has no `perf`,
        # and callgrind's instruction counts are deterministic, so they are the
        # screening signal — never the performance claim (see addendum 1 and W3).
        APP="${1:?usage: audit-run.sh cg <APP>}"
        mkdir -p "$A/prof"
        out="$A/prof/$APP.callgrind"
        case "$APP" in
            A1-grep)   valgrind --tool=callgrind --callgrind-out-file="$out" \
                          "$B/A1-grep" -F -c "fn " "$W/corpus.txt" ;;
            A2-zstd)   valgrind --tool=callgrind --callgrind-out-file="$out" \
                          "$B/A2-zstd" -3 -c "$W/corpus.txt" ;;
            A3-jq)     valgrind --tool=callgrind --callgrind-out-file="$out" \
                          "$B/A3-jq" '[.[] | select(.v > 50000) | {k, t}] | length' "$W/records.json" ;;
            A4-sqlite) valgrind --tool=callgrind --callgrind-out-file="$out" \
                          "$B/A4-sqlite3" :memory: < "$W/bench.sql" ;;
            A5-pngtest) valgrind --tool=callgrind --callgrind-out-file="$out" \
                          "$B/A5-pngtest" "$W/image.png" ;;
            A6-lua)    valgrind --tool=callgrind --callgrind-out-file="$out" \
                          "$B/A6-lua" "$W/bench.lua" ;;
            *) echo "unknown app $APP" >&2; exit 2 ;;
        esac >"$STDOUT_SINK" 2>/dev/null
        callgrind_annotate --threshold=90 "$out" 2>/dev/null | tail -n +20
        ;;
    noise)
        REPS="${1:-5}"
        echo "== noise floor: the SAME binary run $REPS times. The spread is the"
        echo "== floor any claimed improvement has to clear. Nothing else may run."
        for app in $APPS; do measure "$app" "$REPS"; done
        ;;
    run)
        APP="${1:?usage: audit-run.sh run <APP> [reps]}"; REPS="${2:-5}"
        measure "$APP" "$REPS"
        ;;
    *) echo "usage: audit-run.sh [noise|run <APP>] [reps]" >&2; exit 2 ;;
esac
