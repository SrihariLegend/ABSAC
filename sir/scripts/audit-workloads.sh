#!/usr/bin/env bash
# Opportunity audit — generate the six frozen workloads.
#
# Every input is generated deterministically from the frozen tree, so a rerun on
# the same commit reproduces byte-identical inputs. Hashes are written to
# $AUDIT/workloads.sha256. The workload commands themselves are in audit-run.sh.
#
#   scripts/audit-workloads.sh [DEST]     # default /tmp/audit
set -euo pipefail

A="${1:-/tmp/audit}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
W="$A/work"; mkdir -p "$W"

# ---- A1/A2 text corpus: the frozen tree's own text, repeated to 64 MiB ------
# Real source text, deterministic from the commit, and large enough that a scan
# is ~100ms rather than lost in process startup.
CORPUS="$W/corpus.txt"
if [ ! -f "$CORPUS" ]; then
    tmp=$(mktemp -d)
    # Two git dirs live in this tree and plain `git` reads the abandoned one,
    # so the real history must be named explicitly.
    git --git-dir="$ROOT/recovery.git" archive --format=tar HEAD | tar -xf - -C "$tmp"
    cat "$tmp"/*.md "$tmp"/docs/*.md "$tmp"/sir/crates/*/src/*.rs \
        "$tmp"/sir/crates/*/tests/*.rs "$tmp"/sir/crates/*/tests/cases/*.rs \
        "$tmp"/sir/scripts/*.sh "$tmp"/gate6a/*.ll 2>/dev/null > "$W/base.txt"
    : > "$CORPUS"
    while [ "$(stat -c%s "$CORPUS")" -lt 67108864 ]; do cat "$W/base.txt" >> "$CORPUS"; done
    rm -rf "$tmp"
fi

# ---- A3 JSON: 200k records, generated from a fixed LCG ----------------------
JQ_IN="$W/records.json"
if [ ! -f "$JQ_IN" ]; then
    python3 - "$JQ_IN" <<'PY'
import sys, json
s = 0x2545F4914F6CDD1D
def nxt():
    global s
    s = (s * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)
    return s >> 33
recs = [{"k": nxt() % 1000, "v": nxt() % 100000, "t": "tag%d" % (nxt() % 16),
         "f": (nxt() % 10000) / 100.0} for _ in range(200000)]
with open(sys.argv[1], "w") as fh:
    json.dump(recs, fh)
PY
fi

# ---- A4 SQL: 1M rows from a recursive CTE, then index and query -------------
SQL="$W/bench.sql"
if [ ! -f "$SQL" ]; then
    cat > "$SQL" <<'SQL'
PRAGMA journal_mode = OFF;
PRAGMA synchronous = OFF;
CREATE TABLE t(a INTEGER, b INTEGER, c TEXT);
INSERT INTO t SELECT i, i*7919 % 1000003, 'row' || i % 97
  FROM (WITH RECURSIVE s(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM s WHERE i < 1000000)
        SELECT i FROM s);
CREATE INDEX idx_a ON t(a);
CREATE INDEX idx_b ON t(b);
SELECT count(*), sum(b) FROM t WHERE a BETWEEN 100000 AND 900000;
SELECT c, count(*) FROM t GROUP BY c ORDER BY 2 DESC LIMIT 5;
SELECT count(*) FROM t t1 JOIN t t2 ON t1.a = t2.b WHERE t1.a < 50000;
SQL
fi

# ---- A5 PNG: 2048x2048 RGB, gradient plus fixed-seed noise ------------------
PNG="$W/image.png"
if [ ! -f "$PNG" ]; then
    python3 - "$PNG" <<'PY'
import sys, zlib, struct
Wd = Ht = 2048
s = 0x9E3779B97F4A7C15
rows = bytearray()
for y in range(Ht):
    rows.append(0)
    for x in range(Wd):
        s = (s * 6364136223846793005 + 1442695040888963407) & ((1 << 64) - 1)
        n = (s >> 40) & 0xFF
        rows += bytes(((x * 255) // Wd, (y * 255) // Ht, n))
def chunk(t, d):
    c = t + d
    return struct.pack(">I", len(d)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)
out = b"\x89PNG\r\n\x1a\n"
out += chunk(b"IHDR", struct.pack(">IIBBBBB", Wd, Ht, 8, 2, 0, 0, 0))
out += chunk(b"IDAT", zlib.compress(bytes(rows), 6))
out += chunk(b"IEND", b"")
open(sys.argv[1], "wb").write(out)
PY
fi

# ---- A6 Lua: a fixed mixed workload -----------------------------------------
LUA="$W/bench.lua"
if [ ! -f "$LUA" ]; then
    cat > "$LUA" <<'LUA'
-- floating point, integer arithmetic, table churn and string work
local function fmath(n)
  local a, b = 1.0, 0.5
  for i = 1, n do a = a * 1.0000001 + b * 0.9999999; b = b * 0.9999999 + 0.0000001 end
  return a + b
end
local function iarith(n)
  local acc = 0
  for i = 1, n do acc = (acc + i * 2654435761) % 4294967296 end
  return acc
end
local function tables(n)
  local t = {}
  for i = 1, n do t[i % 4096] = i end
  local s = 0
  for k, v in pairs(t) do s = s + v end
  return s
end
local function strings(n)
  local parts = {}
  for i = 1, n do parts[#parts + 1] = tostring(i % 1000) end
  return #table.concat(parts, ",")
end
local total = 0
for _ = 1, 8 do
  total = total + fmath(2000000) + iarith(2000000) + tables(200000) + strings(20000)
end
print(total)
LUA
fi

sha256sum "$CORPUS" "$JQ_IN" "$SQL" "$PNG" "$LUA" > "$A/workloads.sha256"
cat "$A/workloads.sha256"
