/**
 * Benchmark orchestration: for a corpus entry, produce all four variants,
 * compile them, run each binary, and parse the reported elapsed time.
 *
 * The actual kernel timing happens inside the generated C harness
 * (`clock_gettime`); the CLI only measures wall-clock for display/status.
 */
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { compileVariant, formatExecError, generateHarnessSource, resolveSirDir, runAbsacEmit } from './compile.js';
import type { CliConfig, CorpusEntry, EntryResult, VariantDef, VariantResult } from './types.js';
import { VARIANTS } from './types.js';

/** Progress + logging hooks consumed by the Ink UI. */
export interface RunHooks {
  onProgress: (message: string) => void;
  onLog: (line: string) => void;
}

/** Yield to the event loop so the TUI can repaint between blocking steps. */
const nextTick = (): Promise<void> => new Promise((resolve) => setImmediate(resolve));

/** Run every variant for a single corpus entry. */
export async function runEntry(
  entry: CorpusEntry,
  config: CliConfig,
  hooks: RunHooks,
  workDir: string,
): Promise<EntryResult> {
  const iterations = config.iterations ?? entry.config.iterations;
  const warmup = config.warmup ?? entry.config.warmup;
  const results: VariantResult[] = [];

  for (const variant of VARIANTS) {
    hooks.onProgress(`[${entry.id}] ${variant.label}: preparing`);
    await nextTick();

    // ── Obtain the kernel source for this variant ─────────────
    let kernelSource: string;
    if (variant.source === 'absac') {
      hooks.onLog(`[${entry.id}] ${variant.label}: emit (${entry.config.absac_emit_cmd})`);
      const emit = runAbsacEmit(entry.config.absac_emit_cmd, resolveSirDir(config.corpusDir));
      if (!emit.ok) {
        results.push(errorResult(variant, `ABSAC emit failed:\n${emit.error}`));
        hooks.onLog(`  EMIT ERROR\n${emit.error ?? ''}`);
        continue;
      }
      kernelSource = emit.source!;
    } else {
      kernelSource = entry.source;
    }

    // ── Generate + compile the harness ────────────────────────
    const harness = generateHarnessSource(entry, kernelSource, iterations, warmup);
    const variantDir = path.join(workDir, entry.id, variant.id);
    const compiled = compileVariant({ entry, variant, workDir: variantDir, source: harness });
    hooks.onLog(`[${entry.id}] ${variant.label}: ${compiled.compileCmd ?? ''}`);

    if (!compiled.ok) {
      results.push(errorResult(variant, `compile failed:\n${compiled.error}`));
      hooks.onLog(`  COMPILE ERROR\n${compiled.error ?? ''}`);
      continue;
    }

    // ── Run the binary and parse timing ───────────────────────
    hooks.onProgress(`[${entry.id}] ${variant.label}: running (${iterations} iters)`);
    await nextTick();
    const run = runBinary(compiled.binaryPath!, variant, config.seed);
    if (run.status === 'ok') {
      hooks.onLog(`[${entry.id}] ${variant.label}: ${run.timeMs!.toFixed(3)} ms`);
    } else {
      hooks.onLog(`[${entry.id}] ${variant.label}: ${compiled.binaryPath!}\n  RUN ERROR\n${run.error ?? ''}`);
    }
    results.push(run);
  }

  const baseline = results.find((r) => r.variant === 'baseline' && r.status === 'ok');
  return { entry, results, baselineMs: baseline?.timeMs ?? null };
}

/** Execute a compiled benchmark binary and parse its stdout (seconds → ms). */
export function runBinary(binaryPath: string, variant: VariantDef, seed: number): VariantResult {
  try {
    const stdout = execSync(`"${binaryPath}" ${seed}`, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 5 * 60_000,
      maxBuffer: 1024 * 1024,
    });
    const parsed = Number.parseFloat(stdout.trim());
    if (!Number.isFinite(parsed)) {
      throw new Error(`unparseable timing output: ${JSON.stringify(stdout.trim())}`);
    }
    return {
      variant: variant.id,
      kind: variant.kind,
      label: variant.label,
      status: 'ok',
      timeMs: parsed * 1000,
      error: null,
      stdout,
      stderr: null,
    };
  } catch (err) {
    return {
      variant: variant.id,
      kind: variant.kind,
      label: variant.label,
      status: 'error',
      timeMs: null,
      error: formatExecError(err),
      stdout: null,
      stderr: decode(err, 'stderr'),
    };
  }
}

function errorResult(variant: VariantDef, error: string): VariantResult {
  return {
    variant: variant.id,
    kind: variant.kind,
    label: variant.label,
    status: 'error',
    timeMs: null,
    error,
    stdout: null,
    stderr: null,
  };
}

function decode(err: unknown, channel: 'stdout' | 'stderr'): string | null {
  const e = err as { [k: string]: unknown };
  const buf = e[channel];
  if (Buffer.isBuffer(buf)) return buf.toString('utf8');
  return typeof buf === 'string' ? buf : null;
}

/** Create a fresh working directory for a benchmark run. */
export function createWorkDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'perft-'));
}
