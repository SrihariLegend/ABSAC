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

### CORRECTION: this is shipped, not merely unset — my first reading was wrong

I first recorded this as *"already known upstream and deliberate"*, on the
strength of `jaq#42` ("jq ships without `-DNDEBUG`") and a jq FAQ entry about
`[-2] | implode`. **That reading was wrong and is withdrawn.** The FAQ entry
describes an assert the project *removed* in 1.7.1, not a policy of shipping
asserts; `jaq#42` mentions the flag in passing while benchmarking something
else. Neither is a decision by jq's maintainers about their build.

What decides it is the shipped artifact. jq's **official `jq-1.8.2` release
binary** (`jq-linux-amd64`) contains **100 assert expression strings**,
including:

```
p->tokenpos <= p->tokenlen
p->tokenpos < p->tokenlen
src/jv_parse.c
```

and a build of the same source with `-DNDEBUG` contains **zero** of them. So
asserts are compiled into the binaries users download, and they cost ~7-10%
of wall clock. Whether that is intended is a question for the maintainers — it
is not something this measurement establishes either way.

Compared against the actual release artifact, best of 7:

| | official 1.8.2 | local `-DNDEBUG` | |
|---|---:|---:|---:|
| audit workload | 291 ms | 263 ms | **−9.6%** |
| logs workload | 412 ms | 371 ms | **−10.0%** |

The official binary's other build flags are unknown, so this is corroboration
rather than a controlled comparison; the controlled A/B at fixed optimization
above is the evidence, and it gives 6.6% / 8.5%. The two agree.

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

## 4. The teardown worklist starts on the heap, and moving it to the stack is worth 0.8%

`jv_free` opens with `jv* pending = NULL; ... cap = 0;`, so the *first* array or
object it tears down -- which is every array and every object -- calls
`jv_mem_realloc(NULL, ...)`, a malloc, and then `jv_mem_free` at the end. For a
workload of 200k small objects that is 200k malloc/free pairs spent on the
bookkeeping of freeing, on top of the memory being reclaimed. It also calls
`jv_mem_free(NULL)` on every scalar.

Starting the worklist as a 16-element stack array and moving to the heap only
on overflow:

| | before | after | |
|---|---:|---:|---:|
| audit workload | 3,540,333,506 | 3,511,886,096 | **−0.80%** |
| logs workload | 5,425,236,000 | 5,391,729,973 | **−0.62%** |

Small, and smaller than the 2.70% `realloc` figure suggests, because that cost
is concentrated in the *few large* worklists — the top-level 200k-element array
— which still go to the heap. The saving is ~200 instructions x 200k object
frees, and it comes out where that predicts. Not proposed: 0.8% does not pay
for a maintainer's review, and saying so is more useful than shipping it.

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

**One PR, and it is the test fix — not the flag.**

`PATCH-jq_test.diff` hoists the side-effecting `test_start_state()` calls out of
`assert()` in `src/jq_test.c`. That change stands on its own: **anyone who
builds jq with `-DNDEBUG` today silently stops running the check that
`test_jq_start_resets_state` exists to perform, and `make check` reports
success anyway.** Distro packagers define `NDEBUG` as a matter of course, so
this is not hypothetical.

Verified rather than reasoned. The probe is a string literal that only the
eliminated call would print, searched in the linked binary:

| source | `NDEBUG` | string present |
|---|---|---|
| original | no | 1 |
| original | **yes** | **0** ← the check is gone |
| fixed | **yes** | **1** ← restored |
| fixed | no | 1 |

`make check` passes 7/7 and 14/14 both ways.

`-DNDEBUG` as a build change is **not** part of this PR. It is a 7-10% win and
the measurement is above, but it would disable 367 internal consistency checks
in a tool whose whole job is handling input it did not author, and that is a
maintainer's call rather than a drive-by. The honest form is the test fix in a
PR, with the measurement posted alongside so the decision can be made on
numbers.

Neither `jv_get_kind` inlining (an ABI decision) nor SIMD (a project) nor the
0.8% worklist change is proposed. For `jq#1892`, finding #1 is a boundary
condition worth posting — measured, and explicit that it depends on workload
shape.
