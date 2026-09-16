//! Sorted, hash-consed kernel terms.
//!
//! Gate 4B needs three sorts at once: mathematical integers (lengths,
//! indices, sums), booleans (guards, predicates) and fixed-width
//! bitvectors (bytes, masks, accumulators). Every constructor
//! type-checks its operands, and structurally equal terms share one
//! id, so rewriting is a matter of comparing ids.
//!
//! Application symbols (`App`) are how the object theory enters:
//! array reads, range folds, and the intrinsic models are all declared
//! signatures. The kernel never defines their meaning implicitly —
//! every application is either unfolded by a cited definition or
//! discharged by a cited lemma.

use std::collections::HashMap;
use std::fmt;

/// Kernel sorts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Sort {
    Int,
    Bool,
    Bv(u32),
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sort::Int => write!(f, "Int"),
            Sort::Bool => write!(f, "Bool"),
            Sort::Bv(w) => write!(f, "Bv{w}"),
        }
    }
}

/// A declared variable.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct Symbol(pub u32);

/// A declared uninterpreted function symbol.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct FuncId(pub u32);

/// A kernel term: index into [`Arena`].
pub type Tid = u32;

/// Bitvector operations (mirrors the solver's operation set).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum BvOp {
    And,
    Or,
    Xor,
    Add,
    Sub,
    Mul,
    Shl,
    Lshr,
    Ashr,
    Ult,
    Ule,
    Eq,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Node {
    IntConst(i128),
    BoolConst(bool),
    BvConst(u128, u32),

    Var(Symbol),

    IntAdd(Tid, Tid),
    IntSub(Tid, Tid),
    IntMul(Tid, Tid),
    IntNeg(Tid),
    IntIte(Tid, Tid, Tid),

    IntLt(Tid, Tid),
    IntLe(Tid, Tid),
    IntEq(Tid, Tid),
    BoolNot(Tid),
    BoolAnd(Tid, Tid),
    BoolOr(Tid, Tid),
    BoolIte(Tid, Tid, Tid),
    BoolEq(Tid, Tid),

    BvBin(BvOp, Tid, Tid),
    BvNot(Tid),
    BvNeg(Tid),
    BvIte(Tid, Tid, Tid),
    BvExtract(Tid, u32, u32),
    BvConcat(Tid, Tid),
    BvZeroExt(Tid, u32),
    BvSignExt(Tid, u32),
    BvPopcount(Tid),

    /// Convert a bitvector to an integer (unsigned / zero-extended).
    BvToInt(Tid),
    /// Convert an integer to a bitvector, truncating the low bits.
    IntToBv(Tid, u32),
    /// Convert a boolean to 0/1.
    BoolToInt(Tid),

    App(FuncId, Vec<Tid>),
}

impl Node {
    /// Same operator / constructor, ignoring operands. Used by the
    /// kernel's term matcher (induction-hypothesis instantiation).
    pub fn same_form(&self, other: &Node) -> bool {
        use Node::*;
        match (self, other) {
            (IntConst(a), IntConst(b)) => a == b,
            (BoolConst(a), BoolConst(b)) => a == b,
            (BvConst(a, wa), BvConst(b, wb)) => a == b && wa == wb,
            (Var(a), Var(b)) => a == b,
            (IntAdd(..), IntAdd(..))
            | (IntSub(..), IntSub(..))
            | (IntMul(..), IntMul(..))
            | (IntNeg(..), IntNeg(..))
            | (IntIte(..), IntIte(..))
            | (IntLt(..), IntLt(..))
            | (IntLe(..), IntLe(..))
            | (IntEq(..), IntEq(..))
            | (BoolNot(..), BoolNot(..))
            | (BoolAnd(..), BoolAnd(..))
            | (BoolOr(..), BoolOr(..))
            | (BoolIte(..), BoolIte(..))
            | (BoolEq(..), BoolEq(..))
            | (BvNot(..), BvNot(..))
            | (BvNeg(..), BvNeg(..))
            | (BvIte(..), BvIte(..))
            | (BvConcat(..), BvConcat(..))
            | (BvPopcount(..), BvPopcount(..))
            | (BvToInt(..), BvToInt(..))
            | (BoolToInt(..), BoolToInt(..)) => true,
            (BvBin(a, _, _), BvBin(b, _, _)) => a == b,
            (BvExtract(_, ha, la), BvExtract(_, hb, lb)) => ha == hb && la == lb,
            (BvZeroExt(_, a), BvZeroExt(_, b)) => a == b,
            (BvSignExt(_, a), BvSignExt(_, b)) => a == b,
            (IntToBv(_, a), IntToBv(_, b)) => a == b,
            (App(fa, aa), App(fb, ab)) => fa == fb && aa.len() == ab.len(),
            _ => false,
        }
    }

