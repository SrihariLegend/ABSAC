//! Tseitin bit-blasting: every bitvector term becomes a vector of CNF
//! literals (LSB first), one literal per bit, with clauses defining the
//! gate.
//!
//! The encoding is deliberately conservative: every gate is fully
//! defined (bi-implication), so a satisfying assignment of the CNF is
//! exactly an assignment of bits to terms.

use std::collections::HashMap;

use crate::bv::{BinOp, Bv, Node, Term, VarId};

/// A CNF formula. Literals are non-zero `i32`; variable `n` is the
/// literal `n` and its negation is `-n`.
#[derive(Clone, Debug)]
pub struct Cnf {
    pub num_vars: u32,
    pub clauses: Vec<Vec<i32>>,
    /// A literal that is fixed true (a dedicated variable with a unit
    /// clause). Used for boolean constants.
    pub true_lit: i32,
}

impl Default for Cnf {
    fn default() -> Self {
        Self::new()
    }
}

impl Cnf {
    pub fn new() -> Self {
        let mut cnf = Cnf {
            num_vars: 0,
            clauses: Vec::new(),
            true_lit: 0,
        };
        let t = cnf.new_lit();
        cnf.clauses.push(vec![t]);
        cnf.true_lit = t;
        cnf
    }

    /// Allocate a fresh variable and return its positive literal.
    pub fn new_lit(&mut self) -> i32 {
        self.num_vars += 1;
        self.num_vars as i32
    }

    pub fn add_clause<I: IntoIterator<Item = i32>>(&mut self, lits: I) {
        let clause: Vec<i32> = lits.into_iter().collect();
        debug_assert!(!clause.contains(&0), "zero literal is not a literal");
        self.clauses.push(clause);
    }

    /// The always-true literal.
    pub fn t(&self) -> i32 {
        self.true_lit
    }

    /// The always-false literal.
    pub fn f(&self) -> i32 {
        -self.true_lit
    }

    /// `z <=> a /\ b`
    pub fn and2(&mut self, a: i32, b: i32) -> i32 {
        // Constant/value folding keeps the CNF small for encodings with
        // constant operands (e.g. a literal divisor in division).
        if a == self.t() {
            return b;
        }
        if b == self.t() {
            return a;
        }
        if a == self.f() || b == self.f() {
            return self.f();
        }
        if a == b {
            return a;
        }
        if a == -b {
            return self.f();
        }
        let z = self.new_lit();
        self.add_clause([-a, -b, z]);
        self.add_clause([a, -z]);
        self.add_clause([b, -z]);
        z
    }

    /// `z <=> a \/ b`
    pub fn or2(&mut self, a: i32, b: i32) -> i32 {
        if a == self.t() || b == self.t() {
            return self.t();
        }
        if a == self.f() {
            return b;
        }
        if b == self.f() {
            return a;
        }
        if a == b {
            return a;
        }
        if a == -b {
            return self.t();
        }
        let z = self.new_lit();
        self.add_clause([a, b, -z]);
        self.add_clause([-a, z]);
        self.add_clause([-b, z]);
        z
    }

    /// `z <=> a xor b`
    pub fn xor2(&mut self, a: i32, b: i32) -> i32 {
        if a == self.t() {
            return -b;
        }
        if b == self.t() {
            return -a;
        }
        if a == self.f() {
            return b;
        }
        if b == self.f() {
            return a;
        }
        if a == b {
            return self.f();
        }
        if a == -b {
            return self.t();
        }
        let z = self.new_lit();
        self.add_clause([-a, -b, -z]);
        self.add_clause([a, b, -z]);
        self.add_clause([a, -b, z]);
        self.add_clause([-a, b, z]);
        z
    }

    /// `z <=> a == b`
    pub fn xnor2(&mut self, a: i32, b: i32) -> i32 {
        if a == b {
            return self.t();
        }
        if a == -b {
            return self.f();
        }
        if a == self.t() {
            return b;
        }
        if b == self.t() {
            return a;
        }
        if a == self.f() {
            return -b;
        }
        if b == self.f() {
            return -a;
        }
        let z = self.new_lit();
        self.add_clause([-a, b, -z]);
        self.add_clause([a, -b, -z]);
        self.add_clause([a, b, z]);
        self.add_clause([-a, -b, z]);
        z
    }

