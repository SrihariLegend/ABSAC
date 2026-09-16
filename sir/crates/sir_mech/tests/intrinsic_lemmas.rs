//! Intrinsic-model lemmas, proved (not assumed) by the kernel.
//!
//! The AVX2 model used by Gate 4B is:
//!
//! ```text
//!   pcmpeqb(x, y)      = 0xFF if x == y else 0x00          (per byte)
//!   movemask(v)[i]     = msb of lane i
//!   popcount(v)        = number of set bits
//! ```
//!
//! Everything below is a theorem about those definitions: the per-byte
//! lemma (a movemask bit is byte equality), the popcount decomposition
//! (popcount = sum of its bits), and the 32-lane chunk identities the
//! k50/k18 vector loops rely on.

use sir_mech::kernel::{Context, Kernel, KernelError, SchemaId, Theorem};
use sir_mech::term::{BvOp, Sort, Symbol, Tid};

// ── small helpers (one arena borrow per call) ────────────────────────

fn varg(k: &mut Kernel, name: &str, sort: Sort) -> Tid {
    let s = k.arena_mut().declare_var(name, sort);
    k.arena_mut().var(s)
}

fn bv_and(k: &mut Kernel, a: Tid, b: Tid) -> Tid {
    k.arena_mut().bv_bin(BvOp::And, a, b)
}

fn bv_eq(k: &mut Kernel, a: Tid, b: Tid) -> Tid {
    k.arena_mut().bv_bin(BvOp::Eq, a, b)
}

fn bv_const(k: &mut Kernel, v: u128, w: u32) -> Tid {
    k.arena_mut().bv(v, w)
}

/// `movemask` bit for one lane: `msb(pcmpeqb(a, b))` as a 1-bit term.
fn lane_bit(k: &mut Kernel, a: Tid, b: Tid) -> Tid {
    let eq = bv_eq(k, a, b);
    let ones = bv_const(k, 0xFF, 8);
    let zero = bv_const(k, 0x00, 8);
    let cmp = k.arena_mut().bv_ite(eq, ones, zero);
    k.arena_mut().bv_extract(cmp, 7, 7)
}

fn int_of(k: &mut Kernel, t: Tid) -> Tid {
    k.arena_mut().bv_to_int(t)
}

fn sum_of(k: &mut Kernel, terms: &[Tid]) -> Tid {
    let zero = k.arena_mut().int(0);
    let mut acc = zero;
    for &t in terms {
        acc = k.arena_mut().add(acc, t);
    }
    acc
}

fn fresh_hole(k: &mut Kernel, sort: Sort) -> Symbol {
    let name = format!("hole{}", k.arena().symbol_count());
    k.arena_mut().declare_var(name, sort)
}

fn find(k: &Kernel, name: &str) -> Symbol {
    for i in 0..k.arena().symbol_count() as u32 {
        let s = Symbol(i);
        if k.arena().var_name(s) == name {
            return s;
        }
    }
    panic!("symbol {name} not found");
}

// ── the per-byte lemma ───────────────────────────────────────────────

#[test]
fn per_byte_lemma_proved_and_replayed() {
    let mut k = Kernel::new();
    let x = varg(&mut k, "x", Sort::Bv(8));
    let y = varg(&mut k, "y", Sort::Bv(8));
    let bit = lane_bit(&mut k, x, y);
    let lhs = int_of(&mut k, bit);
    let eq = bv_eq(&mut k, x, y);
    let rhs = k.arena_mut().bool_to_int(eq);
    let thm = k.bitblast_eq(lhs, rhs).expect("per-byte lemma");
    k.replay(&thm).expect("per-byte lemma replay");
}

// ── popcount = sum of bits ───────────────────────────────────────────

#[test]
fn popcount_is_sum_of_bits() {
    let mut k = Kernel::new();
    let x = varg(&mut k, "x", Sort::Bv(32));
    let pc = k.arena_mut().bv_popcount(x);
    let lhs = int_of(&mut k, pc);
    let mut bits = Vec::new();
    for bit in 0..32 {
        let b = k.arena_mut().bv_extract(x, bit, bit);
        bits.push(int_of(&mut k, b));
    }
    let rhs = sum_of(&mut k, &bits);
    let thm = k.bitblast_eq(lhs, rhs).expect("popcount lemma");
    k.replay(&thm).expect("popcount lemma replay");
}

// ── the 32-lane cardinality chunk lemma ──────────────────────────────

