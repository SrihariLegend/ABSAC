#!/usr/bin/env bash
# Measure hllMerge: upstream (v0) against the patch (v6), by instruction count.
#
#   ./measure.sh [N]        N PFCOUNT commands per run, default 200
#
# Method, and why it is shaped this way:
#
#   * The signal is callgrind's instruction count, not wall clock. This box has
#     two cores and no `perf`; instruction counts under valgrind are
#     deterministic and unaffected by whatever else is running, so a difference
#     that appears here is real and a difference that does not is not
#     measurable by this method.
#
#   * A run at N=0 (start, load the RDB, shut down) is measured for each binary
#     and SUBTRACTED, so what is reported is the marginal cost of the N
#     PFCOUNTs, not the cost of loading a database.
#
#   * Both binaries load the SAME dump.rdb, written once by v0, and the run
#     verifies all four keys are densly encoded before it measures anything --
#     a sparse key would take the other branch and the measurement would say
#     nothing about hllMergeDense.
#
#   * PFCOUNT on a single key returns the cached cardinality in the header; it
#     is PFCOUNT over several keys that must merge. Hence four keys.
set -euo pipefail

N="${1:-200}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$HERE/bin"; DATA="$HERE/data"; PROF="$HERE/prof"
mkdir -p "$DATA" "$PROF"
PORT=7799
RDB="$DATA/dump.rdb"
# redis-cli is not installed system-wide; use the one this tree built.
CLI="$HERE/src/src/redis-cli"
[ -x "$CLI" ] || { echo "no redis-cli at $CLI -- build the tree first"; exit 2; }

have_valgrind() { command -v valgrind >/dev/null; }
have_valgrind || { echo "valgrind not installed; nothing to measure"; exit 2; }

# ---- the frozen dataset, written once ---------------------------------------
if [ ! -f "$RDB" ]; then
    echo "== building dataset with v0 (this happens once)"
    "$BIN/redis-server-v0" --port $PORT --daemonize yes --save '' --dir "$DATA" \
        --dbfilename dump.rdb --logfile "$DATA/build.log"
    sleep 1
    for k in k1 k2 k3 k4; do
        for i in $(seq 1 20000); do echo "PFADD $k $k-$i"; done | "$CLI" -p $PORT >/dev/null
    done
    echo "SAVE" | "$CLI" -p $PORT >/dev/null
    for k in k1 k2 k3 k4; do
        enc=$(echo "PFDEBUG ENCODING $k" | "$CLI" -p $PORT)
        [ "$enc" = "dense" ] || { echo "FATAL: $k is '$enc', not dense"; exit 2; }
    done
    echo "SHUTDOWN NOSAVE" | "$CLI" -p $PORT >/dev/null 2>&1 || true
    sleep 1
    echo "== dataset built, all four keys dense"
fi

drive() { # N
    local n="$1" i
    for ((i = 0; i < n; i++)); do echo "PFCOUNT k1 k2 k3 k4"; done
    echo "SHUTDOWN NOSAVE"
}

run_cg() { # binary outfile N
    valgrind --tool=callgrind --callgrind-out-file="$2" \
        "$1" --port $PORT --save '' --dir "$DATA" --dbfilename dump.rdb \
        --logfile "$DATA/run.log" >/dev/null 2>&1 &
    local vg=$!
    # wait for the server to accept commands, then drive it
    for _ in $(seq 1 100); do
        "$CLI" -p $PORT PING >/dev/null 2>&1 && break
        sleep 0.3
    done
    drive "$3" | "$CLI" -p $PORT >/dev/null 2>&1 || true
    wait $vg 2>/dev/null || true
}

total() { grep -m1 "^summary:" "$1" | awk '{print $2}'; }

echo "== N=$N PFCOUNT k1 k2 k3 k4 per run, callgrind instruction counts"
for v in v0 v6; do
    run_cg "$BIN/redis-server-$v" "$PROF/$v-n0.callgrind" 0
    run_cg "$BIN/redis-server-$v" "$PROF/$v-nN.callgrind" "$N"
    a=$(total "$PROF/$v-n0.callgrind"); b=$(total "$PROF/$v-nN.callgrind")
    printf '%s  startup=%s  with_commands=%s  marginal=%s\n' \
        "$v" "$a" "$b" "$(( b - a ))"
done

echo
awk -v n="$N" '
  /^v0/ { v0 = $NF } /^v6/ { v6 = $NF }
  END {
    printf "v0 marginal per run : %d instructions\n", v0;
    printf "v6 marginal per run : %d instructions\n", v6;
    printf "per command v0      : %.0f\n", v0 / n;
    printf "per command v6      : %.0f\n", v6 / n;
    printf "SPEEDUP the kernel  : %.3fx   (v0 / v6, instruction count)\n", v0 / v6;
  }' <(for v in v0 v6; do
        a=$(total "$PROF/$v-n0.callgrind"); b=$(total "$PROF/$v-nN.callgrind")
        echo "$v $(( b - a ))"
      done)
