/**
 * Ink app entry point.
 *
 * Parses CLI args, runs the benchmark pipeline (corpus → compile → run),
 * and renders a results table + summary in a terminal UI.
 */
import React, { useEffect, useRef, useState } from 'react';
import { Box, Text, render, useApp, useInput, useStdin } from 'ink';
import { parseArgs } from './cli.js';
import { listEntries, loadEntry } from './corpus.js';
import { createWorkDir, runEntry } from './benchmark.js';
import type { CliConfig, EntryResult } from './types.js';
import { ResultsTable } from './components/ResultsTable.js';
import { Summary } from './components/Summary.js';

const config = parseArgs(process.argv);

/* ------------------------------------------------------------------ */
/* Tiny spinner (Ink 7 no longer ships one)                            */
/* ------------------------------------------------------------------ */

const FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

function Spinner(): React.JSX.Element {
  const [frame, setFrame] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setFrame((f) => (f + 1) % FRAMES.length), 80);
    return () => clearInterval(id);
  }, []);
  return <Text color="cyan">{FRAMES[frame]}</Text>;
}

function VerboseLog({ log }: { log: string[] }): React.JSX.Element {
  const tail = log.slice(-40);
  return (
    <Box flexDirection="column" marginTop={1}>
      <Text bold dimColor>Verbose log</Text>
      {tail.map((line, i) => (
        <Text key={i} dimColor>
          {line}
        </Text>
      ))}
    </Box>
  );
}

/* ------------------------------------------------------------------ */
/* App                                                                 */
/* ------------------------------------------------------------------ */

type Phase = 'running' | 'done' | 'error';

function App({ config }: { config: CliConfig }): React.JSX.Element {
  const { exit } = useApp();
  const { isRawModeSupported } = useStdin();
  const rawModeOk = isRawModeSupported === true;
  const [phase, setPhase] = useState<Phase>('running');
  const [progress, setProgress] = useState('discovering corpus entries…');
  const [log, setLog] = useState<string[]>([]);
  const [results, setResults] = useState<EntryResult[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [elapsedMs, setElapsedMs] = useState<number | null>(null);
  const startRef = useRef(0n);

  // Allow `q` to quit, but only when stdin actually supports raw mode
  // (piped/non-TTY stdin would otherwise make Ink throw).
  useInput((input) => {
    if (input === 'q') exit();
  }, { isActive: rawModeOk });

  useEffect(() => {
    let cancelled = false;

    const pushLog = (line: string): void => {
      if (!cancelled) setLog((prev) => [...prev, ...line.split('\n').filter(Boolean)]);
    };

    void (async () => {
      try {
        startRef.current = process.hrtime.bigint();
        const entries = config.all
          ? listEntries(config.corpusDir)
          : [loadEntry(config.corpusDir, config.entryId!)];
        if (entries.length === 0) throw new Error('no corpus entries found');

        const workDir = createWorkDir();
        const collected: EntryResult[] = [];
        for (const entry of entries) {
          const result = await runEntry(
            entry,
            config,
            {
              onProgress: (m) => {
                if (!cancelled) setProgress(m);
              },
              onLog: pushLog,
            },
            workDir,
          );
          collected.push(result);
          if (!cancelled) setResults([...collected]);
        }
        if (!cancelled) {
          setElapsedMs(Number(process.hrtime.bigint() - startRef.current) / 1e6);
          setResults(collected);
          setPhase('done');
        }
      } catch (e) {
        if (!cancelled) {
          setError((e as Error).message);
          setPhase('error');
        }
      }
    })();

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Leave the final frame on screen briefly, then exit cleanly.
  useEffect(() => {
    if (phase === 'done' || phase === 'error') {
      const t = setTimeout(() => exit(), config.verbose ? 1200 : 350);
      return () => clearTimeout(t);
    }
  }, [phase, exit, config.verbose]);

  if (phase === 'error') {
    return (
      <Box flexDirection="column">
        <Text bold color="red">
          Error
        </Text>
        <Text color="red">{error}</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column">
      <Text bold color="cyan">
        perft
      </Text>
      <Text dimColor>
        corpus {config.corpusDir} · seed 0x{config.seed.toString(16)}
        {config.iterations ? ` · iterations ${config.iterations}` : ''}
      </Text>

      <Box marginTop={1}>
        {phase === 'running' ? (
          <>
            <Spinner />
            <Text> {progress}</Text>
          </>
        ) : (
          <Text color="green">
            ✓ benchmark complete{elapsedMs != null ? ` in ${(elapsedMs / 1000).toFixed(2)}s` : ''}
          </Text>
        )}
      </Box>

      {results.length > 0 && (
        <Box marginTop={1} flexDirection="column">
          <ResultsTable results={results} />
        </Box>
      )}

      {phase === 'done' && <Summary results={results} />}

      {config.verbose && log.length > 0 && <VerboseLog log={log} />}
    </Box>
  );
}

render(<App config={config} />);
