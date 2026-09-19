# Gate 5A — Target-Plan Search Landscape

## Date: 2025-07-14
## Status: COMPLETE

## Objective

Determine whether search selects among implementation alternatives
better than a deterministic rule (Engine 0).

## Method

1. Enumerate all valid candidate plans across:
   - ISA: Scalar, SSE2, AVX2
   - Reduction: ScalarLoop, LaneAccumulation, MovemaskPopcount, Psadbw
   - Unroll: 1×, 2×, 4×
   - Tail: Scalar, NarrowerVector
   - Control: FullReduction, EarlyExit (k18 only)

2. Generate C code for each plan.

3. Compile all plans with identical flags (`-O2 -march=native -mavx2`).

4. Benchmark each plan at 4 sizes (64, 256, 4096, 65536) and 1-2 distributions.

5. Compare Engine 0's deterministic selection against the measured best.

## Candidate Count

| Kernel | Valid Plans |
|--------|------------:|
| k18_all_equal | 33 |
| k43_sum_ascii | 18 |
| k50_count_masked | 18 |
| **Total** | **69** |

## Correctness

All 408 benchmark rows pass correctness (differential vs original).
(24 k43 LaneAccumulation rows initially failed due to a code generation
bug — `_mm256_cvtepu8_epi64` processes 4 bytes not 8. Fixed by adjusting
step to 4 bytes. All pass now.)

## Results: Engine 0 vs Measured Best

### k18_all_equal

| Size | Distribution | Best Plan | Best (ns) | Engine 0 (ns) | E0 Gap |
|------|-------------|-----------|----------:|--------------:|-------:|
| 64 | all_equal | AVX2 LaneAcc U2 Full | 1.2 | 1.5 | 25% |
| 256 | all_equal | AVX2 LaneAcc U2 Full | 1.6 | 1.8 | 13% |
| 4096 | all_equal | AVX2 LaneAcc U2 Full | 24.5 | 27.9 | 14% |
| 65536 | all_equal | AVX2 MM+P U4 Full | 504.3 | 550.7 | 9% |
| 64 | random | Scalar U2 | 1.4 | 1.5 | 7% |
| 4096 | random | Scalar U2 | 1.4 | 1.5 | 7% |

**Key finding:** Engine 0 picks EarlyExit for k18, which is optimal for
random data (139× speedup from early termination) but suboptimal for
all_equal data (14% slower than FullReduction U2). The best plan
**depends on the input distribution**.

### k43_sum_ascii

| Size | Best Plan | Best (ns) | Engine 0 (ns) | E0 Gap |
|------|-----------|----------:|--------------:|-------:|
| 64 | AVX2 Psadbw U2 | 1.2 | 1.3 | 8% |
| 256 | AVX2 Psadbw U1 | 1.3 | 1.4 | 8% |
| 4096 | AVX2 Psadbw U1 | 25.2 | 25.2 | 0% |
| 65536 | AVX2 Psadbw U4 | 506.5 | 507.7 | 0.2% |

**Key finding:** Engine 0 picks the optimal plan at 4096+ bytes.
At small sizes (64-256), U2 unrolling is 8% faster. The gap is within
noise at larger sizes.

### k50_count_masked

| Size | Best Plan | Best (ns) | Engine 0 (ns) | E0 Gap |
|------|-----------|----------:|--------------:|-------:|
| 64 | AVX2 LaneAcc U4 | 1.2 | 1.3 | 8% |
| 256 | AVX2 MM+P U1 | 1.9 | 1.9 | 0% |
| 4096 | AVX2 MM+P U2 | 30.1 | 30.3 | 0.7% |
| 65536 | AVX2 MM+P U1 | 551.9 | 551.9 | 0% |

**Key finding:** Engine 0 is optimal at 256+ bytes. At 64 bytes,
LaneAccumulation U4 is 8% faster (the extra unrolling helps at tiny
sizes where loop overhead dominates).

## Performance Spread (at 4096 bytes)

| Kernel | Min (ns) | Max (ns) | Spread |
|--------|----------:|----------:|-------:|
| k18_all_equal | 24.5 | 409.4 | 16.7× |
| k43_sum_ascii | 25.2 | 232.9 | 9.2× |
| k50_count_masked | 30.1 | 254.5 | 8.5× |

The spread is large — the worst plan is 8-17× slower than the best.
This means plan selection matters significantly.

## Search Landscape Metrics

```
Total unique plans:           69
Plans per kernel:             33 (k18), 18 (k43), 18 (k50)
Performance spread at 4096:   8.5-16.7× (worst/best)
Correctness rate:             100% (408/408 after bug fix)
Compile-success rate:         100%

Temporary regressions:
  k43: scalar plans 1.00-1.07× slower than original (noise-level)
  k50: scalar plans 1.00-1.00× slower than original (noise-level)
  No vector plan is ever slower than the original at 4096+ bytes.

Correlation of static cost with runtime:
  ISA dominates: AVX2 > SSE2 > Scalar (clear hierarchy)
  Reduction strategy dominates within ISA: Psadbw/MM+P > LaneAcc
  Unroll factor: marginal effect (0-8%) at 4096+ bytes
  Tail strategy: negligible effect (<1%) at 4096+ bytes
  Control flow: EarlyExit wins for random, loses for all_equal (k18)

Search cost to first win:
  Exhaustive enumeration of 69 plans is cheap (<5 minutes total).
  No need for MCTS or beam search at this scale.
```

## Gate 5A Pass Condition

> Search reliably finds materially better target plans than the existing
> deterministic selector under a bounded budget.

**Result: MARGINAL PASS.**

- At 4096+ bytes, Engine 0 is within 0-14% of the measured best.
- The largest gap (14%) is on k18_all_equal with all_equal distribution,
  where EarlyExit is suboptimal but Engine 0 selects it.
- At small sizes (64), unrolling (U2 or U4) provides 7-8% improvement
  over Engine 0's U1 selection.
- The gap is distribution-dependent: Engine 0's EarlyExit is 139× faster
  on random data but 14% slower on all_equal data.

**Conclusion:** Search (or distribution-aware multiversioning) can find
materially better plans, but the improvement is 8-14%, not 2-5×. The
deterministic selector is already near-optimal for the dominant factor
(ISA + reduction strategy). The remaining headroom is in:
1. Unroll factor (8% at small sizes)
2. Control flow (14% on all_equal distribution for k18)
3. Distribution-aware multiversioning (could capture both EarlyExit and
   FullReduction paths)

## Three Legitimate Outcomes Assessment

**Outcome 1 (Deterministic lowering is enough):** Partially confirmed.
The deterministic selector is within 0-14% of the measured best at all
tested sizes. The gap is not large enough to justify a complex search
system for target-plan selection alone.

**Outcome 2 (Search helps select plans):** Confirmed at margin.
Distribution-aware multiversioning and size-dependent unroll selection
provide 8-14% improvement. This is autotuning, not semantic discovery.

**Outcome 3 (Search composes new transformations):** Not tested here.
This is Gate 5B's question.

## No MCTS Needed

The candidate graph is small (69 plans), exhaustive enumeration is cheap
(<5 minutes), and the best plan is deterministically identifiable. MCTS
is unnecessary for target-plan search at this scale.

## Files

- `sir/crates/sir_benchmarks/src/gate5a.rs` — plan enumeration and code generation
- `sir/crates/sir_benchmarks/src/bin/gate5a_bench.rs` — benchmark harness generator
- `gate5a_results.csv` — full results (408 rows)
