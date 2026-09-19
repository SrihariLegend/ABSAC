#!/bin/bash
# Gate 6A-v2 — Blind evaluation archive (Generation H2)
# Corpus classified + hashed BEFORE the frozen candidate runs.
# Candidate C2 = commit 5355778.

set -e
cd "$(dirname "$0")"

echo "═══════════════════════════════════════════════════════════"
echo "Gate 6A-v2 — Blind Evaluation Archive (Generation H2)"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Frozen candidate: $(git rev-parse HEAD)"
echo ""
echo "── SHA256 (frozen before the run) ──"
sha256sum v2_corpus.c v2_corpus.ll
echo ""

cat << 'EOF'
── Expected classification (frozen BEFORE the run) ──

ID   Class     Category                       Key property
─────────────────────────────────────────────────────────────────
W01  POSITIVE  Cardinality i16, zext count
W02  POSITIVE  Sum u32→u64
W03  POSITIVE  Sum of map (buf[i]+1)          transformed element
W04  POSITIVE  All i8, select-reset, signed
W05  POSITIVE  Cardinality Gt zero, signed
W06  POSITIVE  Cardinality Ne, constant bound
W07  POSITIVE  All u8, Gt predicate, u8 acc
W08  POSITIVE  Two loops (Cardinality+Sum)    tracks two-loop coverage gap

X01  NEGATIVE  volatile STORE in loop          effects gap
X02  NEGATIVE  signed i32 sum (nsw risk)       overflow semantics gap
X03  NEGATIVE  sliding window buf[i]+buf[i-1]  two access functions, one base
X04  NEGATIVE  fixed stride 4                  non-unit constant stride
X05  NEGATIVE  C11 atomic loads (acquire)      atomic ordering
X06  NEGATIVE  running max lookalike           not a monoid reduction
X07  NEGATIVE  self-referential recurrence     acc used in own update
X08  NEGATIVE  early exit + global side effect non-local state write
EOF
echo ""
clang -O1 -emit-llvm -S v2_corpus.c -o v2_corpus.ll
echo "── v2_corpus.ll hash (frozen) ──"
sha256sum v2_corpus.ll
echo ""
echo "Archive complete: $(date -Iseconds)"
echo "Frozen candidate: 5355778 (C2). Run ONCE, freeze results, then inspect."
