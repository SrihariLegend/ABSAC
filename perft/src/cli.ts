/**
 * CLI argument parsing (commander).
 *
 *   perft --entry 001_redis_bitcount
 *   perft --all
 *   perft --all --iterations 1000000 --verbose
 */
import path from 'node:path';
import { Command, InvalidArgumentError } from 'commander';
import type { CliConfig } from './types.js';

/** Default input seed passed to every generated harness (reproducible runs). */
export const DEFAULT_SEED = 0x9e3779b9;

function positiveInt(value: string): number {
  const n = Number(value);
  if (!Number.isInteger(n) || n <= 0) {
    throw new InvalidArgumentError(`expected a positive integer, got "${value}"`);
  }
  return n;
}

function nonNegativeInt(value: string): number {
  const n = Number(value);
  if (!Number.isInteger(n) || n < 0) {
    throw new InvalidArgumentError(`expected a non-negative integer, got "${value}"`);
  }
  return n;
}

function uint32(value: string): number {
  const n = Number(value);
  if (!Number.isFinite(n) || n < 0 || n > 0xffffffff) {
    throw new InvalidArgumentError(`expected a uint32, got "${value}"`);
  }
  return Math.floor(n);
}

export function parseArgs(argv: string[]): CliConfig {
  const program = new Command();
  program
    .name('perft')
    .description('Benchmark C kernels before/after ABSAC optimization (baseline vs clang vs ABSAC).')
    .option('--entry <id>', 'run a single corpus entry by id')
    .option('--all', 'run every corpus entry under the corpus directory')
    .option('--iterations <n>', 'override the iteration count from bench.json', positiveInt)
    .option('--warmup <n>', 'override the warmup count from bench.json', nonNegativeInt)
    .option('--verbose', 'show per-variant detail (compile/run commands and timings)', false)
    .option('--corpus-dir <dir>', 'corpus directory', path.resolve(process.cwd(), '..', 'corpus'))
    .option('--seed <n>', 'input seed for the generated harness', uint32, DEFAULT_SEED);

  program.exitOverride();

  try {
    program.parse(argv);
  } catch (err) {
    const code = (err as { code?: string }).code;
    if (code === 'commander.helpDisplayed' || code === 'commander.help') process.exit(0);
    if (code === 'commander.unknownOption' || code === 'commander.missingArgument' || code === 'commander.invalidArgument') {
      console.error((err as Error).message);
      process.exit(1);
    }
    throw err;
  }

  const opts = program.opts<{
    entry?: string;
    all?: boolean;
    iterations?: number;
    warmup?: number;
    verbose?: boolean;
    corpusDir: string;
    seed: number;
  }>();

  if (opts.entry && opts.all) {
    console.error('error: --entry and --all are mutually exclusive');
    process.exit(1);
  }
  if (!opts.entry && !opts.all) {
    console.error('error: specify --entry <id> or --all (see --help)');
    process.exit(1);
  }

  return {
    entryId: opts.entry ?? null,
    all: Boolean(opts.all),
    iterations: opts.iterations ?? null,
    warmup: opts.warmup ?? null,
    verbose: Boolean(opts.verbose),
    corpusDir: path.resolve(opts.corpusDir),
    seed: opts.seed,
  };
}
