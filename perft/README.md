# perft

A minimal TypeScript + [Ink](https://github.com/vadimdemedes/ink) CLI that
benchmarks C kernels before and after ABSAC optimization. It compiles each
kernel at several optimization levels, times the kernel *inside* the generated
C binary (using `clock_gettime(CLOCK_MONOTONIC)`), and renders a colored
results table.

```
┌──────────────────────────┬──────────────┬───────────┬─────────┐
│ Kernel                   │ Variant      │ Time (ms) │ Speedup │
├──────────────────────────┼──────────────┼───────────┼─────────┤
│ Redis BITCOUNT tail loop │ baseline -O0 │   114.23  │   1.00x │
│                          │ clang -O3    │    15.96  │   7.16x │
│                          │ clang native │    15.99  │   7.15x │
│                          │ ABSAC        │    64.90  │   1.76x │
└──────────────────────────┴──────────────┴───────────┴─────────┘
```

## Requirements

- Node.js ≥ 20
- `clang` on `PATH` (for the kernel compilation step)
- `cargo` on `PATH` if any corpus entry uses an `absac_emit_cmd` that invokes it

## Install & build

```bash
cd perft
npm install
npm run build
```

## Usage

```bash
# one corpus entry
node dist/index.js --entry 001_redis_bitcount

# every corpus entry under ../corpus
node dist/index.js --all

# override iterations, override warmup, show per-variant detail
node dist/index.js --all --iterations 1000000 --warmup 100 --verbose

# custom corpus directory
node dist/index.js --all --corpus-dir /path/to/corpus
```

Options:

| Flag | Description |
|------|-------------|
| `--entry <id>` | Run a single corpus entry by id |
| `--all` | Run every corpus entry under the corpus directory |
| `--iterations <n>` | Override the iteration count from `bench.json` |
| `--warmup <n>` | Override the warmup count from `bench.json` |
| `--verbose` | Show per-variant detail (emit/compile/run commands and timings) |
| `--corpus-dir <dir>` | Corpus directory (default: `../corpus`) |
| `--seed <n>` | Input seed for the generated harness (default: `0x9e3779b9`) |

Press `q` (or `Ctrl-C`) to quit early.

## Corpus format

Each entry lives in `../corpus/<id>/`:

- `source.c` — the original C kernel
- `NOTES.md` — human-readable provenance notes (not parsed)
- `bench.json` — machine-readable configuration:

```json
{
  "id": "001_redis_bitcount",
  "name": "Redis BITCOUNT tail loop",
  "function": "redis_bitcount_tail",
  "return_type": "long long",
  "params": [
    { "name": "buf", "type": "const uint8_t *", "kind": "buffer", "size": 4096 },
    { "name": "count", "type": "long", "kind": "scalar", "value": 4096 }
  ],
  "iterations": 100000,
  "warmup": 1000,
  "absac_emit_cmd": "cargo run -p sir_benchmarks --bin emit_c -- 001_redis_bitcount"
}
```

`params` support two kinds:

- `buffer` — declared as `TYPE name[size]`, filled with a deterministic
  pattern derived from the run seed
- `scalar` — declared as `TYPE name = value`

The `absac_emit_cmd` is executed (from the `sir/` workspace directory) and its
stdout is treated as the rewritten C.

## What happens per entry

For each entry the harness produces four variants:

| Variant | Source | clang flags |
|---------|--------|-------------|
| `baseline` | `source.c` | `-O0` |
| `clang_O3` | `source.c` | `-O3` |
| `clang_native` | `source.c` | `-O3 -march=native` |
| `absac` | stdout of `absac_emit_cmd` | `-O2` |

Each variant is compiled as a standalone C file: the kernel followed by an
auto-generated `main()` that builds the inputs from `bench.json`, warms up,
times `iterations` calls, and prints the elapsed seconds to stdout (just the
number). Two anti-optimization guards keep the measurement honest:

1. The input seed is read from `argv` at runtime, so the compiler cannot
   constant-fold the kernel at `-O3`.
2. The accumulated result is printed to *stderr* as a checksum, so the timed
   loop cannot be dead-code-eliminated.

Compilation and run failures are handled gracefully: the failing variant is
shown in red with its diagnostics, and the remaining variants still run.

## Colors

| Variant | Color | Meaning |
|---------|-------|---------|
| baseline | gray | reference (`-O0`) |
| clang variants | blue | hand-written compiler optimization |
| ABSAC | green | beats `clang -O3` |
| ABSAC | red | loses to `clang -O3`, or errored |

## Project layout

```
perft/
  src/
    index.tsx            Ink app entry (renders table + summary)
    cli.ts               commander argument parsing
    compile.ts           C harness generation + clang + absac emit
    benchmark.ts         run orchestration + binary timing
    corpus.ts            corpus discovery + bench.json parsing
    table.ts             Unicode box-drawing table renderer
    components/
      ResultsTable.tsx   per-variant results table
      Summary.tsx        aggregate speedups + ABSAC win/loss tally
```
