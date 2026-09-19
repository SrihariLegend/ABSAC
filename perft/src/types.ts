/**
 * Shared types for the ABSAC benchmark harness.
 */

/** The four variants every corpus entry is compiled into. */
export type VariantId = 'baseline' | 'clang_O3' | 'clang_native' | 'absac';

/** Variant family — drives both compilation and display color. */
export type VariantKind = 'baseline' | 'clang' | 'absac';

/** Static description of a variant (compilation flags + source origin). */
export interface VariantDef {
  id: VariantId;
  kind: VariantKind;
  label: string;
  clangFlags: string[];
  /** Where the kernel source comes from. */
  source: 'source' | 'absac';
}

/** The four variants, in display order. */
export const VARIANTS: VariantDef[] = [
  { id: 'baseline', kind: 'baseline', label: 'baseline -O0', clangFlags: ['-O0'], source: 'source' },
  { id: 'clang_O3', kind: 'clang', label: 'clang -O3', clangFlags: ['-O3'], source: 'source' },
  { id: 'clang_native', kind: 'clang', label: 'clang native', clangFlags: ['-O3', '-march=native'], source: 'source' },
  { id: 'absac', kind: 'absac', label: 'ABSAC', clangFlags: ['-O2'], source: 'absac' },
];

/** A single benchmark parameter from `bench.json`. */
export interface CorpusParam {
  name: string;
  type: string;
  kind: 'buffer' | 'scalar';
  /** Buffer params: number of elements. */
  size?: number;
  /** Scalar params: initial value. */
  value?: number | string;
}

/** The parsed `bench.json` of a corpus entry. */
export interface BenchConfig {
  id: string;
  name: string;
  function: string;
  return_type: string;
  params: CorpusParam[];
  iterations: number;
  warmup: number;
  absac_emit_cmd: string;
}

/** A fully loaded corpus entry. */
export interface CorpusEntry {
  id: string;
  dir: string;
  name: string;
  function: string;
  returnType: string;
  source: string;
  config: BenchConfig;
}

/** Result of compiling + running a single variant. */
export interface VariantResult {
  variant: VariantId;
  kind: VariantKind;
  label: string;
  status: 'ok' | 'error';
  /** Elapsed wall-clock time in milliseconds, or null on failure. */
  timeMs: number | null;
  error: string | null;
  /** Raw stdout captured from the benchmark binary. */
  stdout: string | null;
  /** Raw stderr captured from the benchmark binary. */
  stderr: string | null;
}

/** All variant results for one corpus entry. */
export interface EntryResult {
  entry: CorpusEntry;
  results: VariantResult[];
  /** Baseline time in ms (null when the baseline itself failed). */
  baselineMs: number | null;
}

/** Parsed CLI configuration. */
export interface CliConfig {
  entryId: string | null;
  all: boolean;
  iterations: number | null;
  warmup: number | null;
  verbose: boolean;
  corpusDir: string;
  seed: number;
}
