//! Quantifier-free integer linear arithmetic over kernel terms.
//!
//! Used for loop-index reasoning (`i = 32k`, guard failure implies
//! `n - i < 32`), bounds, and reassociation of sums. The procedure is
//! a Fourier–Motzkin elimination with the GCD tightening rule for
//! integers and strict-inequality normalization.
//!
//! Soundness direction: the procedure is used **only** to refute the
//! negation of an entailment. Every transformation preserves or
//! over-approximates the integer solution set, so `Valid` is a proof.
//! A satisfiable (or too-large) system yields `Unknown`, never a false
//! claim — the engine is sound but intentionally incomplete, and it
//! fails closed.

use std::collections::BTreeMap;

use crate::term::{Arena, Node, Sort, Tid};

/// A linear form `Σ c_i * atom_i + k` over integer-valued atoms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinForm {
    pub coeffs: BTreeMap<Tid, i128>,
    pub constant: i128,
}

impl LinForm {
    fn constant(k: i128) -> Self {
        LinForm {
            coeffs: BTreeMap::new(),
            constant: k,
        }
    }

    fn atom(t: Tid) -> Self {
        let mut coeffs = BTreeMap::new();
        coeffs.insert(t, 1);
        LinForm {
            coeffs,
            constant: 0,
        }
    }

    fn add_scaled(&mut self, other: &LinForm, scale: i128) {
        for (&atom, &c) in &other.coeffs {
            let entry = self.coeffs.entry(atom).or_insert(0);
            *entry += scale * c;
            if *entry == 0 {
                self.coeffs.remove(&atom);
            }
        }
        self.constant += scale * other.constant;
    }

    fn scaled(&self, scale: i128) -> LinForm {
        let mut out = LinForm::constant(self.constant * scale);
        for (&atom, &c) in &self.coeffs {
            out.coeffs.insert(atom, c * scale);
        }
        out
    }
}

/// Linearize an integer term. Non-linear subterms (products of two
/// unknowns, `ite`, conversions, applications) become opaque atoms.
/// Treating them as independent unknowns is a relaxation: it can lose
/// proofs, never create them.
pub fn linearize(arena: &Arena, t: Tid) -> LinForm {
    assert_eq!(arena.sort(t), Sort::Int, "linearize expects an Int term");
    match arena.node(t).clone() {
        Node::IntConst(v) => LinForm::constant(v),
        Node::IntAdd(a, b) => {
            let mut out = linearize(arena, a);
            let rhs = linearize(arena, b);
            out.add_scaled(&rhs, 1);
            out
        }
        Node::IntSub(a, b) => {
            let mut out = linearize(arena, a);
            let rhs = linearize(arena, b);
            out.add_scaled(&rhs, -1);
            out
        }
        Node::IntNeg(a) => {
            let inner = linearize(arena, a);
            inner.scaled(-1)
        }
        Node::IntMul(a, b) => {
            let la = linearize(arena, a);
            let lb = linearize(arena, b);
            if la.coeffs.is_empty() {
                lb.scaled(la.constant)
            } else if lb.coeffs.is_empty() {
                la.scaled(lb.constant)
            } else {
                LinForm::atom(t)
            }
        }
        _ => LinForm::atom(t),
    }
}

/// A normalized linear inequality: `Σ c_i * atom_i ≤ rhs` (strict:
/// `< rhs`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Constraint {
    coeffs: BTreeMap<Tid, i128>,
    rhs: i128,
    strict: bool,
}

impl Constraint {
    /// Normalize: divide by the GCD of the coefficients (integer
    /// tightening) and drop zero coefficients.
    fn normalize(mut self) -> Self {
        // Every linear form over integer variables has an integer
        // value, so `S < rhs` is exactly `S <= rhs - 1`. Normalizing
        // strictness away is what makes the integer tightening above
        // complete for the guard/bound fragments we use.
        if self.strict {
            self.strict = false;
            self.rhs -= 1;
        }
        self.coeffs.retain(|_, c| *c != 0);
        let mut g: i128 = 0;
        for &c in self.coeffs.values() {
            g = gcd(g, c.abs());
        }
        if g > 1 {
            for c in self.coeffs.values_mut() {
                *c /= g;
            }
            self.rhs = self.rhs.div_euclid(g);
        }
        self
    }

    fn is_trivially_false(&self) -> bool {
        if !self.coeffs.is_empty() {
            return false;
        }
        if self.strict {
            self.rhs <= 0
        } else {
            self.rhs < 0
        }
    }