    /// Operand term ids, in a deterministic order.
    pub fn children(&self) -> Vec<Tid> {
        use Node::*;
        match self {
            IntConst(_) | BoolConst(_) | BvConst(_, _) | Var(_) => vec![],
            IntAdd(a, b)
            | IntSub(a, b)
            | IntMul(a, b)
            | IntLt(a, b)
            | IntLe(a, b)
            | IntEq(a, b)
            | BoolAnd(a, b)
            | BoolOr(a, b)
            | BoolEq(a, b)
            | BvBin(_, a, b)
            | BvConcat(a, b) => vec![*a, *b],
            IntNeg(a)
            | BoolNot(a)
            | BvNot(a)
            | BvNeg(a)
            | BvPopcount(a)
            | BvToInt(a)
            | BoolToInt(a)
            | BvZeroExt(a, _)
            | BvSignExt(a, _)
            | IntToBv(a, _)
            | BvExtract(a, _, _) => vec![*a],
            IntIte(c, x, y) | BoolIte(c, x, y) | BvIte(c, x, y) => vec![*c, *x, *y],
            App(_, args) => args.clone(),
        }
    }
}

#[derive(Clone, Debug)]
struct FuncInfo {
    name: String,
    arg_sorts: Vec<Sort>,
    ret_sort: Sort,
}

#[derive(Clone, Debug, Default)]
pub struct Arena {
    nodes: Vec<Node>,
    sorts: Vec<Sort>,
    interns: HashMap<Node, Tid>,
    var_names: Vec<(Symbol, String, Sort)>,
    funcs: Vec<FuncInfo>,
}

