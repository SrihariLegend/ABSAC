# Redis `hllMerge` — reconstruction, and why it is not a PR

## Status: **not mergeable. Do not spend more time on this track.**

Redis 8.0 and later already ship `hllMergeDenseAVX2` and `hllMergeDenseAarch64`
with runtime CPU dispatch. The original finding was measured against 7.2.5,
whose `hyperloglog.c` contains only the scalar loop — the win was real against
that tree and was obsolete upstream before it was measured. Nothing here can be
merged, and the residual "upstream over-reads" angle is dead too: `sds` allocates
a trailing NUL, so the extra byte is inside the allocation in practice.

Recorded in `docs/CAPABILITY_LEDGER.md` (2026-10-12, "CORRECTION: the `hllMerge`
win is upstream-obsolete"). This directory exists because the original artifacts
were written to `/tmp` and lost.

## What is here

| Path | What it is |
|---|---|
| `PATCH.diff` | The patch, 93 insertions / 6 deletions, against redis 7.2.5 |
| `src/` | redis 7.2.5, patch applied (`git apply`) |
| `bin/redis-server-v0` | Pristine 7.2.5, clang -O3 |
| `bin/redis-server-v6` | Patched, same compiler and flags |
| `test_merge.c` | Equivalence + placement witness for the kernel |
| `hllMergeDense.inc` | The shipped function, extracted verbatim by `awk` for the witness |
| `measure.sh` | The measurement, dataset build included |
| `prof/*.callgrind` | Raw callgrind output (gitignored) |

Reproduce: `git apply PATCH.diff` in `src/`, `make -j2 CC=clang OPT=-O3`, then
`./measure.sh 200`.

## The kernel

`hllMergeDense` merges a dense HLL's packed registers into `max[]` eight at a
time. Registers are six bits apiece in a little-endian bit stream, so the group
holding registers `8k..8k+7` starts on a byte boundary and spans exactly six
bytes. The routine loads those six bytes, spreads the eight fields one per byte,
and takes a byte-wise maximum against `max[8k..8k+7]` in one SWAR step.

**Bounds.** The wide loop runs 2047 times, for registers 0..16375; its eight-byte
load starts at byte `6k` with `k <= 2046`, so the highest byte touched is 12283
against an array whose last byte is 12287. The tail covers registers
16376..16383 as two groups of four, each read as exactly the three bytes it
occupies, the last ending at 12287.

## The witness, and the two defects it caught

`test_merge.c` asks two different questions, and both can fail.

**Equivalence** — 320 trials over random, identical, all-zero, all-max and
one-hot distributions, against the upstream loop compiled in the same
translation unit. All identical. The `max[]` values are random bytes 0..255, not
registers, because `hllMerge` is `MAX(max[i], register)` and says nothing about
what a caller put in `max`.

**Bounds** — a placement witness. The register array is mapped so its final byte
is the final byte of a page with a `PROT_NONE` page immediately after it, so any
read of byte 12288 is `SIGSEGV` rather than a silent over-read. Forked, so the
parent survives to say which side faulted:

```
upstream      : SIGSEGV (reads registers[12288])
hllMergeDense: returned
```

The witness is *for* upstream failing that line. It is the recorded over-read
reproduced from source: `HLL_DENSE_GET_REGISTER` reads `_p[_byte+1]`
unconditionally, and for register 16383 — whose six bits end inside byte 12287 —
that is byte 12288, one past the array and unnecessary.

**Two defects, both mine, both found by the witness rather than by reasoning:**

1. **The SWAR max was wrong for `max[]` bytes >= 128.** The first version used
   `((m | H) - v) & H`, which needs bit 7 free in both operands. `max[]` is not
   guaranteed to hold six-bit registers, so it is not. Upstream returned 144
   where the kernel returned 39. Fixed by comparing the low seven bits and
   letting any byte with bit 7 set win outright, which it must, since `v <= 63`.
   The equivalence sweep over bytes 0..255 is what holds that fix.

2. **The witness's own guard-page arithmetic was wrong**, mapping two pages for a
   three-page array and computing a pointer before the mapping. This is the third
   time in this project that the *checking apparatus* was the defective
   component. It is worth naming each time, because a witness that crashes
   before it reports reads exactly like a witness that found nothing.

## Measurement

`measure.sh` runs each binary under callgrind, at N=0 and at N=200
`PFCOUNT k1 k2 k3 k4`, and subtracts so that what is reported is the marginal
cost of the commands rather than the cost of loading a database. All four keys
are asserted dense (`PFDEBUG ENCODING`) before anything is measured — a sparse
key takes the other branch and would say nothing about this kernel.

Four runs, callgrind instruction counts:

| | v0 (upstream) | v6 (patched) |
|---|---:|---:|
| marginal, 200 commands | 239.5–240.1 M | 60.4–60.9 M |
| per command | ~1,197,000 | ~304,000 |

**3.93x – 3.97x**, stable to about 1% across four runs. The recorded figure for
the lost original was 3.484x on the same regime and method; same sign, same
order, not the same number, and the difference is not attributable without the
original.

**A noise observation worth recording.** At N=0 the two binaries do identical
work, yet v6's startup figure ranged 7.66 M – 8.15 M across runs while v0's
stayed near 8.15 M. The cause is the harness racing the server's accept loop
with a `PING` poll (the number of poll cycles before the client connects is not
fixed), not a code difference. It cancels in the marginal — which is stable to
under 1% — but it means the raw `with_commands` totals must not be compared
directly, and a run that reported only those would be off by up to 0.5%.

## Deviations from the lost original, flagged not smoothed

The original `PATCH.diff` was lost with `/tmp` and this is a reconstruction. It
matches the recorded *behaviour and bounds*; it does not and cannot match the
recorded bytes.

1. **Size.** Recorded as 19 changed lines in 2 hunks. This is 93 insertions / 6
   deletions in 3 hunks. The difference is comment volume, not logic — the
   bounds argument is written down here rather than assumed, and the hunk count
   differs because the helper is its own hunk.
2. **Vector instructions.** The record says "zero vector instructions in v6's
   `hllMerge`". clang -O3 auto-vectorizes this reconstruction into SSE2
   (`punpcklqdq`, `psllq`, `por` over two groups per iteration). The bounds claim
   is therefore made about the *generated* code, verified by the placement
   witness on the same -O3 build, rather than at source level. That is a
   weaker guarantee than the original's, because it is a property of this
   compiler at this optimization level, and the `_Static_assert`s above do not
   constrain it. A future clang that widens a load would be caught by the
   witness, which is why the witness is kept and is wired to fail.
3. **The `max[]` domain.** Upstream's compare accepts any byte. The first
   reconstruction did not; see defect 1 above. The shipped version does.
4. **Endianness.** `memcpy` into a `uint64_t` is a native load and would be
   wrong on a big-endian build, where upstream's byte-indexed macro is correct.
   Hand-assembling the bytes fixes it but costs eight loads — clang emits
   `movzbl`/shift chains, not one load. The shipped version keeps the single
   load and byte-swaps under `__ORDER_BIG_ENDIAN__`.
