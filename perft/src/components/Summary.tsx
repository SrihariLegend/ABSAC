/**
 * Summary — aggregate view across all benchmarked entries:
 * error counts, geometric-mean speedups per variant, and the ABSAC
 * win/loss tally against `clang -O3`.
 */
import React from 'react';
import { Box, Text } from 'ink';
import chalk from 'chalk';
import type { EntryResult } from '../types.js';

interface Props {
  results: EntryResult[];
}

interface VariantStats {
  id: string;
  label: string;
  speedups: number[]; // per-entry speedup vs baseline (only where both succeeded)
  errors: number;
}

function geometricMean(values: number[]): number | null {
  if (values.length === 0) return null;
  const sumLogs = values.reduce((acc, v) => acc + Math.log(v), 0);
  return Math.exp(sumLogs / values.length);
}

export function Summary({ results }: Props): React.JSX.Element {
  const variants = [
    { id: 'clang_O3', label: 'clang -O3', color: chalk.blue },
    { id: 'clang_native', label: 'clang native', color: chalk.blue },
    { id: 'absac', label: 'ABSAC', color: chalk.green },
  ];

  const stats: VariantStats[] = variants.map((v) => ({
    id: v.id,
    label: v.label,
    speedups: [],
    errors: 0,
  }));

  // Every failed variant (including the baseline) counts toward the total.
  const totalErrors = results.reduce(
    (acc, entry) => acc + entry.results.filter((r) => r.status === 'error').length,
    0,
  );
  let absacWins = 0;
  let absacComparable = 0;

  for (const entry of results) {
    const baseline = entry.results.find((r) => r.variant === 'baseline' && r.status === 'ok');
    const baselineMs = baseline?.timeMs ?? null;
    const o3 = entry.results.find((r) => r.variant === 'clang_O3' && r.status === 'ok');
    const absac = entry.results.find((r) => r.variant === 'absac' && r.status === 'ok');

    for (const stat of stats) {
      const result = entry.results.find((r) => r.variant === stat.id);
      if (!result) continue;
      if (result.status === 'error') {
        stat.errors += 1;
      } else if (baselineMs != null && result.timeMs != null && baselineMs > 0) {
        stat.speedups.push(baselineMs / result.timeMs);
      }
    }

    if (o3 && absac) {
      absacComparable += 1;
      if (absac.timeMs! <= o3.timeMs!) absacWins += 1;
    }
  }

  const entries = results.length;

  return (
    <Box flexDirection="column" marginTop={1}>
      <Text bold color="cyan">Summary</Text>
      <Text dimColor>
        {entries} entr{entries === 1 ? 'y' : 'ies'} · {entries * 4} variants ·{' '}
        {totalErrors === 0 ? chalk.green('no errors') : chalk.red(`${totalErrors} errors`)}
      </Text>

      <Box flexDirection="column" marginTop={1}>
        <Text bold>Geometric mean speedup vs baseline -O0</Text>
        {stats.map((stat) => {
          const v = variants.find((v) => v.id === stat.id)!;
          const mean = geometricMean(stat.speedups);
          return (
            <Text key={stat.id}>
              {'  ' + v.color(stat.label.padEnd(14))}{' '}
              {mean != null ? chalk.white(`${mean.toFixed(2)}x`) : chalk.gray('n/a')}
              {stat.errors > 0 ? chalk.red(`  (${stat.errors} error${stat.errors === 1 ? '' : 's'})`) : ''}
            </Text>
          );
        })}
      </Box>

      {absacComparable > 0 && (
        <Box marginTop={1}>
          <Text>
            ABSAC{' '}
            {absacWins === absacComparable
              ? chalk.green('beats')
              : absacWins === 0
                ? chalk.red('loses to')
                : chalk.yellow('beats')}{' '}
            clang -O3 on {chalk.white(`${absacWins}/${absacComparable}`)} comparable entr
            {absacComparable === 1 ? 'y' : 'ies'}
          </Text>
        </Box>
      )}
    </Box>
  );
}