fn cardinality_chunk(
    k: &mut Kernel,
    bytes: &[Tid],
    mask: Tid,
    target: Tid,
) -> Result<Theorem, KernelError> {
    assert_eq!(bytes.len(), 32);
    let mut bits = Vec::new();
    let mut indicators = Vec::new();
    for &b in bytes {
        let masked = bv_and(k, b, mask);
        bits.push(lane_bit(k, masked, target));
        let eq = bv_eq(k, masked, target);
        indicators.push(k.arena_mut().bool_to_int(eq));
    }
    let movemask = k.arena_mut().from_bits(&bits).expect("movemask width");
    let pc = k.arena_mut().bv_popcount(movemask);
    let lhs = int_of(k, pc);
    let rhs = sum_of(k, &indicators);
    k.bitblast_eq(lhs, rhs)
}

#[test]
fn cardinality_chunk_lemma_proved() {
    let mut k = Kernel::new();
    let mut bytes = Vec::new();
    for i in 0..32 {
        bytes.push(varg(&mut k, &format!("b{i}"), Sort::Bv(8)));
    }
    let mask = varg(&mut k, "mask", Sort::Bv(8));
    let target = varg(&mut k, "target", Sort::Bv(8));
    let thm = cardinality_chunk(&mut k, &bytes, mask, target).expect("chunk lemma");
    k.replay(&thm).expect("chunk lemma replay");
}

// ── the 32-lane `All` chunk lemma ────────────────────────────────────

#[test]
fn all_chunk_lemma_proved() {
    let mut k = Kernel::new();
    let mut bytes = Vec::new();
    for i in 0..32 {
        bytes.push(varg(&mut k, &format!("b{i}"), Sort::Bv(8)));
    }
    let val = varg(&mut k, "val", Sort::Bv(8));
    let mut bits = Vec::new();
    let mut conj = k.arena_mut().bool(true);
    for &b in &bytes {
        bits.push(lane_bit(&mut k, b, val));
        let eq = bv_eq(&mut k, b, val);
        conj = k.arena_mut().bool_and(conj, eq);
    }
    let movemask = k.arena_mut().from_bits(&bits).expect("movemask width");
    let full = bv_const(&mut k, 0xFFFF_FFFF, 32);
    let lhs = bv_eq(&mut k, movemask, full);
    let thm = k.bitblast_eq(lhs, conj).expect("all chunk lemma");
    k.replay(&thm).expect("all chunk lemma replay");
}

// ── composition: rewriting a lemma inside a sum context ──────────────

#[test]
fn sum_context_rewrite() {
    let mut k = Kernel::new();
    let x = varg(&mut k, "x", Sort::Bv(8));
    let y = varg(&mut k, "y", Sort::Bv(8));
    let bit = lane_bit(&mut k, x, y);
    let lhs = int_of(&mut k, bit);
    let eq = bv_eq(&mut k, x, y);
    let rhs = k.arena_mut().bool_to_int(eq);
    let lemma = k.bitblast_eq(lhs, rhs).expect("per-byte lemma");

    let b0 = varg(&mut k, "b0", Sort::Bv(8));
    let v = varg(&mut k, "val", Sort::Bv(8));
    let x_sym = find(&k, "x");
    let y_sym = find(&k, "y");
    let instance = k
        .instantiate(&lemma, &[(x_sym, b0), (y_sym, v)])
        .expect("instantiate");

    let h = fresh_hole(&mut k, Sort::Int);
    let hv = k.arena_mut().var(h);
    let other_bit = lane_bit(&mut k, v, v);
    let other = int_of(&mut k, other_bit);
    let template = k.arena_mut().add(hv, other);
    let rewritten = k
        .congr(Context { hole: h, template }, &instance)
        .expect("congr");
    k.replay(&rewritten).expect("replay rewrite");
}

/// The composition pattern Gate 4B uses: a proven schema is instantiated
/// at concrete terms and then rewritten inside a larger context.
#[test]
fn schema_instantiation_composes() {
    let mut k = Kernel::new();
    // schema: P(x) = x + 0
    let x = k.arena_mut().declare_var("x", Sort::Int);
    let (lhs, rhs) = {
        let xv = k.arena_mut().var(x);
        let zero = k.arena_mut().int(0);
        (xv, k.arena_mut().add(xv, zero))
    };
    let schema_id: SchemaId = k.schema("P(x)=x+0", &[x], lhs, rhs);
    let seven = k.arena_mut().int(7);
    let instance = k.apply_schema(schema_id, &[(x, seven)]).expect("schema");
    assert_eq!(k.describe(&instance.statement), "[] ⊢ 7 = 7 + 0");
    // 7 + 0 = 7 by arithmetic.
    let seven_plus_zero = instance.statement.rhs;
    let arith = k.arith_eq(seven_plus_zero, seven).expect("arith");
    // Chain: 7 = 7 + 0 = 7.
    let chained = k.trans(&instance, &arith).expect("trans");
    assert_eq!(chained.statement.lhs, seven);
    assert_eq!(chained.statement.rhs, seven);
    k.replay(&chained).expect("replay chained");
}
