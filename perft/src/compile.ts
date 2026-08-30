/**
 * C code generation + clang compilation + ABSAC emit-command invocation.
 *
 * The CLI synthesizes a standalone C benchmark file: the kernel (either the
 * original `source.c` or the C emitted by ABSAC) followed by an auto-generated
 * `main()` that builds the inputs from the `bench.json` params, warms up,
 * times `iterations` calls with `clock_gettime(CLOCK_MONOTONIC)`, and prints
 * the elapsed seconds to stdout.
 */
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import type { CorpusEntry, CorpusParam, VariantDef } from './types.js';

/* ------------------------------------------------------------------ */
/* Helpers                                                             */
/* ------------------------------------------------------------------ */

/** Extract the array element type from a pointer type like `const uint8_t *`. */
function bufferElementType(type: string): string {
  let t = type.replace(/\bconst\b/g, '').replace(/\*/g, '').trim();
  if (t.length === 0 || t === 'void') t = 'unsigned char';
  return t;
}

/** Render a scalar param's initial value as a C literal/expression. */
function scalarValue(param: CorpusParam): string {
  if (typeof param.value === 'number') return String(param.value);
  if (typeof param.value === 'string') return param.value;
  return '0';
}

/** True when the return type can be accumulated with `+=` in the timing loop. */
function isAccumulable(returnType: string): boolean {
  const t = returnType.trim();
  if (t === 'void') return false;
  if (t.includes('*')) return false;
  if (/^(struct|union|enum)\b/.test(t)) return false;
  return true;
}

/** One declared + initialized param inside the generated `main()`. */
interface ParamCode {
  decl: string;
  init: string | null;
}

function buildParamCode(param: CorpusParam): ParamCode {
  if (param.kind === 'buffer') {
    const elem = bufferElementType(param.type);
    const size = Math.max(1, Math.floor(param.size ?? 256));
    return {
      decl: `${elem} ${param.name}[${size}];`,
      init: `for (int i = 0; i < ${size}; i++) ${param.name}[i] = (${elem})(seed + (uint32_t)(i * 31 + 17));`,
    };
  }
  return { decl: `${param.type} ${param.name} = ${scalarValue(param)};`, init: null };
}

/* ------------------------------------------------------------------ */
/* Harness generation                                                  */
/* ------------------------------------------------------------------ */

/**
 * Generate the full C source for one variant: the kernel source followed by
 * a benchmark `main()` synthesized from the entry's `bench.json` params.
 */
export function generateHarnessSource(
  entry: CorpusEntry,
  kernelSource: string,
  iterations: number,
  warmup: number,
): string {
  const cfg = entry.config;
  const params = cfg.params.map(buildParamCode);
  const callArgs = cfg.params.map((p) => p.name).join(', ');
  const call = `${cfg.function}(${callArgs})`;
  const accumulable = isAccumulable(cfg.return_type);

  const lines: string[] = [
    '/* expose clock_gettime under strict -std=c11 (glibc feature test) */',
    '#define _POSIX_C_SOURCE 199309L',
    '',
    '#include <stdint.h>',
    '#include <time.h>',
    '#include <stdio.h>',
    '#include <stdlib.h>',
    '',
    kernelSource.trimEnd(),
    '',
    'int main(int argc, char **argv) {',
    '    /* seed read at runtime so the optimizer cannot constant-fold the kernel */',
    '    uint32_t seed = (argc > 1) ? (uint32_t)strtoul(argv[1], NULL, 0) : 0x9e3779b9u;',
    '',
    '    /* ---- input data (generated from bench.json params) ---- */',
  ];

  for (const p of params) {
    lines.push(`    ${p.decl}`);
    if (p.init) lines.push(`    ${p.init}`);
  }

  lines.push('', `    /* ---- warmup (${warmup} iterations) ---- */`);
  if (accumulable) {
    lines.push(`    for (int i = 0; i < ${warmup}; i++) {`, `        volatile ${cfg.return_type} r = ${call};`, '        (void)r;', '    }');
  } else {
    lines.push(`    for (int i = 0; i < ${warmup}; i++) { (void)${call}; }`);
  }

  lines.push('', '    /* ---- benchmark (timed with CLOCK_MONOTONIC) ---- */', '    struct timespec start, end;', '    clock_gettime(CLOCK_MONOTONIC, &start);');
  if (accumulable) {
    lines.push(`    ${cfg.return_type} total = 0;`, `    for (int i = 0; i < ${iterations}; i++) {`, `        total += ${call};`, '    }');
  } else {
    lines.push(`    for (int i = 0; i < ${iterations}; i++) { (void)${call}; }`);
  }
  lines.push('    clock_gettime(CLOCK_MONOTONIC, &end);');

  lines.push('', '    double elapsed = (double)(end.tv_sec - start.tv_sec) + (double)(end.tv_nsec - start.tv_nsec) / 1e9;', '    printf("%.9f\\n", elapsed);');
  if (accumulable) {
    // Keep the accumulated result observable (stderr, not stdout) so the
    // optimizer cannot dead-code-eliminate the timed loop.
    lines.push('    fprintf(stderr, "checksum: %lld\\n", (long long)total);');
  } else {
    lines.push('    fprintf(stderr, "checksum: ok\\n");');
  }
  lines.push('    return 0;', '}', '');

  return lines.join('\n');
}

