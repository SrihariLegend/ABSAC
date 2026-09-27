#!/usr/bin/env bash
# Opportunity audit — select the four sites for one application.
#
#   scripts/audit-sample.sh <callgrind-out-file> <binary-path>
#
# Implements the frozen rule (AUDIT_PREREGISTRATION.md, "Site sampling"):
#   two hottest non-overlapping regions, plus two sampled from the remaining
#   hot regions with probability proportional to self cost, seed 0xA0D17.
#
# Self cost, so caller and callee do not overlap: a function's own instructions
# are counted once, in the function that executed them. Only functions in the
# application's own binary are candidates — libc and the dynamic loader are not
# ours to transform. Sampling weights are printed for every drawn site, as the
# pre-registration requires.
set -uo pipefail

CG="${1:?usage: audit-sample.sh <callgrind-file> <binary>}"
BIN="${2:?usage: audit-sample.sh <callgrind-file> <binary>}"
SEED=0xA0D17

callgrind_annotate --threshold=99.99 "$CG" 2>/dev/null |
    awk -v bin="$BIN" -v seed="$SEED" '
    # `n` and `m` must be initialised: an unset awk variable is "" and not 0,
    # so `c[n]` on the first record stores under key "" while the numeric sum
    # below reads c[0] — which silently drops the largest entry (the hot one).
    BEGIN { n = 0; m = 0 }
    index($0, "[" bin "]") {
        cost = $1; gsub(/,/, "", cost)
        # Take the name from the end of the line rather than from a field
        # position: callgrind_annotate pads "(%5.2f%%)", so a percentage under
        # ten contains a space and shifts every later field by one.
        line = $0; sub(/ *\[[^]]*\] *$/, "", line)
        name = line; sub(/.*[ \t]/, "", name); sub(/.*:/, "", name)
        if (cost + 0 > 0) { c[n] = cost + 0; nm[n] = name; n++ }
    }
    END {
        # insertion sort, descending by cost (n is small)
        for (i = 1; i < n; i++) {
            for (j = i; j > 0 && c[j-1] < c[j]; j--) {
                t = c[j]; c[j] = c[j-1]; c[j-1] = t
                t = nm[j]; nm[j] = nm[j-1]; nm[j-1] = t
            }
        }
        total = 0
        for (i = 0; i < n; i++) total += c[i]
        printf "SITES %s  functions=%d  total=%d\n", bin, n, total

        printf "  H1  %-24s %12d  %6.2f%%\n", nm[0], c[0], 100*c[0]/total
        printf "  H2  %-24s %12d  %6.2f%%\n", nm[1], c[1], 100*c[1]/total

        # remaining pool: everything but the two hottest
        m = 0; rest_sum = 0
        for (i = 2; i < n; i++) { rc[m] = c[i]; rn[m] = nm[i]; rest_sum += c[i]; m++ }
        if (m < 2) { print "  (fewer than four candidate functions)"; exit }

        s = seed
        for (d = 0; d < 2; d++) {
            s = (s * 1103515245 + 12345) % 2147483648
            u = (s / 2147483648.0) * rest_sum
            acc = 0; k = 0
            while (k < m - 1 && acc + rc[k] < u) { acc += rc[k]; k++ }
            printf "  S%d  %-24s %12d  %6.2f%%  weight=%.6f\n",
                   d + 1, rn[k], rc[k], 100*rc[k]/total, rc[k]/rest_sum
            rest_sum -= rc[k]; rc[k] = 0            # draw without replacement
        }
    }'