    fn is_trivially_true(&self) -> bool {
        if !self.coeffs.is_empty() {
            return false;
        }
        if self.strict {
            self.rhs > 0
        } else {
            self.rhs >= 0
        }
    }
}

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Convert a positive boolean term into linear constraints.
fn to_constraints(arena: &Arena, t: Tid) -> Option<Vec<Constraint>> {
    assert_eq!(arena.sort(t), Sort::Bool);
    match arena.node(t).clone() {
        Node::BoolConst(true) => Some(vec![]),
        Node::BoolConst(false) => Some(vec![Constraint {
            coeffs: BTreeMap::new(),
            rhs: -1,
            strict: false,
        }]),
        Node::BoolAnd(a, b) => {
            let mut ca = to_constraints(arena, a)?;
            let cb = to_constraints(arena, b)?;
            ca.extend(cb);
            Some(ca)
        }
        Node::BoolNot(a) => to_constraints_neg(arena, a),
        Node::IntLe(a, b) => Some(vec![ineq(arena, a, b, false)]),
        Node::IntLt(a, b) => Some(vec![ineq(arena, a, b, true)]),
        Node::IntEq(a, b) => {
            let le = ineq(arena, a, b, false);
            let ge = ineq(arena, b, a, false);
            Some(vec![le, ge])
        }
        // `x <=> true` is just `x`; other equivalences are not
        // conjunctive.
        Node::BoolEq(a, b) => {
            let is_true = |t: Tid| matches!(arena.node(t), Node::BoolConst(true));
            if is_true(b) {
                to_constraints(arena, a)
            } else if is_true(a) {
                to_constraints(arena, b)
            } else {
                None
            }
        }
        Node::BoolOr(_, _) | Node::BoolIte(_, _, _) => None,
        _ => None,
    }
}

/// Convert a negated boolean term into constraints (only when the
/// negation stays conjunctive).
fn to_constraints_neg(arena: &Arena, t: Tid) -> Option<Vec<Constraint>> {
    match arena.node(t).clone() {
        Node::BoolConst(true) => Some(vec![Constraint {
            coeffs: BTreeMap::new(),
            rhs: -1,
            strict: false,
        }]),
        Node::BoolConst(false) => Some(vec![]),
        Node::BoolNot(inner) => to_constraints(arena, inner),
        Node::BoolOr(a, b) => {
            let mut ca = to_constraints_neg(arena, a)?;
            let cb = to_constraints_neg(arena, b)?;
            ca.extend(cb);
            Some(ca)
        }
        Node::IntLe(a, b) => Some(vec![ineq(arena, b, a, true)]),
        Node::IntLt(a, b) => Some(vec![ineq(arena, b, a, false)]),
        Node::IntEq(_, _) | Node::BoolAnd(_, _) | Node::BoolEq(_, _) | Node::BoolIte(_, _, _) => {
            None
        }
        _ => None,
    }
}

fn ineq(arena: &Arena, a: Tid, b: Tid, strict: bool) -> Constraint {
    let (la, lb) = (linearize(arena, a), linearize(arena, b));
    // a <= b  <=>  a - b <= 0
    let mut coeffs = la.coeffs.clone();
    for (&atom, &c) in &lb.coeffs {
        let entry = coeffs.entry(atom).or_insert(0);
        *entry -= c;
        if *entry == 0 {
            coeffs.remove(&atom);
        }
    }
    Constraint {
        coeffs,
        rhs: lb.constant - la.constant,
        strict,
    }
    .normalize()
}

/// Result of an arithmetic entailment query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArithOutcome {
    /// The entailment holds for all integers.
    Valid,
    /// The procedure could not prove it (fail closed).
    Unknown(String),
}