/* ------------------------------------------------------------------ */
/* ABSAC emit command                                                  */
/* ------------------------------------------------------------------ */

export interface EmitResult {
  ok: boolean;
  source?: string;
  error?: string;
}

/**
 * Run the entry's `absac_emit_cmd` (e.g. `cargo run -p sir_benchmarks …`)
 * and capture the rewritten C from stdout. Runs from the `sir/` workspace
 * directory so `cargo` finds the workspace.
 */
export function runAbsacEmit(cmd: string, cwd: string): EmitResult {
  try {
    const stdout = execSync(cmd, {
      cwd,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 5 * 60_000,
      maxBuffer: 64 * 1024 * 1024,
    });
    return { ok: true, source: stdout };
  } catch (err) {
    return { ok: false, error: formatExecError(err) };
  }
}

/* ------------------------------------------------------------------ */
/* clang compilation                                                   */
/* ------------------------------------------------------------------ */

export interface CompileResult {
  ok: boolean;
  binaryPath?: string;
  compileCmd?: string;
  error?: string;
}

export interface CompileInput {
  entry: CorpusEntry;
  variant: VariantDef;
  workDir: string;
  source: string;
}

/** Write the harness to `workDir/<variant>.c` and compile it with clang. */
export function compileVariant(input: CompileInput): CompileResult {
  const { variant, workDir, source } = input;
  fs.mkdirSync(workDir, { recursive: true });

  const srcPath = path.join(workDir, `${variant.id}.c`);
  const binPath = path.join(workDir, variant.id);
  fs.writeFileSync(srcPath, source);

  const args = ['-std=c11', ...variant.clangFlags, '-o', binPath, srcPath];
  const compileCmd = ['clang', ...args].map(shellQuote).join(' ');

  try {
    execSync(compileCmd, { stdio: 'pipe', timeout: 5 * 60_000, maxBuffer: 64 * 1024 * 1024 });
    return { ok: true, binaryPath: binPath, compileCmd };
  } catch (err) {
    return { ok: false, compileCmd, error: formatExecError(err) };
  }
}

/* ------------------------------------------------------------------ */
/* Error formatting + misc                                             */
/* ------------------------------------------------------------------ */

/** Turn an `execSync` thrown error into a concise, readable message. */
export function formatExecError(err: unknown): string {
  const e = err as {
    message?: string;
    stdout?: Buffer | string;
    stderr?: Buffer | string;
    status?: number | null;
    signal?: string | null;
  };
  const stderr = decode(e.stderr).trim();
  const stdout = decode(e.stdout).trim();
  const parts: string[] = [];
  if (stderr) parts.push(stderr);
  else if (stdout) parts.push(stdout);
  if (e.status != null) parts.push(`exit status ${e.status}`);
  else if (e.signal) parts.push(`killed by signal ${e.signal}`);
  if (parts.length === 0) parts.push(e.message ?? String(err));
  return parts.join('\n');
}

function decode(buf: Buffer | string | undefined): string {
  if (!buf) return '';
  return Buffer.isBuffer(buf) ? buf.toString('utf8') : buf;
}

/** Quote a token for a POSIX shell command line. */
function shellQuote(token: string): string {
  if (/^[a-zA-Z0-9_./=+:-]+$/.test(token)) return token;
  return `'${token.replace(/'/g, `'\\''`)}'`;
}

/** Resolve the directory in which to run `cargo` (the `sir/` workspace). */
export function resolveSirDir(corpusDir: string): string {
  const root = path.resolve(path.dirname(corpusDir));
  const sir = path.join(root, 'sir');
  return fs.existsSync(path.join(sir, 'Cargo.toml')) ? sir : root;
}
