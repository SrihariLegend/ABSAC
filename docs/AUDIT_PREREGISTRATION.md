# Opportunity audit — pre-registration

**Frozen 2026-09-27, before any of the six applications below was read, built or
profiled.** Tree state: commit `687b34763812f6fa72fbb73ea7f05d2d7eb7c89a`, evidence
manifest `tmp/verify/manifest.sha256`
(`1d6dca0041e90dd3bd4cd87391a732c7f80199a363aef257d7054a052abc0408`), `VERIFY PASS
mode=fast`, 42/42 corpora green.

This document exists to be falsifiable. Everything in it that could be adjusted after
seeing a result — the applications, the workloads, the site tags, the predictions, the
success criteria — is fixed here, and any later change must be appended as a dated
correction rather than edited in place.

## The question

The project's own record says the only family that fires on independently-authored code
is `arithmetic` (multiply→shift), and that the domain families recognise counted,
effect-free, single-stream array reductions — a shape clang vectorizes before the
lowerer sees it. The commercial question is whether there is an **automatic opportunity
set**: sites where a certified, profitable patch can be produced without a human
supplying facts per case.

Three of the advisor's warnings are specifically on trial here, because each is
currently unfalsifiable as stated:

- **W1 — the concentration may be ours, not the world's.** "Your selection process and
  your researchers' vocabulary could themselves create the apparent concentration.
  'Everything successful turns out to have an algebraic description' is unfalsifiable
  unless classes are defined before searching."
- **W2 — a null against a strong baseline is a product null.** The `redisPopcount`
  result that vanished against a stronger baseline is "a product null, not evidence of
  available headroom".
- **W3 — instruction reductions are not the product.** "If only callgrind improves, you
  have instruction reductions, not yet the performance product."

This audit is designed so that W1 and W3 can fail. W2 is assumed and not testable here.

## Applications — frozen, six, one per workload class

Chosen by **workload class, not by suspected optimizability**, and selected before any
reading. Nothing here appears in `gate6a/`, the 19 Embench corpora, or the Redis work —
those have been used to shape the transformations and are contaminated for this purpose.

| # | Application | Workload class | Why it is in the sample |
|---|---|---|---|
| A1 | **GNU grep** | bounded first-match scan | This is the project's declared wedge. It is the positive control. |
| A2 | **zstd** | compression / entropy coding | Bit-level scans and match finding — adjacent to the favoured class. |
| A3 | **jq** | parsing and query | Recursive descent, hash tables, pointer chasing. |
| A4 | **sqlite** | database / storage | Switch-dispatch interpreter over B-tree pages. |
| A5 | **libpng** | image transform | Per-row byte filters — counted single-stream byte loops. |
| A6 | **lua** | language runtime | VM dispatch. Control-heavy. |

**Predeclared prediction per application**, recorded now and scored without revision:

| # | Prediction |
|---|---|
| A1 | **Yield expected.** If the wedge cannot produce a certified profitable patch here, the wedge is dead and that is the audit's most important result. |
| A2 | **Partial.** Expect recognition in the bit/entropy paths, not in the match finder. |
| A3 | **No yield expected.** |
| A4 | **No yield expected** in the VDBE core; possible in page-level byte work. |
| A5 | **Yield expected** — row filters are close to the favoured class. |
| A6 | **No yield expected.** |

Three of six predict no yield. That is deliberate: a sample chosen for optimizability
would make a positive result meaningless.

## Workloads — frozen

Each is deterministic, CPU-bound, and reproducible from a committed script. The harness
is committed before profiling; if it must change, the change is appended as a dated
correction and the affected application is re-run from scratch.

| # | Workload |
|---|---|
| A1 | `grep -F -c <literal>` over a fixed concatenation of source files |
| A2 | `zstd -3 -c` over a fixed tar of source files |
| A3 | a fixed jq filter over a fixed JSON document |
| A4 | a fixed SQL script over a fixed database: create, insert N rows, index, query |
| A5 | a fixed read → transform → write pass over a fixed PNG |
| A6 | a fixed Lua benchmark script |

Versions are pinned at clone time and recorded with the commit hash of each upstream.

## Site sampling — frozen

Per application, four sites, 24 total:

1. **Two hottest non-overlapping regions** by self cost.
2. **Two sampled from the remaining hot regions**, with probability proportional to
   self cost, using a seed fixed now: `0xA0D17`.

Sampling weights and **failed intake** are recorded for every site. A site that cannot
be lowered at all is still a site and still counts in the denominator.

Granularity is the **function** — the unit at which ABSAC transforms.

## Site tags — assigned before proposals, from the taxonomy