impl Arena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn sort(&self, t: Tid) -> Sort {
        self.sorts[t as usize]
    }

    pub fn node(&self, t: Tid) -> &Node {
        &self.nodes[t as usize]
    }

    pub fn var_name(&self, s: Symbol) -> &str {
        self.var_names
            .iter()
            .find(|(sym, _, _)| *sym == s)
            .map(|(_, n, _)| n.as_str())
            .unwrap_or("<undeclared>")
    }

    /// Number of declared variables.
    pub fn symbol_count(&self) -> usize {
        self.var_names.len()
    }

    pub fn func_name(&self, f: FuncId) -> &str {
        &self.funcs[f.0 as usize].name
    }

    pub fn func_signature(&self, f: FuncId) -> (Vec<Sort>, Sort) {
        let info = &self.funcs[f.0 as usize];
        (info.arg_sorts.clone(), info.ret_sort)
    }

    /// Declare a variable with a name and sort.
    pub fn declare_var(&mut self, name: impl Into<String>, sort: Sort) -> Symbol {
        let id = Symbol(self.var_names.len() as u32);
        self.var_names.push((id, name.into(), sort));
        id
    }

    /// Declare an uninterpreted function symbol.
    pub fn declare_func(
        &mut self,
        name: impl Into<String>,
        arg_sorts: Vec<Sort>,
        ret_sort: Sort,
    ) -> FuncId {
        let id = FuncId(self.funcs.len() as u32);
        self.funcs.push(FuncInfo {
            name: name.into(),
            arg_sorts,
            ret_sort,
        });
        id
    }

    pub fn var(&mut self, symbol: Symbol) -> Tid {
        let sort = self
            .var_names
            .iter()
            .find(|(s, _, _)| *s == symbol)
            .map(|(_, _, sort)| *sort)
            .unwrap_or_else(|| panic!("undeclared symbol {symbol:?}"));
        self.intern(Node::Var(symbol), sort)
    }

    pub fn int(&mut self, v: i128) -> Tid {
        self.intern(Node::IntConst(v), Sort::Int)
    }

    pub fn bool(&mut self, v: bool) -> Tid {
        self.intern(Node::BoolConst(v), Sort::Bool)
    }

    pub fn bv(&mut self, v: u128, width: u32) -> Tid {
        assert!(width >= 1 && width <= 64, "invalid width {width}");
        let masked = if width == 128 {
            v
        } else {
            v & ((1u128 << width) - 1)
        };
        self.intern(Node::BvConst(masked, width), Sort::Bv(width))
    }

    pub fn add(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Int, "add lhs");
        self.expect(b, Sort::Int, "add rhs");
        self.intern(Node::IntAdd(a, b), Sort::Int)
    }

    pub fn sub(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Int, "sub lhs");
        self.expect(b, Sort::Int, "sub rhs");
        self.intern(Node::IntSub(a, b), Sort::Int)
    }

    pub fn mul(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Int, "mul lhs");
        self.expect(b, Sort::Int, "mul rhs");
        self.intern(Node::IntMul(a, b), Sort::Int)
    }

    pub fn neg(&mut self, a: Tid) -> Tid {
        self.expect(a, Sort::Int, "neg");
        self.intern(Node::IntNeg(a), Sort::Int)
    }

    pub fn int_ite(&mut self, c: Tid, t: Tid, e: Tid) -> Tid {
        self.expect(c, Sort::Bool, "ite condition");
        self.expect(t, Sort::Int, "ite then");
        self.expect(e, Sort::Int, "ite else");
        self.intern(Node::IntIte(c, t, e), Sort::Int)
    }

    pub fn lt(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Int, "lt lhs");
        self.expect(b, Sort::Int, "lt rhs");
        self.intern(Node::IntLt(a, b), Sort::Bool)
    }

    pub fn le(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Int, "le lhs");
        self.expect(b, Sort::Int, "le rhs");
        self.intern(Node::IntLe(a, b), Sort::Bool)
    }

    pub fn int_eq(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Int, "int_eq lhs");
        self.expect(b, Sort::Int, "int_eq rhs");
        self.intern(Node::IntEq(a, b), Sort::Bool)
    }

    pub fn bool_not(&mut self, a: Tid) -> Tid {
        self.expect(a, Sort::Bool, "bool_not");
        self.intern(Node::BoolNot(a), Sort::Bool)
    }

    pub fn bool_and(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Bool, "and lhs");
        self.expect(b, Sort::Bool, "and rhs");
        self.intern(Node::BoolAnd(a, b), Sort::Bool)
    }

    pub fn bool_or(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Bool, "or lhs");
        self.expect(b, Sort::Bool, "or rhs");
        self.intern(Node::BoolOr(a, b), Sort::Bool)
    }

    pub fn bool_ite(&mut self, c: Tid, t: Tid, e: Tid) -> Tid {
        self.expect(c, Sort::Bool, "bool ite condition");
        self.expect(t, Sort::Bool, "bool ite then");
        self.expect(e, Sort::Bool, "bool ite else");
        self.intern(Node::BoolIte(c, t, e), Sort::Bool)
    }

    pub fn bool_eq(&mut self, a: Tid, b: Tid) -> Tid {
        self.expect(a, Sort::Bool, "bool_eq lhs");
        self.expect(b, Sort::Bool, "bool_eq rhs");
        self.intern(Node::BoolEq(a, b), Sort::Bool)
    }

    pub fn bv_bin(&mut self, op: BvOp, a: Tid, b: Tid) -> Tid {
        let (sa, sb) = (self.sort(a), self.sort(b));
        let (wa, wb) = match (sa, sb) {
            (Sort::Bv(x), Sort::Bv(y)) => (x, y),
            _ => panic!("bv_bin on non-bitvector operands: {sa:?}, {sb:?}"),
        };
        assert_eq!(wa, wb, "bv_bin width mismatch");
        let ret = match op {
            BvOp::Ult | BvOp::Ule | BvOp::Eq => Sort::Bool,
            _ => Sort::Bv(wa),
        };
        self.intern(Node::BvBin(op, a, b), ret)
    }

    pub fn bv_not(&mut self, a: Tid) -> Tid {
        let w = self.bv_width(a, "bv_not");
        self.intern(Node::BvNot(a), Sort::Bv(w))
    }

    pub fn bv_neg(&mut self, a: Tid) -> Tid {
        let w = self.bv_width(a, "bv_neg");
        self.intern(Node::BvNeg(a), Sort::Bv(w))
    }

    pub fn bv_ite(&mut self, c: Tid, t: Tid, e: Tid) -> Tid {
        self.expect(c, Sort::Bool, "bv_ite condition");
        let wt = self.bv_width(t, "bv_ite then");
        let we = self.bv_width(e, "bv_ite else");
        assert_eq!(wt, we, "bv_ite width mismatch");
        self.intern(Node::BvIte(c, t, e), Sort::Bv(wt))
    }

    pub fn bv_extract(&mut self, a: Tid, hi: u32, lo: u32) -> Tid {
        let w = self.bv_width(a, "bv_extract");
        assert!(lo <= hi && hi < w, "extract {hi}..{lo} of width {w}");
        self.intern(Node::BvExtract(a, hi, lo), Sort::Bv(hi - lo + 1))
    }

    pub fn bv_concat(&mut self, high: Tid, low: Tid) -> Tid {
        let wh = self.bv_width(high, "concat high");
        let wl = self.bv_width(low, "concat low");
        let w = wh + wl;
        assert!(w <= 64, "concat width {w} exceeds 64");
        self.intern(Node::BvConcat(high, low), Sort::Bv(w))
    }

    pub fn bv_zero_ext(&mut self, a: Tid, width: u32) -> Tid {
        let w = self.bv_width(a, "zero_ext");
        assert!(width >= w && width <= 64, "invalid zero_ext width {width}");
        if width == w {
            return a;
        }
        self.intern(Node::BvZeroExt(a, width), Sort::Bv(width))
    }

    pub fn bv_sign_ext(&mut self, a: Tid, width: u32) -> Tid {
        let w = self.bv_width(a, "sign_ext");
        assert!(width >= w && width <= 64, "invalid sign_ext width {width}");
        if width == w {
            return a;
        }
        self.intern(Node::BvSignExt(a, width), Sort::Bv(width))
    }

    pub fn bv_popcount(&mut self, a: Tid) -> Tid {
        let w = self.bv_width(a, "popcount");
        self.intern(Node::BvPopcount(a), Sort::Bv(w))
    }

    /// Build a bitvector from 1-bit terms, LSB first. The total width
    /// must not exceed 64.
    pub fn from_bits(&mut self, bits: &[Tid]) -> Option<Tid> {
        if bits.is_empty() || bits.len() > 64 {
            return None;
        }
        for &b in bits {
            if self.bv_width(b, "from_bits") != 1 {
                return None;
            }
        }
        let mut acc = bits[0];
        for &b in &bits[1..] {
            acc = self.bv_concat(b, acc);
        }
        Some(acc)
    }

    pub fn bv_to_int(&mut self, a: Tid) -> Tid {
        self.bv_width(a, "bv_to_int");
        self.intern(Node::BvToInt(a), Sort::Int)
    }

    pub fn int_to_bv(&mut self, a: Tid, width: u32) -> Tid {
        self.expect(a, Sort::Int, "int_to_bv");
        assert!(width >= 1 && width <= 64, "invalid width {width}");
        self.intern(Node::IntToBv(a, width), Sort::Bv(width))
    }

    pub fn bool_to_int(&mut self, a: Tid) -> Tid {
        self.expect(a, Sort::Bool, "bool_to_int");
        self.intern(Node::BoolToInt(a), Sort::Int)
    }

    /// Apply an uninterpreted function symbol.
    pub fn app(&mut self, f: FuncId, args: &[Tid]) -> Tid {
        let (arg_sorts, ret_sort) = self.func_signature(f);
        assert_eq!(
            arg_sorts.len(),
            args.len(),
            "arity mismatch for function {}",
            self.func_name(f)
        );
        for (i, (&arg, &expected)) in args.iter().zip(arg_sorts.iter()).enumerate() {
            let actual = self.sort(arg);
            assert_eq!(
                actual,
                expected,
                "argument {i} of {}: expected {expected}, got {actual}",
                self.func_name(f)
            );
        }
        self.intern(Node::App(f, args.to_vec()), ret_sort)
    }

    /// Substitute variables (by term id) in `t`, rebuilding hash-consed
    /// terms. Any variable absent from the map is left alone.
    pub fn substitute(&mut self, t: Tid, map: &HashMap<Tid, Tid>) -> Tid {
        if let Some(&replacement) = map.get(&t) {
            assert_eq!(
                self.sort(replacement),
                self.sort(t),
                "substitution changes sort"
            );
            return replacement;
        }
        let node = self.nodes[t as usize].clone();
        let rebuilt = match node {
            Node::IntConst(_) | Node::BoolConst(_) | Node::BvConst(_, _) | Node::Var(_) => {
                return t
            }
            Node::IntAdd(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.add(a, b)
            }
            Node::IntSub(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.sub(a, b)
            }
            Node::IntMul(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.mul(a, b)
            }
            Node::IntNeg(a) => {
                let a = self.substitute(a, map);
                self.neg(a)
            }
            Node::IntIte(c, x, y) => {
                let (c, x, y) = (
                    self.substitute(c, map),
                    self.substitute(x, map),
                    self.substitute(y, map),
                );
                self.int_ite(c, x, y)
            }
            Node::IntLt(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.lt(a, b)
            }
            Node::IntLe(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.le(a, b)
            }
            Node::IntEq(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.int_eq(a, b)
            }
            Node::BoolNot(a) => {
                let a = self.substitute(a, map);
                self.bool_not(a)
            }
            Node::BoolAnd(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.bool_and(a, b)
            }
            Node::BoolOr(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.bool_or(a, b)
            }
            Node::BoolIte(c, x, y) => {
                let (c, x, y) = (
                    self.substitute(c, map),
                    self.substitute(x, map),
                    self.substitute(y, map),
                );
                self.bool_ite(c, x, y)
            }
            Node::BoolEq(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.bool_eq(a, b)
            }
            Node::BvBin(op, a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.bv_bin(op, a, b)
            }
            Node::BvNot(a) => {
                let a = self.substitute(a, map);
                self.bv_not(a)
            }
            Node::BvNeg(a) => {
                let a = self.substitute(a, map);
                self.bv_neg(a)
            }
            Node::BvIte(c, x, y) => {
                let (c, x, y) = (
                    self.substitute(c, map),
                    self.substitute(x, map),
                    self.substitute(y, map),
                );
                self.bv_ite(c, x, y)
            }
            Node::BvExtract(a, hi, lo) => {
                let a = self.substitute(a, map);
                self.bv_extract(a, hi, lo)
            }
            Node::BvConcat(a, b) => {
                let (a, b) = (self.substitute(a, map), self.substitute(b, map));
                self.bv_concat(a, b)
            }
            Node::BvZeroExt(a, w) => {
                let a = self.substitute(a, map);
                self.bv_zero_ext(a, w)
            }
            Node::BvSignExt(a, w) => {
                let a = self.substitute(a, map);
                self.bv_sign_ext(a, w)
            }
            Node::BvPopcount(a) => {
                let a = self.substitute(a, map);
                self.bv_popcount(a)
            }
            Node::BvToInt(a) => {
                let a = self.substitute(a, map);
                self.bv_to_int(a)
            }
            Node::IntToBv(a, w) => {
                let a = self.substitute(a, map);
                self.int_to_bv(a, w)
            }
            Node::BoolToInt(a) => {
                let a = self.substitute(a, map);
                self.bool_to_int(a)
            }
            Node::App(f, args) => {
                let args: Vec<Tid> = args.iter().map(|&a| self.substitute(a, map)).collect();
                self.app(f, &args)
            }
        };
        rebuilt
    }

    /// Render a term in a readable, deterministic form.
    pub fn display(&self, t: Tid) -> String {
        let mut out = String::new();
        self.write_term(t, &mut out);
        out
    }

    fn write_term(&self, t: Tid, out: &mut String) {
        match self.nodes[t as usize].clone() {
            Node::IntConst(v) => out.push_str(&format!("{v}")),
            Node::BoolConst(v) => out.push_str(if v { "true" } else { "false" }),
            Node::BvConst(v, w) => out.push_str(&format!("0x{v:x}:bv{w}")),
            Node::Var(s) => out.push_str(self.var_name(s)),
            Node::IntAdd(a, b) => self.write_infix(a, b, " + ", out),
            Node::IntSub(a, b) => self.write_infix(a, b, " - ", out),
            Node::IntMul(a, b) => self.write_infix(a, b, " * ", out),
            Node::IntNeg(a) => {
                out.push('-');
                self.write_par(t, a, out);
            }
            Node::IntIte(c, x, y) => {
                out.push_str("ite(");
                self.write_term(c, out);
                out.push_str(", ");
                self.write_term(x, out);
                out.push_str(", ");
                self.write_term(y, out);
                out.push(')');
            }
            Node::IntLt(a, b) => self.write_infix(a, b, " < ", out),
            Node::IntLe(a, b) => self.write_infix(a, b, " <= ", out),
            Node::IntEq(a, b) => self.write_infix(a, b, " = ", out),
            Node::BoolNot(a) => {
                out.push('!');
                self.write_par(t, a, out);
            }
            Node::BoolAnd(a, b) => self.write_infix(a, b, " && ", out),
            Node::BoolOr(a, b) => self.write_infix(a, b, " || ", out),
            Node::BoolIte(c, x, y) => {
                out.push_str("ite(");
                self.write_term(c, out);
                out.push_str(", ");
                self.write_term(x, out);
                out.push_str(", ");
                self.write_term(y, out);
                out.push(')');
            }
            Node::BoolEq(a, b) => self.write_infix(a, b, " <=> ", out),
            Node::BvBin(op, a, b) => {
                self.write_infix(a, b, &format!(" {} ", bv_op_symbol(op)), out)
            }
            Node::BvNot(a) => {
                out.push('~');
                self.write_par(t, a, out);
            }
            Node::BvNeg(a) => {
                out.push('-');
                self.write_par(t, a, out);
            }
            Node::BvIte(c, x, y) => {
                out.push_str("ite(");
                self.write_term(c, out);
                out.push_str(", ");
                self.write_term(x, out);
                out.push_str(", ");
                self.write_term(y, out);
                out.push(')');
            }
            Node::BvExtract(a, hi, lo) => {
                self.write_term(a, out);
                out.push_str(&format!("[{hi}:{lo}]"));
            }
            Node::BvConcat(a, b) => {
                self.write_term(a, out);
                out.push_str(" ++ ");
                self.write_term(b, out);
            }
            Node::BvZeroExt(a, w) => {
                out.push_str(&format!("zext{w}("));
                self.write_term(a, out);
                out.push(')');
            }
            Node::BvSignExt(a, w) => {
                out.push_str(&format!("sext{w}("));
                self.write_term(a, out);
                out.push(')');
            }
            Node::BvPopcount(a) => {
                out.push_str("popcount(");
                self.write_term(a, out);
                out.push(')');
            }
            Node::BvToInt(a) => {
                out.push_str("int(");
                self.write_term(a, out);
                out.push(')');
            }
            Node::IntToBv(a, w) => {
                out.push_str(&format!("trunc{w}("));
                self.write_term(a, out);
                out.push(')');
            }
            Node::BoolToInt(a) => {
                out.push_str("if(");
                self.write_term(a, out);
                out.push_str(", 1, 0)");
            }
            Node::App(f, args) => {
                out.push_str(self.func_name(f));
                out.push('(');
                for (i, &a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write_term(a, out);
                }
                out.push(')');
            }
        }
    }

    fn write_infix(&self, a: Tid, b: Tid, op: &str, out: &mut String) {
        self.write_term(a, out);
        out.push_str(op);
        self.write_term(b, out);
    }

    fn write_par(&self, parent: Tid, child: Tid, out: &mut String) {
        let needs = matches!(
            self.nodes[child as usize],
            Node::IntAdd(_, _) | Node::IntSub(_, _) | Node::IntMul(_, _)
        );
        if needs {
            out.push('(');
            self.write_term(child, out);
            out.push(')');
        } else {
            self.write_term(child, out);
        }
        let _ = parent;
    }

    fn bv_width(&self, t: Tid, what: &str) -> u32 {
        match self.sort(t) {
            Sort::Bv(w) => w,
            other => panic!("{what}: expected bitvector, got {other:?}"),
        }
    }

    fn expect(&self, t: Tid, expected: Sort, what: &str) {
        let actual = self.sort(t);
        assert_eq!(
            actual, expected,
            "{what}: expected {expected}, got {actual}"
        );
    }

    fn intern(&mut self, node: Node, sort: Sort) -> Tid {
        if let Some(&id) = self.interns.get(&node) {
            if self.sorts[id as usize] != sort {
                // Cannot happen: the sort is a function of the node.
                panic!("hash-consing sort conflict");
            }
            return id;
        }
        let id = self.nodes.len() as Tid;
        self.nodes.push(node.clone());
        self.sorts.push(sort);
        self.interns.insert(node, id);
        id
    }
}

