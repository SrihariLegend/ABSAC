# A use case for ABSAC: hand-rolled search loops, which clang cannot vectorize

Measured 2026-10-12. `work/search-case/`. Reproduce:

```bash
mkdir -p bin
clang -O3 -o bin/compare compare.c && ./bin/compare   # the size sweep
clang -O3 -o bin/verify  verify.c  && ./bin/verify    # correctness on the large path
```

and the referee, which shows the rewrite firing and the gate's own verdict:

```bash
SIR_OPT_TRACE=1 sir/target/release/emit_c_diff probe2_O1.ll \
    --function find_first --cases 500
#   SIR_OPT find_first def=403
#   EMITC_SUMMARY clean=1 mismatched=0 lower_refused=0 harness_errors=0
```

## The case

A developer writes the ordinary linear search:

```c
long find_first(const unsigned char *a, long n, unsigned char c) {
    long i;
    for (i = 0; i < n; i++)
        if (a[i] == c) return i;
    return n;
}
```

clang -O3 compiles that to a **scalar byte-at-a-time loop with two branches per
byte**, and does not call `memchr`:

```
find_first:
	movq	%rsi, %rax
	testq	%rsi, %rsi
	jle	.LBB0_5
	xorl	%ecx, %ecx
.LBB0_2:
	cmpb	%dl, (%rdi,%rcx)
	je	.LBB0_3
	incq	%rcx
	cmpq	%rcx, %rax
	jne	.LBB0_2
```

Verified, not assumed: `clang -O3 -S` on this function emits **0 `memchr`
calls and 0 vector instructions**. An early-exit search loop that returns an
index is not vectorizable — the trip count depends on the data — and LLVM has no
`memchr` idiom recognition for it. The library call is right there and clang
will not use it.

ABSAC recognises the loop and rewrites it to a `memchr`-based scan. Its own
emitted code (`emitted.c`, verbatim) is:

```c
static uint64_t __sir_memchr(const uint8_t *base, int key, uint64_t n) {
    if (n == 0) return 0;
    uint64_t head = n < 16 ? n : 16;
    for (uint64_t i = 0; i < head; i++)
        if (base[i] == (uint8_t)key) return i;
    if (n <= 16) return n;
    const void *hit = memchr((const void *)(base + 16), key, (size_t)(n - 16));
    return hit ? (uint64_t)((const uint8_t *)hit - base) : n;
}
```

A 16-byte inline head scan so early hits avoid the call, then the vectorized
`memchr` for the rest. It is a better kernel than a bare `memchr` call, and it
is not one a developer would have written by hand — it exists because ABSAC
knows both the search semantics *and* the size distribution it has to win on.

## The measurement

Original loop vs ABSAC's emitted rewrite, same compiler, same `-O3`, both
`noinline` so the harness cannot inline one and not the other. Needle absent, so
every byte is examined. Best of N.

| size | clang -O3 loop | ABSAC rewrite | speedup |
|---:|---:|---:|---:|
| 64 B | 18 ns | 15 ns | 1.20× |
| 1 KiB | 145 ns | 18 ns | **8.06×** |
| 64 KiB | 8,428 ns | 329 ns | **25.62×** |
| 1 MiB | 134,862 ns | 6,952 ns | **19.40×** |
| 16 MiB | 3,013,860 ns | 273,428 ns | **11.02×** |
| 64 MiB | 16,330,263 ns | 2,529,531 ns | **6.46×** |

Peak 25.6×, and it stays 6× at 64 MiB where the scan is memory-bandwidth bound
rather than instruction bound.

## Correctness

Two independent checks, because they cover different paths.

- **ABSAC's referee**: `def=403`, `clean cases=500`, `mismatched=0`. This is the
  equivalence obligation plus a differential against the original.
- **`verify.c`**: the referee's differential feeds only `lens[] = {0..32}`, so it
  never reaches the branch where `__sir_memchr` actually calls `memchr`
  (`n > 16`). The path I am claiming a win on is the path it does not check.
  Checked separately: **90 configurations, 0 mismatches**, sizes
  0/1/15/16/17/31/32/33/64/1000/65536/1 MiB/4 MiB × needle positions
  {absent, 0, 15, 16, 17, 31, 32, n/2, n-1}.

## The referee cannot see any of this, and that is the finding

Both the differential driver and the timing driver use

```c
static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};
```

**A 32-byte ceiling.** The `search` family's win is a function of scan length —
it is the cost of a byte loop versus a vectorized library call — and at n ≤ 32
there is no scan to vectorize. So:

- The referee timed the rewrite at **0.82×** (`ref_ns=752064 new_ns=916862`).
- The same rewrite at 64 KiB is **25.6×**.

The harness did not measure the transformation; it measured the size guard. This
is the same defect the ledger already records for `crc32pseudo`, where "the
referee's input distribution, not the code, decided what counted as clean" —
here it decides what counts as *fast*, and it has the same effect: a correct,
large win reads as a loss, and the family looks useless on real code.

**This is very likely why `search` does not fire on the Embench corpus.** Anyone
reading the gate would conclude the family does not pay. It pays; the ruler is
32 bytes long.

## The shape requirement, stated plainly

The recognizer does not match the `return -1` convention. It requires the
not-found sentinel to *be* the scan bound:

- carried values must be exactly `(false, <bound>, 0)` — found flag, bound,
  counter;
- the result select must be `found ? index : bound`;
- termination must be `Lt(index, bound)` against that same carried bound.

So `return n` matches and `return -1` does not, even though both are the same
loop. This is a real limitation, not a detail: `memchr` returns a pointer and
index-based searches conventionally return `-1`, so a large fraction of
real-world instances are refused on the sentinel alone. The `return -1` form
lowered to carried `(false, -1, 0)` with termination against the *parameter*
`n`, and the recognizer refused it because the `Lt` did not meet the carried
bound.

## Scope, honestly

- **One function shape**, tested at eight needle positions against thirteen
  sizes. Not a corpus.
- The win depends on the input being long and the hit being late or absent. For
  a hit in the first 16 bytes the head scan makes it a wash (1.20× at 64 B);
  for a hit at byte 0 it is strictly more work than the original.
- Nothing here is a compiler result yet: the rewrite was verified on a
  hand-written kernel of the shape the family accepts, not discovered by
  lowering a third-party project.