    /// `z <=> (c ? t : e)`
    pub fn mux(&mut self, c: i32, t: i32, e: i32) -> i32 {
        if c == self.t() {
            return t;
        }
        if c == self.f() {
            return e;
        }
        if t == e {
            return t;
        }
        let z = self.new_lit();
        self.add_clause([-c, -t, z]);
        self.add_clause([-c, t, -z]);
        self.add_clause([c, -e, z]);
        self.add_clause([c, e, -z]);
        z
    }

    pub fn and_all(&mut self, lits: &[i32]) -> i32 {
        let mut acc = self.t();
        for &l in lits {
            acc = self.and2(acc, l);
        }
        acc
    }

    pub fn or_all(&mut self, lits: &[i32]) -> i32 {
        let mut acc = self.f();
        for &l in lits {
            acc = self.or2(acc, l);
        }
        acc
    }
}

/// Tseitin encoder over one [`Bv`] arena.
pub struct Encoder<'a> {
    bv: &'a Bv,
    cnf: Cnf,
    cache: HashMap<Term, Vec<i32>>,
    var_bits: HashMap<VarId, Vec<i32>>,
}

impl<'a> Encoder<'a> {
    pub fn new(bv: &'a Bv) -> Self {
        Self {
            bv,
            cnf: Cnf::new(),
            cache: HashMap::new(),
            var_bits: HashMap::new(),
        }
    }

    pub fn cnf(&self) -> &Cnf {
        &self.cnf
    }

    pub fn cnf_mut(&mut self) -> &mut Cnf {
        &mut self.cnf
    }

    pub fn finish(self) -> Cnf {
        self.cnf
    }

    /// The CNF literals encoding each declared variable, LSB first.
    pub fn var_bit_map(&self) -> HashMap<VarId, Vec<i32>> {
        self.var_bits.clone()
    }

    /// Encode a term as one literal per bit, LSB first.
    pub fn encode(&mut self, t: Term) -> Vec<i32> {
        if let Some(bits) = self.cache.get(&t) {
            return bits.clone();
        }
        let width = self.bv.width(t);
        let bits = match self.bv.node(t) {
            Node::Const(value) => (0..width)
                .map(|i| {
                    if (value >> i) & 1 == 1 {
                        self.cnf.t()
                    } else {
                        self.cnf.f()
                    }
                })
                .collect(),
            Node::Var(id) => {
                let bits: Vec<i32> = (0..width).map(|_| self.cnf.new_lit()).collect();
                self.var_bits.entry(id).or_insert_with(|| bits.clone());
                bits
            }
            Node::Bin(op, a, b) => {
                let ab = self.encode(a);
                let bb = self.encode(b);
                match op {
                    BinOp::And => zip_gate(&mut self.cnf, &ab, &bb, Cnf::and2),
                    BinOp::Or => zip_gate(&mut self.cnf, &ab, &bb, Cnf::or2),
                    BinOp::Xor => zip_gate(&mut self.cnf, &ab, &bb, Cnf::xor2),
                    BinOp::Add => {
                        let fals = self.cnf.f();
                        add_bits(&mut self.cnf, &ab, &bb, fals).0
                    }
                    BinOp::Sub => {
                        let not_b: Vec<i32> = bb.iter().map(|&l| -l).collect();
                        let one = self.cnf.t();
                        add_bits(&mut self.cnf, &ab, &not_b, one).0
                    }
                    BinOp::Mul => mul_bits(&mut self.cnf, &ab, &bb),
                    BinOp::Shl => self.shift_bits(&ab, &bb, ShiftKind::Left),
                    BinOp::Lshr => self.shift_bits(&ab, &bb, ShiftKind::LogicalRight),
                    BinOp::Ashr => self.shift_bits(&ab, &bb, ShiftKind::ArithmeticRight),
                    // Unsigned division/remainder: restoring division.
                    // Division by zero yields all-ones and remainder by
                    // zero the dividend (SMT-LIB bitvector semantics),
                    // which is exactly what the circuit computes when
                    // the divisor bits are all zero.
                    BinOp::Udiv => udivrem_bits(&mut self.cnf, &ab, &bb).0,
                    BinOp::Urem => udivrem_bits(&mut self.cnf, &ab, &bb).1,
                    BinOp::Ult => vec![ult_bits(&mut self.cnf, &ab, &bb)],
                    BinOp::Ule => {
                        let lt = ult_bits(&mut self.cnf, &ab, &bb);
                        let eq = eq_bits(&mut self.cnf, &ab, &bb);
                        vec![self.cnf.or2(lt, eq)]
                    }
                    BinOp::Eq => vec![eq_bits(&mut self.cnf, &ab, &bb)],
                }
            }
            Node::Ite(c, x, y) => {
                let cb = self.encode(c)[0];
                let xb = self.encode(x);
                let yb = self.encode(y);
                xb.iter()
                    .zip(yb.iter())
                    .map(|(&xt, &yt)| self.cnf.mux(cb, xt, yt))
                    .collect()
            }
            Node::Extract(a, _, lo) => {
                let ab = self.encode(a);
                ab[lo as usize..=(lo as usize + width as usize - 1)].to_vec()
            }
            Node::Concat(high, low) => {
                let mut bits = self.encode(low);
                bits.extend(self.encode(high));
                bits
            }
            Node::ZeroExt(a) => {
                let mut bits = self.encode(a);
                while bits.len() < width as usize {
                    bits.push(self.cnf.f());
                }
                bits
            }
            Node::SignExt(a) => {
                let ab = self.encode(a);
                let sign = *ab.last().expect("operand has at least one bit");
                let mut bits = ab;
                while bits.len() < width as usize {
                    bits.push(sign);
                }
                bits
            }
            Node::Popcount(a) => {
                let ab = self.encode(a);
                let mut acc = vec![self.cnf.f(); width as usize];
                let fals = self.cnf.f();
                for &bit in &ab {
                    // add one 1-bit value into the accumulator
                    let mut addend = vec![bit];
                    while addend.len() < width as usize {
                        addend.push(self.cnf.f());
                    }
                    acc = add_bits(&mut self.cnf, &acc, &addend, fals).0;
                }
                acc
            }
        };
        debug_assert_eq!(bits.len(), width as usize);
        self.cache.insert(t, bits.clone());
        bits
    }