fn bv_op_symbol(op: BvOp) -> &'static str {
    match op {
        BvOp::And => "&",
        BvOp::Or => "|",
        BvOp::Xor => "^",
        BvOp::Add => "+bv",
        BvOp::Sub => "-bv",
        BvOp::Mul => "*bv",
        BvOp::Shl => "<<",
        BvOp::Lshr => ">>u",
        BvOp::Ashr => ">>s",
        BvOp::Ult => "<u",
        BvOp::Ule => "<=u",
        BvOp::Eq => "==bv",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_consing_shares_structure() {
        let mut a = Arena::new();
        let x = a.declare_var("x", Sort::Int);
        let y = a.declare_var("y", Sort::Int);
        let (x, y) = (a.var(x), a.var(y));
        let s1 = a.add(x, y);
        let s2 = a.add(x, y);
        assert_eq!(s1, s2, "structurally equal terms must share an id");
    }

    #[test]
    fn substitution_rebuilds() {
        let mut a = Arena::new();
        let x = a.declare_var("x", Sort::Int);
        let y = a.declare_var("y", Sort::Int);
        let (x, y) = (a.var(x), a.var(y));
        let sum = a.add(x, y);
        let five = a.int(5);
        let mut map = HashMap::new();
        map.insert(x, five);
        let out = a.substitute(sum, &map);
        assert_eq!(a.display(out), "5 + y");
    }

    #[test]
    fn app_signature_checked() {
        let mut a = Arena::new();
        let f = a.declare_func("read", vec![Sort::Int], Sort::Bv(8));
        let i = a.declare_var("i", Sort::Int);
        let i = a.var(i);
        let r = a.app(f, &[i]);
        assert_eq!(a.sort(r), Sort::Bv(8));
        assert_eq!(a.display(r), "read(i)");
    }

    #[test]
    fn sort_display_and_constants() {
        let mut a = Arena::new();
        let b = a.bv(0x1FF, 8);
        assert_eq!(a.display(b), "0xff:bv8");
        let c = a.bool(true);
        assert_eq!(a.display(c), "true");
    }
}
