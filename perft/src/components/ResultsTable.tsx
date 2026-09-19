/**
 * ResultsTable — renders one bordered table per benchmark run.
 *
 * Row color scheme (chalk):
 *   baseline      → gray
 *   clang variants→ blue
 *   ABSAC         → green when it beats `clang -O3`, red when it loses
 *   errors        → red
 */
import React from 'react';
import { Box, Text } from 'ink';
import chalk from 'chalk';
import type { EntryResult, VariantResult } from '../types.js';
import { renderTable } from '../table.js';
import type { Cell } from '../table.js';

interface Props {
  results: EntryResult[];
}

/** Pick the chalk color for a variant row. */
function colorFor(result: VariantResult, clangO3Ms: number | null): (s: string) => string {
  if (result.status === 'error') return chalk.red;
  if (result.kind === 'baseline') return chalk.gray;
  if (result.kind === 'clang') return chalk.blue;
  // ABSAC: green when it beats (or ties) clang -O3, red when it loses.
  if (clangO3Ms != null && result.timeMs != null) {
    return result.timeMs <= clangO3Ms ? chalk.green : chalk.red;
  }
  return chalk.green;
}

function formatSpeedup(result: VariantResult, baselineMs: number | null): string {
  if (result.status !== 'ok' || result.timeMs == null) return '—';
  if (result.variant === 'baseline') return '1.00x';
  if (baselineMs == null || baselineMs <= 0) return '—';
  return `${(baselineMs / result.timeMs).toFixed(2)}x`;
}

export function ResultsTable({ results }: Props): React.JSX.Element {
  const rows: Cell[][] = [];
  const errors: Array<{ entry: string; variant: string; error: string }> = [];

  for (const entry of results) {
    const clangO3 = entry.results.find((r) => r.variant === 'clang_O3' && r.status === 'ok');
    const clangO3Ms = clangO3?.timeMs ?? null;

    entry.results.forEach((result, index) => {
      const color = colorFor(result, clangO3Ms);
      rows.push([
        // Kernel name shown once per group, empty for continuation rows.
        { text: index === 0 ? entry.entry.name : '' },
        { text: result.label, color },
        {
          text: result.status === 'ok' && result.timeMs != null ? result.timeMs.toFixed(3) : '—',
          color,
          align: 'right',
        },
        { text: formatSpeedup(result, entry.baselineMs), color, align: 'right' },
      ]);

      if (result.status === 'error' && result.error) {
        errors.push({ entry: entry.entry.id, variant: result.label, error: result.error });
      }
    });
  }

  const lines = renderTable(['Kernel', 'Variant', 'Time (ms)', 'Speedup'], rows);

  return (
    <Box flexDirection="column">
      {lines.map((line, i) => (
        <Text key={i}>{line}</Text>
      ))}
      {errors.length > 0 && (
        <Box flexDirection="column" marginTop={1}>
          <Text bold color="red">Errors</Text>
          {errors.map((e, i) => (
            <Box key={i} flexDirection="column" marginTop={1}>
              <Text color="red">
                [{e.entry}] {e.variant}
              </Text>
              {e.error
                .split('\n')
                .slice(0, 6)
                .map((line, j) => (
                  <Text key={j} color="red" dimColor>
                    {'  ' + line}
                  </Text>
                ))}
            </Box>
          ))}
        </Box>
      )}
    </Box>
  );
}

export { chalk };