Every site is tagged before any candidate is built, from this fixed list:

- `S1` scan / reduction / set
- `S2` other numeric work
- `S3` pointer- or control-heavy
- `S4` external-cost dominated (I/O, allocation, syscall)

**The headline result of this audit is the 2×2 of tag against outcome.** The
predeclared prediction is that profitable patches concentrate in `S1`. The
falsification is a profitable patch at an `S3` or `S4` site.

## Outcome categories — refusals in the denominator

Every site gets exactly one outcome, and every outcome is reported:

| Outcome | Meaning |
|---|---|
| `intake-failed` | the lowerer refuses the function; nothing was attempted |
| `unsupported` | lowered, but no family's pattern applies — **this is not "no opportunity"** |
| `unproved` | a candidate was built and the obligation did not discharge |
| `certified-identical` | proved correct, emitted code the same |
| `certified-unprofitable` | proved correct, not faster |
| `profitable` | proved correct and measurably faster |

Per site: **at most 8 proposals.** Rejections are recorded with their reason.

## Cost model

Cost includes the conversions, guards and surrounding execution the patch requires —
not the replaced loop in isolation. A patch that speeds a loop but adds a guard loop
that undoes it is `certified-unprofitable`.

## Methodology, given this machine

`perf` is absent; `callgrind` is the only profiler. `hyperfine` is absent. **`nproc` is
2, on WSL2.** Therefore:

- **Screening** is callgrind instruction count, which is deterministic.
- **The end-to-end claim** is wall-clock, repeated, reported as median with the spread.
- **A baseline-versus-baseline control runs first**, to measure the noise floor of this
  machine under this harness before any result is interpreted.

**Predeclared limitation:** if the baseline-versus-baseline noise floor exceeds 5% on an
end-to-end workload, criterion C below **cannot be evaluated on this machine**, and will
be reported as *unevaluable* — not as a pass, and not as a fail. W3 stands unrefuted in
that case, and the honest conclusion is that instruction reductions were obtained and
the performance product was not demonstrated here.

## Success criteria — fixed now

From the advisor's gate, unchanged:

- **A.** Two independent certified improvements.
- **B.** At least one exceeding **5%** on a predeclared end-to-end workload.
- **C.** The 5% must survive the noise floor of this machine (see above).
- **D.** Redis cases do not count. Embench does not count — it shaped the
  transformations.
- **E.** Refusals stay in the denominator at every reporting level.

## Reporting — fixed now

Project-level, not function hit rate: **useful-patch yield per project**,
**cost-weighted coverage** (share of total measured cost at sites that yielded), and
**saved cost**, each accounting for clustering within a project.

## Prohibited after the fact

- Adding, swapping or dropping an application.
- Revising a prediction.
- Re-tagging a site after its outcome is known.
- **Renaming a winner to preserve the concentration thesis.** If savings appear outside
  `S1`, that is the result and it is reported as such.
- Counting a human-supplied fact as automatic yield.

## Addendum 1 — the baseline compiler configuration

*Appended 2026-09-27, after the freeze above and **before any application was fetched,
built, profiled or measured.** Recorded as an addendum rather than edited in, because it
is an omission in the frozen text and not a revision of it.*

The frozen document did not say which compiler configuration the baseline uses, and the
answer decides whether this audit measures anything. The project's existing 42-corpus
gate feeds the lowerer IR built with `-fno-vectorize -fno-slp-vectorize`, which is why
its families fire at all — and that is precisely the configuration the advisor warned
about: **"restoring performance after synthetic degradation is training, not a product
win."**

Therefore:

- **The baseline is the application's own build with `clang -O3` and default flags,
  auto-vectorization on.** This is what a user actually builds, and it is the baseline
  the claim must beat.
- **ABSAC's intake is the IR that baseline produced.** If the lowerer cannot see a loop
  because clang vectorized it, that is the result of the audit, not a bug in the audit.
- **A secondary diagnostic is recorded separately and is not the product number:**
  ABSAC's outcome on the same sources built with `-fno-vectorize -fno-slp-vectorize`,
  matching the existing gate. It answers "what could we do if the compiler were prevented
  from doing its job", which is a different and weaker question. Any result from it is
  labelled `degraded-baseline` wherever it appears and may not be reported as yield.

This addendum makes a null result more likely and does not make a positive result easier.
It is recorded before the fact for that reason.

## Open risk, stated in advance

The sample is six applications and 24 sites. Astra's W1 cannot be *settled* by n=24 in
either direction — a concentration in `S1` would be non-falsification on a small sample,
not proof of one. The audit can falsify the thesis; it cannot establish it.
