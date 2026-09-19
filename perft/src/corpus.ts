/**
 * Corpus discovery + loading.
 *
 * A corpus entry is a directory `../corpus/<id>/` containing:
 *   - `source.c`    — the original C kernel
 *   - `NOTES.md`    — human-readable provenance notes (not parsed)
 *   - `bench.json`  — machine-readable benchmark configuration
 */
import fs from 'node:fs';
import path from 'node:path';
import type { BenchConfig, CorpusEntry, CorpusParam } from './types.js';

function isDirectory(p: string): boolean {
  try {
    return fs.statSync(p).isDirectory();
  } catch {
    return false;
  }
}

/** List every corpus entry (a directory that contains a `bench.json`). */
export function listEntries(corpusDir: string): CorpusEntry[] {
  if (!fs.existsSync(corpusDir)) {
    throw new Error(`corpus directory not found: ${corpusDir}`);
  }
  const dirs = fs
    .readdirSync(corpusDir)
    .map((name) => path.join(corpusDir, name))
    .filter(isDirectory)
    .filter((dir) => fs.existsSync(path.join(dir, 'bench.json')))
    .sort();
  return dirs.map(loadEntryFromDir);
}

/** Load a single corpus entry by id. */
export function loadEntry(corpusDir: string, id: string): CorpusEntry {
  const dir = path.join(corpusDir, id);
  if (!fs.existsSync(path.join(dir, 'bench.json'))) {
    throw new Error(`entry "${id}" not found (expected bench.json at ${dir})`);
  }
  return loadEntryFromDir(dir);
}

function loadEntryFromDir(dir: string): CorpusEntry {
  const benchPath = path.join(dir, 'bench.json');
  const sourcePath = path.join(dir, 'source.c');

  let config: BenchConfig;
  try {
    config = JSON.parse(fs.readFileSync(benchPath, 'utf8')) as BenchConfig;
  } catch (err) {
    throw new Error(`invalid bench.json in ${dir}: ${(err as Error).message}`);
  }
  validateConfig(config, dir);

  if (!fs.existsSync(sourcePath)) {
    throw new Error(`missing source.c in ${dir}`);
  }
  const source = fs.readFileSync(sourcePath, 'utf8');

  return {
    id: config.id,
    dir,
    name: config.name,
    function: config.function,
    returnType: config.return_type,
    source,
    config,
  };
}

function validateConfig(config: BenchConfig, dir: string): void {
  const problems: string[] = [];
  if (typeof config.id !== 'string' || config.id.length === 0) problems.push('"id" must be a non-empty string');
  if (typeof config.name !== 'string' || config.name.length === 0) problems.push('"name" must be a non-empty string');
  if (typeof config.function !== 'string' || config.function.length === 0) problems.push('"function" must be a non-empty string');
  if (typeof config.return_type !== 'string') problems.push('"return_type" must be a string');
  if (!Array.isArray(config.params) || config.params.length === 0) problems.push('"params" must be a non-empty array');
  if (!Number.isInteger(config.iterations) || config.iterations <= 0) problems.push('"iterations" must be a positive integer');
  if (!Number.isInteger(config.warmup) || config.warmup < 0) problems.push('"warmup" must be a non-negative integer');
  if (typeof config.absac_emit_cmd !== 'string' || config.absac_emit_cmd.length === 0) problems.push('"absac_emit_cmd" must be a non-empty string');

  if (Array.isArray(config.params)) {
    config.params.forEach((p: CorpusParam, i: number) => {
      if (typeof p.name !== 'string' || p.name.length === 0) problems.push(`params[${i}].name must be a non-empty string`);
      if (typeof p.type !== 'string' || p.type.length === 0) problems.push(`params[${i}].type must be a non-empty string`);
      if (p.kind === 'buffer' && (typeof p.size !== 'number' || p.size <= 0)) problems.push(`params[${i}].size must be a positive number for buffer kind`);
      if (p.kind === 'scalar' && p.value === undefined) problems.push(`params[${i}].value is required for scalar kind`);
      if (p.kind !== 'buffer' && p.kind !== 'scalar') problems.push(`params[${i}].kind must be "buffer" or "scalar"`);
    });
  }

  if (problems.length > 0) {
    throw new Error(`invalid bench.json in ${dir}:\n  - ${problems.join('\n  - ')}`);
  }
}
