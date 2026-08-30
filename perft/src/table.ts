/**
 * Minimal Unicode box-drawing table renderer for the Ink UI.
 *
 * Returns an array of already-styled lines (ANSI color applied via chalk);
 * components map each line to an Ink <Text> element.
 */
import chalk from 'chalk';

export interface Cell {
  text: string;
  /** Optional chalk color function applied to the padded cell. */
  color?: (s: string) => string;
  align?: 'left' | 'right';
}

const BOLD = (s: string): string => chalk.bold(s);

/**
 * Render a bordered table.
 *
 *   ┌────────────┬──────────┐
 *   │ Header     │ Header   │
 *   ├────────────┼──────────┤
 *   │ cell       │     123  │
 *   └────────────┴──────────┘
 */
export function renderTable(headers: string[], rows: Cell[][]): string[] {
  const colCount = headers.length;

  const widths: number[] = headers.map((header, col) => {
    let width = header.length;
    for (const row of rows) {
      const cell = row[col];
      if (cell) width = Math.max(width, cell.text.length);
    }
    return width;
  });

  const pad = (text: string, width: number, align: 'left' | 'right'): string => {
    const gap = Math.max(0, width - text.length);
    return align === 'right' ? ' '.repeat(gap) + text : text + ' '.repeat(gap);
  };

  const renderRow = (cells: Cell[]): string => {
    const parts = cells.map((cell, i) => {
      const align = cell.align ?? 'left';
      const padded = pad(cell.text, widths[i] ?? cell.text.length, align);
      return cell.color ? cell.color(padded) : padded;
    });
    return `│ ${parts.join(' │ ')} │`;
  };

  const border = (left: string, mid: string, right: string): string =>
    left + widths.map((w) => '─'.repeat(w + 2)).join(mid) + right;

  const lines: string[] = [];
  lines.push(border('┌', '┬', '┐'));
  lines.push(renderRow(headers.map((h) => ({ text: h, color: BOLD }))));
  lines.push(border('├', '┼', '┤'));
  for (const row of rows) lines.push(renderRow(row));
  lines.push(border('└', '┴', '┘'));
  return lines;
}

export { chalk };
