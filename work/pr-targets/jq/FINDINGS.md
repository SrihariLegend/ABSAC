# jq 1.8.2 — what the profile says, and what it does not

Measured 2026-10-12 on the audit bench, `clang`, callgrind instruction counts
(`Ir`) plus wall clock. Two workloads, both 200,000 records:

| | audit `records.json` | `work/logs.json` |
|---|---|---|
| shape | `{"k":123,"v":456,"t":"tag7","f":12.34}` | `{"ts":…,"level":…,"service":…,"message":"<12 words>","trace":"<32 hex>"}` |
| size | 9.8 MB | 41.2 MB |
| query | `[.[] \| select(.v > 50000) \| {k, t}] \| length` | `[.[] \| select(.level=="error") \| {ts, service}] \| length` |

## 1. The simdjson hypothesis has a measurable boundary, and the audit workload is on the wrong side of it

`jq#1892` ("simdjson", open since 2019) proposes a SIMD stage-1 scanner. The
quantity that decides whether stage 1 can pay is **how many bytes pass between
structural characters**, because that is what a 16-byte (or 64-byte) block can
skip without branching. Measured on the raw bytes:

| | audit workload | logs workload |
|---|---:|---:|
| mean run between structural chars | **4.19 B** | 10.00 B |
| max run | **5 B** | 10 B |
| 16-byte blocks with **no** structural char | **0 / 611,929 (0.00%)** | 1,126,782 / 2,577,676 (**43.71%**) |
| `jv_parser_next` self cost | 17.87% | **41.73%** |

On the audit workload **not one** 16-byte block is free of a structural
character. A SIMD scanner would load, compare and movemask every block and find
a hit in every one — pure overhead over the byte-at-a-time loop it replaces.
On the logs workload 43.71% of blocks are skippable and the parser is 41.73% of
the program.

**So "should jq use simdjson" is not answerable in general, and an answer given
on one workload shape is not evidence about the other.** That is the finding.
The two shapes differ by a factor that inverts the conclusion, and both are
ordinary JSON.

## 2. jq ships with asserts enabled, and it costs 6.6–8.5% of wall clock

`NDEBUG` appears **nowhere** in `configure.ac`, `Makefile.am` or
`src/Makefile.am`. A stock `./configure && make` produces
`CFLAGS = -g -O2 -pthread`, so every one of the 367 `assert()`s in `src/`
is live in a release build — including `assert(p->tokenpos <= p->tokenlen);`,
which runs **once per input character**:

```
132,971,616 ( 2.45%)    assert(p->tokenpos <= p->tokenlen);
```

Two builds, same compiler, same optimization level, only `-DNDEBUG` differing:

| | asserts ON | `-DNDEBUG` | instruction-count change |
|---|---:|---:|---:|
| audit workload | 3,918,415,481 | 3,305,154,345 | **−15.65%** |
| logs workload | 6,838,575,722 | 5,400,431,251 | **−21.03%** |

Wall clock, best of 5, and output byte-identical on both:

| | asserts ON | `-DNDEBUG` | wall-clock change |
|---|---:|---:|---:|
| audit workload | 289 ms | 270 ms | **−6.6%** |
| logs workload | 402 ms | 368 ms | **−8.5%** |

### This is already known upstream, and it is deliberate

It is not a discovery. `jaq`'s issue #42 records "jq ships without `-DNDEBUG`"
and the author uses `-DNDEBUG` in comparisons. And jq's own FAQ shows the
asserts are load-bearing for safety:

```
jq-1.6 -n '[-2] | implode'
  → Assertion failed: (codepoint >= 0 && codepoint <= 0x10FFFF),
    function jvp_utf8_encode, file src/jv_unicode.c, line 101.
jq-1.7.1 (asserts off)  → "�"
```

Asserts are how jq turns a malformed input into a crash rather than a silently
wrong answer. A PR that just adds `-DNDEBUG` would be arguing against a
deliberate decision, with numbers the maintainers already have.

### A caveat that makes a blanket `-DNDEBUG` wrong anyway

`src/jq_test.c` puts a **side-effecting call** inside an assert:

```c
assert(test_start_state(jq, prog));   // ×2 in test_jq_start_resets_state
```

`test_start_state()` fetches the error message, validates it, prints on failure
and frees. Under `-DNDEBUG` the call disappears entirely, so
`test_jq_start_resets_state` — whose stated purpose is "Test jq_state is reset
after subsequent calls to jq_start" — stops testing that. `make check` still
reports **9/9 PASS**, which is exactly the failure mode this project treats as
the worst kind: a check that cannot fail has not checked anything.

So the correct change is *not* a global `-DNDEBUG`. If asserts are to be off in
release builds, `NDEBUG` must be defined for `libjq` and `jq` but **not** for
`jq_test`, and `test_start_state` should be called outside the assert.

## 3. Where jq's time actually goes

On the **audit workload** (3.54 B instructions), inclusive:

| bucket | share |
|---|---:|
| `jv_parser_next` | 48.81% |
| — tokenizer state machine and scanning (self) | 17.87% |
| — value construction (callees) | ~31% |
| `jq_next` (the interpreter) | 47.12% |
| malloc / free / realloc, as a family | **~31%** |
| UTF-8 validation (`jvp_utf8_is_valid`) | 0.63% |

The parser's *self* cost is not character scanning on this workload — it is
value construction. `jv_number_with_literal` costs **464 instructions per
number** (600,073 calls, 7.86%), and `jv_string_sized` **270 per string**
(1,000,000 calls, 7.63%) for keys that are one to three characters long.

`jv_free` is **12.10% self**, and it opens with a worklist:

```c
void jv_free(jv j) {
  jv* pending = NULL;
  size_t len = 0, cap = 0;
  ...
```

so tearing down an array or object allocates and grows a heap worklist, and
every scalar free still pays the prologue. Freeing the 100,499-element result
of the audit query costs 197 M instructions — about **1,960 per object**.

The dominant cost in jq is per-value allocation, refcounting and teardown. SIMD
on the scanner does not touch it.

## What I checked and rejected

- **`jv_string_sized` re-validates UTF-8 the parser already handled.** It does
  not: `jv_parse.c:503` copies raw bytes through as `*out++ = c;`, so invalid
  input reaches `jv_string_sized` and the validation is what routes it to
  `jvp_string_copy_replace_bad`. Not redundant.
- **Inlining `jv_get_kind`** (2.34% audit, 1.45% logs) is a one-line body
  (`return JVP_KIND(x);`) but it is an exported libjq symbol, so inlining it in
  the header is an ABI change — a decision for the maintainers, not a drive-by.
- **The `max[]`/`alloc` micro-optimizations** that would follow are real but are
  micro-optimizations of a code path whose cost is dominated by malloc.

## Status

**No PR written.** Every candidate either argues against a deliberate upstream
decision (NDEBUG), needs an ABI ruling (`jv_get_kind`), or is a project rather
than a patch (SIMD, allocation). Recommendation is to stop treating jq as the
vehicle for a first PR, and instead post finding #1 to `jq#1892` as a measured
boundary condition — that is a real contribution to that thread, it is honest
about its own workload dependence, and it costs the maintainers nothing to read.
Finding #2's test-weakening caveat is a legitimate standalone issue report.
