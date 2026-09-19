//! Fixed-width bitvector terms with a concrete evaluator.
//!
//! This is the object language Gate 4B lemmas are stated in. Terms are
//! arena indices (a DAG), every node carries an explicit width, and the
//! evaluator is total for every term the builder admits. Every
//! constructor validates its operands, so an ill-formed term cannot be
//! represented.

use std::collections::BTreeMap;
use sir_types::fnv::FastHashMap;
use std::fmt;

/// Largest supported bitvector width.
pub const MAX_WIDTH: u32 = 64;

/// A symbolic input.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct VarId(pub u32);

impl fmt::Display for VarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.0)
    }
}

/// Binary operations. Comparisons produce a 1-bit result.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BinOp {
    And,
    Or,
    Xor,
    Add,
    Sub,
    Mul,
    Shl,
    Lshr,
    Ashr,
    Udiv,
    Urem,
    Ult,
    Ule,
    Eq,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::And => "&",
            BinOp::Or => "|",
            BinOp::Xor => "^",
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Shl => "<<",
            BinOp::Lshr => ">>u",
            BinOp::Ashr => ">>s",
            BinOp::Udiv => "/u",
            BinOp::Urem => "%u",
            BinOp::Ult => "<u",
            BinOp::Ule => "<=u",
            BinOp::Eq => "==",
        }
    }

    /// Comparisons return one bit; everything else preserves the
    /// operand width.
    pub fn result_width(self, operand_width: u32) -> u32 {
        match self {
            BinOp::Ult | BinOp::Ule | BinOp::Eq => 1,
            _ => operand_width,
        }
    }
}

/// A term: an index into a [`Bv`] arena.
pub type Term = u32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Node {
    Const(u64),
    Var(VarId),
    Bin(BinOp, Term, Term),
    Ite(Term, Term, Term),
    Extract(Term, u32, u32),
    Concat(Term, Term),
    ZeroExt(Term),
    SignExt(Term),
    Popcount(Term),
}