    fn shift_bits(&mut self, a: &[i32], amount: &[i32], kind: ShiftKind) -> Vec<i32> {
        let fals = self.cnf.f();
        // Constant shift: pure bit rearrangement.
        if let Some(value) = self.const_bits_value(amount) {
            return constant_shift(a, value, kind, fals);
        }
        let mut acc = a.to_vec();
        for (stage, &bit) in amount.iter().enumerate() {
            let distance = 1u64 << stage;
            // Shifts of >= width produce zero (left/logical) or the
            // sign fill (arithmetic); `constant_shift` handles both.
            let shifted = constant_shift(&acc, distance, kind, fals);
            acc = acc
                .iter()
                .zip(shifted.iter())
                .map(|(&old, &new)| self.cnf.mux(bit, new, old))
                .collect();
        }
        acc
    }

    /// If these bits encode a constant, recover its value.
    fn const_bits_value(&self, bits: &[i32]) -> Option<u64> {
        let t = self.cnf.t();
        let f = self.cnf.f();
        if bits.iter().all(|&l| l == t || l == f) {
            let mut value = 0u64;
            for (i, &l) in bits.iter().enumerate() {
                if l == t {
                    value |= 1u64 << i;
                }
            }
            Some(value)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
enum ShiftKind {
    Left,
    LogicalRight,
    ArithmeticRight,
}

fn constant_shift(a: &[i32], amount: u64, kind: ShiftKind, fals: i32) -> Vec<i32> {
    let width = a.len();
    let mut out = Vec::with_capacity(width);
    if amount >= width as u64 {
        match kind {
            ShiftKind::ArithmeticRight => {
                let sign = *a.last().expect("non-empty");
                out.resize(width, sign);
            }
            _ => {
                out.resize(width, fals);
            }
        }
        return out;
    }
    match kind {
        ShiftKind::Left => {
            let amt = amount as usize;
            for i in 0..width {
                if i < amt {
                    out.push(fals);
                } else {
                    out.push(a[i - amt]);
                }
            }
        }
        ShiftKind::LogicalRight => {
            let amt = amount as usize;
            for i in 0..width {
                if i + amt < width {
                    out.push(a[i + amt]);
                } else {
                    out.push(fals);
                }
            }
        }
        ShiftKind::ArithmeticRight => {
            let amt = amount as usize;
            let sign = *a.last().expect("non-empty");
            for i in 0..width {
                if i + amt < width {
                    out.push(a[i + amt]);
                } else {
                    out.push(sign);
                }
            }
        }
    }
    out
}

fn zip_gate<F>(cnf: &mut Cnf, a: &[i32], b: &[i32], f: F) -> Vec<i32>
where
    F: Fn(&mut Cnf, i32, i32) -> i32,
{
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| f(cnf, x, y))
        .collect()
}

/// Ripple-carry addition; returns (sum bits, carry out).
fn add_bits(cnf: &mut Cnf, a: &[i32], b: &[i32], cin: i32) -> (Vec<i32>, i32) {
    let width = a.len();
    debug_assert_eq!(b.len(), width);
    let mut carry = cin;
    let mut sum = Vec::with_capacity(width);
    for i in 0..width {
        let axb = cnf.xor2(a[i], b[i]);
        let s = cnf.xor2(axb, carry);
        let c1 = cnf.and2(a[i], b[i]);
        let c2 = cnf.and2(carry, axb);
        carry = cnf.or2(c1, c2);
        sum.push(s);
    }
    (sum, carry)
}

/// Shift-and-add multiplication.
/// `a - b` at bit level (LSB-first slices of equal width).
fn sub_bits(cnf: &mut Cnf, a: &[i32], b: &[i32]) -> Vec<i32> {
    let not_b: Vec<i32> = b.iter().map(|&l| -l).collect();
    let one = cnf.t();
    add_bits(cnf, a, &not_b, one).0
}

/// Restoring unsigned division: returns `(quotient, remainder)` for
/// LSB-first bit slices of equal width. Division by zero yields
/// all-ones / the dividend, matching SMT-LIB bitvector semantics.
fn udivrem_bits(cnf: &mut Cnf, a: &[i32], b: &[i32]) -> (Vec<i32>, Vec<i32>) {
    let width = a.len();
    debug_assert_eq!(b.len(), width);
    let mut rem = vec![cnf.f(); width];
    let mut quot = vec![cnf.f(); width];
    for i in (0..width).rev() {
        // rem = (rem << 1) | a[i]
        for j in (1..width).rev() {
            rem[j] = rem[j - 1];
        }
        rem[0] = a[i];
        // ge = rem >= b; the subtraction only lands when it is true.
        let lt = ult_bits(cnf, &rem, b);
        let ge = -lt;
        let diff = sub_bits(cnf, &rem, b);
        for j in 0..width {
            rem[j] = cnf.mux(ge, diff[j], rem[j]);
        }
        quot[i] = ge;
    }
    (quot, rem)
}

fn mul_bits(cnf: &mut Cnf, a: &[i32], b: &[i32]) -> Vec<i32> {
    let width = a.len();
    let mut acc = vec![cnf.f(); width];
    for (i, &bit) in b.iter().enumerate() {
        if i >= width {
            break;
        }
        // partial = a << i, truncated to width
        let mut partial = vec![cnf.f(); i];
        partial.extend_from_slice(&a[..width - i]);
        let masked: Vec<i32> = partial.iter().map(|&p| cnf.and2(p, bit)).collect();
        acc = add_bits(cnf, &acc, &masked, cnf.f()).0;
    }
    acc
}

/// Unsigned less-than, LSB-first prefix accumulation.
fn ult_bits(cnf: &mut Cnf, a: &[i32], b: &[i32]) -> i32 {
    let mut lt = cnf.f();
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        let new_bit = cnf.and2(-ai, bi);
        let same = cnf.xnor2(ai, bi);
        let carry_lt = cnf.and2(same, lt);
        lt = cnf.or2(new_bit, carry_lt);
    }
    lt
}

fn eq_bits(cnf: &mut Cnf, a: &[i32], b: &[i32]) -> i32 {
    let mut eq = cnf.t();
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        let bit_eq = cnf.xnor2(ai, bi);
        eq = cnf.and2(eq, bit_eq);
    }
    eq
}