/// Prove `assumptions ⇒ goal` in linear integer arithmetic.
pub fn prove_entailment(arena: &Arena, assumptions: &[Tid], goal: Tid) -> ArithOutcome {
    // Structural decomposition of the goal.
    match arena.node(goal).clone() {
        Node::BoolConst(true) => return ArithOutcome::Valid,
        Node::BoolConst(false) => {
            // Goal false: only provable if the assumptions are
            // inconsistent.
            return refute(arena, assumptions, &[]);
        }
        Node::IntEq(a, b) => {
            let le = prove_literal(arena, assumptions, a, b, false);
            let ge = prove_literal(arena, assumptions, b, a, false);
            match (le, ge) {
                (ArithOutcome::Valid, ArithOutcome::Valid) => ArithOutcome::Valid,
                _ => ArithOutcome::Unknown("equality needs both directions".to_string()),
            }
        }
        Node::IntLe(a, b) => prove_literal(arena, assumptions, a, b, false),
        Node::IntLt(a, b) => prove_literal(arena, assumptions, a, b, true),
        Node::BoolAnd(a, b) => {
            let ra = prove_entailment(arena, assumptions, a);
            let rb = prove_entailment(arena, assumptions, b);
            match (ra, rb) {
                (ArithOutcome::Valid, ArithOutcome::Valid) => ArithOutcome::Valid,
                (ArithOutcome::Unknown(m), _) => ArithOutcome::Unknown(m),
                (_, ArithOutcome::Unknown(m)) => ArithOutcome::Unknown(m),
            }
        }
        Node::BoolNot(inner) => match arena.node(inner).clone() {
            Node::IntLe(a, b) => prove_literal(arena, assumptions, b, a, true),
            Node::IntLt(a, b) => prove_literal(arena, assumptions, b, a, false),
            Node::BoolConst(c) => {
                if c {
                    refute(arena, assumptions, &[])
                } else {
                    ArithOutcome::Valid
                }
            }
            _ => ArithOutcome::Unknown("unsupported negated goal".to_string()),
        },
        Node::BoolEq(a, b) => {
            // a <=> b: prove both implications.
            let mut with_a: Vec<Tid> = assumptions.to_vec();
            with_a.push(a);
            let mut with_b: Vec<Tid> = assumptions.to_vec();
            with_b.push(b);
            let ab = prove_entailment(arena, &with_a, b);
            let ba = prove_entailment(arena, &with_b, a);
            match (ab, ba) {
                (ArithOutcome::Valid, ArithOutcome::Valid) => ArithOutcome::Valid,
                (ArithOutcome::Unknown(m), _) => ArithOutcome::Unknown(m),
                (_, ArithOutcome::Unknown(m)) => ArithOutcome::Unknown(m),
            }
        }
        _ => ArithOutcome::Unknown("unsupported goal shape".to_string()),
    }
}

/// Prove `Σ assumptions ⇒ a (≤|<) b`.
fn prove_literal(arena: &Arena, assumptions: &[Tid], a: Tid, b: Tid, strict: bool) -> ArithOutcome {
    // Negate the goal: ¬(a ≤ b) is b < a; ¬(a < b) is b ≤ a.
    let negated = ineq(arena, b, a, !strict);
    refute(arena, assumptions, std::slice::from_ref(&negated))
}

fn refute(arena: &Arena, assumptions: &[Tid], extra: &[Constraint]) -> ArithOutcome {
    let mut system = Vec::new();
    for &a in assumptions {
        match to_constraints(arena, a) {
            Some(cs) => system.extend(cs),
            // An assumption outside the fragment is *skipped*: the
            // resulting system is weaker than the real one, so a
            // refutation still proves the entailment. (Proving from
            // fewer assumptions is sound; it can only lose proofs.)
            None => continue,
        }
    }
    system.extend(extra.iter().cloned());

    // Soundness: the constants in `extra` are already normalized.
    let outcome = eliminate(system);
    match outcome {
        ElimOutcome::Unsat => ArithOutcome::Valid,
        ElimOutcome::SatOrUnknown => {
            ArithOutcome::Unknown("system is satisfiable or not decided".to_string())
        }
        ElimOutcome::TooLarge => ArithOutcome::Unknown("constraint system grew too large".into()),
    }
}

enum ElimOutcome {
    Unsat,
    /// Sound-but-incomplete: the relaxation is satisfiable, so no
    /// conclusion about the integer system can be drawn.
    SatOrUnknown,
    TooLarge,
}

const MAX_CONSTRAINTS: usize = 4096;