fn mask(width: u32) -> u64 {
    debug_assert!(width >= 1 && width <= 64);
    if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

/// A bitvector arena. All terms are immutable once built, so several
/// terms may share subterms.
#[derive(Clone, Debug)]
pub struct Bv {
    nodes: Vec<Node>,
    widths: Vec<u32>,
    var_widths: BTreeMap<VarId, u32>,
    /// Structural interning: identical (node, width) pairs share one term id,
    /// so definitional unfoldings compare by identity instead of bit-blasting.
    interns: FastHashMap<(Node, u32), Term>,
}

impl Bv {
    /// Create an arena with reserved capacity for `terms` terms.
    pub fn with_capacity(terms: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(terms),
            widths: Vec::with_capacity(terms),
            var_widths: BTreeMap::new(),
            interns: FastHashMap::with_capacity_and_hasher(terms, Default::default()),
        }
    }

    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    /// Number of terms in the arena.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Width of a term.
    pub fn width(&self, t: Term) -> u32 {
        self.widths[t as usize]
    }

    /// The arena node for a term.
    pub fn node(&self, t: Term) -> Node {
        self.nodes[t as usize]
    }

    /// Every declared variable and its width, in declaration order.
    pub fn vars(&self) -> Vec<(VarId, u32)> {
        self.var_widths.iter().map(|(k, v)| (*k, *v)).collect()
    }

    pub fn constant(&mut self, value: u64, width: u32) -> Term {
        self.check_width(width);
        self.push(Node::Const(value & mask(width)), width)
    }

    pub fn zero(&mut self, width: u32) -> Term {
        self.constant(0, width)
    }

    pub fn one(&mut self, width: u32) -> Term {
        self.constant(1, width)
    }

    /// All-ones constant of the given width.
    pub fn ones(&mut self, width: u32) -> Term {
        self.constant(mask(width), width)
    }

    pub fn var(&mut self, id: VarId, width: u32) -> Term {
        self.check_width(width);
        match self.var_widths.get(&id) {
            Some(existing) => assert_eq!(
                *existing, width,
                "variable {id} redeclared with width {width} (was {existing})"
            ),
            None => {
                self.var_widths.insert(id, width);
            }
        }
        self.push(Node::Var(id), width)
    }

    /// Construct `a op b`. Operands must have equal widths; comparisons
    /// return a 1-bit term.
    pub fn bin(&mut self, op: BinOp, a: Term, b: Term) -> Term {
        let wa = self.width(a);
        let wb = self.width(b);
        assert_eq!(wa, wb, "operand width mismatch for {}", op.symbol());
        let width = op.result_width(wa);
        self.push(Node::Bin(op, a, b), width)
    }

    pub fn and(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::And, a, b)
    }
    pub fn or(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Or, a, b)
    }
    pub fn xor(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Xor, a, b)
    }
    pub fn add(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Add, a, b)
    }
    pub fn sub(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Sub, a, b)
    }
    pub fn mul(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Mul, a, b)
    }
    pub fn shl(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Shl, a, b)
    }
    pub fn lshr(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Lshr, a, b)
    }
    /// Unsigned division (SMT-LIB semantics: by zero is all-ones).
    pub fn udiv(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Udiv, a, b)
    }

    /// Unsigned remainder (SMT-LIB semantics: by zero is the dividend).
    pub fn urem(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Urem, a, b)
    }
    pub fn ult(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Ult, a, b)
    }
    pub fn eq(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Eq, a, b)
    }

    /// Unsigned less-than-or-equal.
    pub fn ule(&mut self, a: Term, b: Term) -> Term {
        self.bin(BinOp::Ule, a, b)
    }

    /// Bitwise NOT.
    pub fn not(&mut self, a: Term) -> Term {
        let ones = self.ones(self.width(a));
        self.xor(a, ones)
    }

    /// Two's-complement negation.
    pub fn neg(&mut self, a: Term) -> Term {
        let zero = self.zero(self.width(a));
        self.sub(zero, a)
    }

    /// `c ? t : e`; `c` is a 1-bit term, `t` and `e` share a width.
    pub fn ite(&mut self, c: Term, t: Term, e: Term) -> Term {
        assert_eq!(self.width(c), 1, "ite condition must be 1 bit");
        let wt = self.width(t);
        let we = self.width(e);
        assert_eq!(wt, we, "ite branch width mismatch");
        self.push(Node::Ite(c, t, e), wt)
    }

    /// Extract bits `[lo, hi]` (inclusive).
    pub fn extract(&mut self, a: Term, hi: u32, lo: u32) -> Term {
        let wa = self.width(a);
        assert!(
            lo <= hi && hi < wa,
            "extract {hi}..{lo} out of range for width {wa}"
        );
        self.push(Node::Extract(a, hi, lo), hi - lo + 1)
    }

    /// Bit at position `i`.
    pub fn bit(&mut self, a: Term, i: u32) -> Term {
        self.extract(a, i, i)
    }

    /// Concatenate `high` (more significant) with `low`.
    pub fn concat(&mut self, high: Term, low: Term) -> Term {
        let wh = self.width(high);
        let wl = self.width(low);
        let width = wh + wl;
        self.check_width(width);
        self.push(Node::Concat(high, low), width)
    }

    /// Pack a little-endian bit slice (index 0 = LSB) into one term.
    pub fn from_bits_lsb(&mut self, bits: &[Term]) -> Term {
        assert!(!bits.is_empty(), "from_bits_lsb needs at least one bit");
        for &b in bits {
            assert_eq!(self.width(b), 1, "bit term must be 1 bit wide");
        }
        let mut acc = bits[0];
        for &b in &bits[1..] {
            acc = self.concat(b, acc);
        }
        acc
    }

    /// Zero-extend to `width`.
    pub fn zero_ext(&mut self, a: Term, width: u32) -> Term {
        let wa = self.width(a);
        assert!(width >= wa, "zero_ext to narrower width");
        self.check_width(width);
        if width == wa {
            return a;
        }
        self.push(Node::ZeroExt(a), width)
    }

    /// Sign-extend to `width`.
    pub fn sign_ext(&mut self, a: Term, width: u32) -> Term {
        let wa = self.width(a);
        assert!(width >= wa, "sign_ext to narrower width");
        self.check_width(width);
        if width == wa {
            return a;
        }
        self.push(Node::SignExt(a), width)
    }
    /// Population count. The result has the same width as the operand
    /// (enough to hold `width`), with the count in the low bits.
    pub fn popcount(&mut self, a: Term) -> Term {
        let width = self.width(a);
        self.push(Node::Popcount(a), width)
    }
    /// Evaluate a term under a concrete model. Unassigned variables
    /// evaluate to zero (callers that need totality should assign all).
    pub fn eval(&self, t: Term, model: &FastHashMap<VarId, u64>) -> u64 {
        let width = self.width(t);
        let raw = match self.nodes[t as usize] {
            Node::Const(value) => value,
            Node::Var(id) => model.get(&id).copied().unwrap_or(0),
            Node::Bin(op, a, b) => {
                let av = self.eval(a, model);
                let bv = self.eval(b, model);
                match op {
                    BinOp::And => av & bv,
                    BinOp::Or => av | bv,
                    BinOp::Xor => av ^ bv,
                    BinOp::Add => av.wrapping_add(bv),
                    BinOp::Sub => av.wrapping_sub(bv),
                    BinOp::Mul => (u128::from(av) * u128::from(bv)) as u64,
                    BinOp::Shl => {
                        if bv >= u64::from(self.width(a)) {
                            0
                        } else {
                            av.wrapping_shl(bv as u32)
                        }
                    }
                    BinOp::Lshr => {
                        if bv >= u64::from(self.width(a)) {
                            0
                        } else {
                            av >> bv
                        }
                    }
                    BinOp::Ashr => {
                        let wa = self.width(a);
                        let signed = sign_extend_u64(av, wa);
                        let sh = if bv >= u64::from(wa) {
                            (wa - 1) as u32
                        } else {
                            bv as u32
                        };
                        (signed >> sh) as u64
                    }
                    // SMT-LIB bitvector semantics: division by zero is
                    // all-ones, remainder by zero is the dividend.
                    BinOp::Udiv => {
                        let w = self.width(a);
                        let d = bv & mask(w);
                        if d == 0 {
                            mask(w)
                        } else {
                            (av & mask(w)) / d
                        }
                    }
                    BinOp::Urem => {
                        let w = self.width(a);
                        let d = bv & mask(w);
                        if d == 0 {
                            av & mask(w)
                        } else {
                            (av & mask(w)) % d
                        }
                    }
                    BinOp::Ult => u64::from(av < bv),
                    BinOp::Ule => u64::from(av <= bv),
                    BinOp::Eq => u64::from(av == bv),
                }
            }
            Node::Ite(c, t, e) => {
                if self.eval(c, model) != 0 {
                    self.eval(t, model)
                } else {
                    self.eval(e, model)
                }
            }
            Node::Extract(a, _, lo) => self.eval(a, model) >> lo,
            Node::Concat(high, low) => {
                let wl = self.width(low);
                (self.eval(high, model) << wl) | self.eval(low, model)
            }
            Node::ZeroExt(a) => self.eval(a, model),
            Node::SignExt(a) => {
                let wa = self.width(a);
                sign_extend_u64(self.eval(a, model) & mask(wa), wa)
            }
            Node::Popcount(a) => u64::from(self.eval(a, model).count_ones()),
        };
        raw & mask(width)
    }

    fn push(&mut self, node: Node, width: u32) -> Term {
        assert!(width >= 1 && width <= MAX_WIDTH, "invalid width {width}");
        if let Some(&id) = self.interns.get(&(node.clone(), width)) {
            return id;
        }
        let id = self.nodes.len() as Term;
        self.nodes.push(node.clone());
        self.widths.push(width);
        self.interns.insert((node, width), id);
        id
    }

    fn check_width(&self, width: u32) {
        assert!(width >= 1 && width <= MAX_WIDTH, "invalid width {width}");
    }
}

impl Default for Bv {
    fn default() -> Self {
        Self::new()
    }
}

fn sign_extend_u64(value: u64, width: u32) -> u64 {
    debug_assert!(width >= 1 && width <= 64);
    let sign = 1u64 << (width - 1);
    if value & sign != 0 {
        value | !mask(width)
    } else {
        value & mask(width)
    }
}
