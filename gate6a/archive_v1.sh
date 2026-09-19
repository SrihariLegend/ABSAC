#!/bin/bash
# Gate 6A-v1 — Blind evaluation archive
# This script archives the corpus classification and hashes BEFORE
# running the frozen candidate. Run once, then freeze results.

set -e
cd "$(dirname "$0")"

echo "═══════════════════════════════════════════════════════════"
echo "Gate 6A-v1 — Blind Evaluation Archive"
echo "═══════════════════════════════════════════════════════════"
echo ""
echo "Frozen candidate commit: $(git rev-parse HEAD)"
echo "Corpus file: gate6a/v1_corpus.c"
echo ""
echo "── SHA256 hashes (frozen before the run) ──"
sha256sum v1_corpus.c
echo ""

echo "── Expected classification (frozen BEFORE the run) ──"
cat << 'EOF'
ID    Class     Expected behavior
─────────────────────────────────────────────────────────────────
V01   POSITIVE  Cardinality, do-while, signed acc, i64 induction
V02   POSITIVE  Sum, u16 elements, u32 accumulator/induction
V03   POSITIVE  All ( ConjunctiveReduction), i64 elements, & form
V04   POSITIVE  Cardinality, Gt predicate, u32 elements
V05   POSITIVE  Sum, signed char elements, i64 accumulator
V06   POSITIVE  Cardinality, != predicate, while (i != n) form
V07   POSITIVE  Cardinality + Sum in two separate loops (fusion shape)
V08   POSITIVE  Sum, non-zero loop start (skip parameter)
N09   NEGATIVE  volatile loads (atomic-like ordering)
N10   NEGATIVE  aliasing store inside loop (memory dependence)
N11   NEGATIVE  reverse traversal (decrementing induction)
N12   NEGATIVE  opaque call inside loop (unknown side effects)
N13   NEGATIVE  dynamic stride (runtime step parameter)
N14   NEGATIVE  conditional/saturating accumulator (not a monoid add)
N15   NEGATIVE  two-array relation (a[i]==b[i]) — recognizer has no
                two-input reduction concept; must NOT fire as
                CardinalityReduction on a alone
N16   NEGATIVE  signed i8→i32 sum with signed overflow semantics
EOF
echo ""

echo "── Compile to LLVM IR (preserving structure) ──"
clang -O1 -emit-llvm -S v1_corpus.c -o v1_corpus.ll
sha256sum v1_corpus.ll
echo ""
echo "Archive complete. Frozen: $(date -Iseconds)"
echo ""
echo "NEXT: run the frozen system ONCE, freeze results, then inspect."