fn eliminate(mut system: Vec<Constraint>) -> ElimOutcome {
    system = system.into_iter().map(|c| c.normalize()).collect();
    if system.iter().any(|c| c.is_trivially_false()) {
        return ElimOutcome::Unsat;
    }
    system.retain(|c| !c.is_trivially_true());

    // Eliminate variables one at a time (smallest alphabetically for
    // determinism).
    while let Some(var) = system.iter().flat_map(|c| c.coeffs.keys().copied()).min() {
        let mut uppers = Vec::new();
        let mut lowers = Vec::new();
        let mut rest = Vec::new();
        for c in system {
            match c.coeffs.get(&var).copied() {
                Some(0) | None => rest.push(c),
                Some(k) if k > 0 => uppers.push(c),
                Some(_) => lowers.push(c),
            }
        }
        let mut next = rest;
        for u in &uppers {
            let cu = u.coeffs[&var]; // > 0
            for l in &lowers {
                let cl = l.coeffs[&var]; // < 0
                                         // (-cl) * u + cu * l: cancels `var`, both multipliers > 0.
                let mut coeffs = BTreeMap::new();
                for (&atom, &c) in &u.coeffs {
                    if atom == var {
                        continue;
                    }
                    *coeffs.entry(atom).or_insert(0) += (-cl) * c;
                }
                for (&atom, &c) in &l.coeffs {
                    if atom == var {
                        continue;
                    }
                    *coeffs.entry(atom).or_insert(0) += cu * c;
                }
                let combined = Constraint {
                    coeffs,
                    rhs: (-cl) * u.rhs + cu * l.rhs,
                    strict: u.strict || l.strict,
                }
                .normalize();
                if combined.is_trivially_false() {
                    return ElimOutcome::Unsat;
                }
                if !combined.is_trivially_true() {
                    next.push(combined);
                }
                if next.len() > MAX_CONSTRAINTS {
                    return ElimOutcome::TooLarge;
                }
            }
        }
        system = next;
    }

    if system.iter().any(|c| c.is_trivially_false()) {
        ElimOutcome::Unsat
    } else {
        ElimOutcome::SatOrUnknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Arena {
        Arena::new()
    }

    #[test]
    fn proves_trivial_bound() {
        let mut a = setup();
        let x = a.declare_var("x", Sort::Int);
        let x = a.var(x);
        let zero = a.int(0);
        let five = a.int(5);
        let assumption = a.le(zero, x);
        let goal = a.lt(x, five);
        // x >= 0 does not imply x < 5.
        assert!(matches!(
            prove_entailment(&a, &[assumption], goal),
            ArithOutcome::Unknown(_)
        ));
    }

    #[test]
    fn proves_guard_failure_bound() {
        // i <= n && !(i + 32 <= n)  =>  n - i <= 31
        let mut a = setup();
        let i = a.declare_var("i", Sort::Int);
        let n = a.declare_var("n", Sort::Int);
        let (i, n) = (a.var(i), a.var(n));
        let zero = a.int(0);
        let k32 = a.int(32);
        let i_plus = a.add(i, k32);
        let guard = a.le(i_plus, n);
        let a1 = a.le(zero, i);
        let a2 = a.le(i, n);
        let not_guard = a.bool_not(guard);
        let k31 = a.int(31);
        let diff = a.sub(n, i);
        let goal = a.le(diff, k31);
        assert_eq!(
            prove_entailment(&a, &[a1, a2, not_guard], goal),
            ArithOutcome::Valid
        );
    }

    #[test]
    fn proves_integer_strict_tightening() {
        // x < 5 => x <= 4 over the integers.
        let mut a = setup();
        let x = a.declare_var("x", Sort::Int);
        let x = a.var(x);
        let five = a.int(5);
        let four = a.int(4);
        let hyp = a.lt(x, five);
        let goal = a.le(x, four);
        assert_eq!(prove_entailment(&a, &[hyp], goal), ArithOutcome::Valid);
    }

    #[test]
    fn proves_linear_combination() {
        // 2k + 3 <= n => 2k + 2 <= n
        let mut a = setup();
        let k = a.declare_var("k", Sort::Int);
        let n = a.declare_var("n", Sort::Int);
        let (k, n) = (a.var(k), a.var(n));
        let two = a.int(2);
        let three = a.int(3);
        let acc = a.mul(two, k);
        let acc_plus_three = a.add(acc, three);
        let hyp = a.le(acc_plus_three, n);
        let acc_plus_two = a.add(acc, two);
        let goal = a.le(acc_plus_two, n);
        assert_eq!(prove_entailment(&a, &[hyp], goal), ArithOutcome::Valid);
        // The converse must not be claimed.
        assert!(matches!(
            prove_entailment(&a, &[goal], hyp),
            ArithOutcome::Unknown(_)
        ));
    }

    #[test]
    fn rejects_unsound_bound() {
        // i <= n && !(i + 32 <= n) does NOT imply n - i <= 30
        let mut a = setup();
        let i = a.declare_var("i", Sort::Int);
        let n = a.declare_var("n", Sort::Int);
        let (i, n) = (a.var(i), a.var(n));
        let zero = a.int(0);
        let k32 = a.int(32);
        let i_plus_32 = a.add(i, k32);
        let guard = a.le(i_plus_32, n);
        let a1 = a.le(zero, i);
        let a2 = a.le(i, n);
        let not_guard = a.bool_not(guard);
        let k30 = a.int(30);
        let diff = a.sub(n, i);
        let goal = a.le(diff, k30);
        assert!(matches!(
            prove_entailment(&a, &[a1, a2, not_guard], goal),
            ArithOutcome::Unknown(_)
        ));
    }
}
