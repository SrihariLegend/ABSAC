//! Symbolic lowering: SIR → bitvector terms, under a substitution
//! environment.
//!
//! This exists so that a transformation can be proved *about itself* — the
//! recognised original and the constructed patch lowered to terms in one
//! language and queried for equality — instead of a general law being
//! proved beside the construction and the construction trusted.
//!
//! Two properties are load-bearing and both are deliberate.
//!
//! **The memo is per-call, never shared across environments.** An unroller
//! visits a loop body once per iteration, each time with a different
//! binding for the counter. A memo keyed by `NodeId` alone would return the
//! first iteration's term for every later iteration: the original would
//! lower to the same expression as a patch that read `a[0]` throughout, and
//! the equality query would then *certify* a transformation that reads the
//! wrong element. That is the specific defect this module is shaped to
//! avoid, so `lower` mints a fresh memo and the environment is consulted
//! before the memo, never after.
//!
//! **Every operator is explicit and everything else refuses.** A closed
//! mapping with exact width, signedness, boolean and conversion semantics,
//! and `None` for any kind not listed. An operator that lowers *nearly*
//! right — a signed comparison lowered unsigned, say — would validate
//! rewrites that are wrong for exactly the inputs that distinguish them,
//! which is worse than refusing.
//!
//! Array reads are modelled without array theory: each `(array, concrete
//! index)` pair is one symbolic variable, shared across every call on this
//! translator, so the same read in the original and in the patch denotes
//! the same value. An index that is not concrete after substitution
//! refuses — independent variables for possibly-aliasing reads would be
//! conservative but could not prove anything, and sharing them without a
//! common key would be unsound.

use std::collections::{BTreeMap, HashMap};

use sir_mech::{Bv, Term, VarId, MAX_WIDTH};
use sir_nodes::{ConvertKind, Function, NodeKind};
use sir_types::{ConstantData, IntegerWidth, NodeId, Type};

/// Values fixed by the caller for this lowering: the loop counter on one
/// particular iteration, the carried value entering it.
#[derive(Clone, Default, Debug)]
pub struct Subst {
    map: BTreeMap<NodeId, Term>,
}

impl Subst {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, node: NodeId, term: Term) {
        self.map.insert(node, term);
    }

    pub fn get(&self, node: NodeId) -> Option<Term> {
        self.map.get(&node).copied()
    }
}

/// Lowers SIR nodes to bitvector terms.
pub struct Translator<'a> {
    function: &'a Function,
    bv: Bv,
    /// One variable per `(array, index)` read, shared across calls.
    elements: HashMap<(NodeId, u64), Term>,
    /// Symbolic inputs that are not array reads: parameters and any
    /// arithmetic the caller chose not to fix.
    inputs: HashMap<NodeId, Term>,
    next: u32,
}

impl<'a> Translator<'a> {
    pub fn new(function: &'a Function) -> Self {
        Self {
            function,
            bv: Bv::new(),
            elements: HashMap::new(),
            inputs: HashMap::new(),
            next: 0,
        }
    }

    pub fn bv(&self) -> &Bv {
        &self.bv
    }

    pub fn constant(&mut self, value: u64, width: u32) -> Term {
        self.bv.constant(value, width)
    }

    /// A free symbolic input of the given width.
    pub fn fresh(&mut self, width: u32) -> Term {
        let id = VarId(self.next);
        self.next += 1;
        self.bv.var(id, width)
    }

    /// The element read at a concrete index. The same `(array, index)`
    /// yields the same variable on every call, which is what lets the
    /// original and the patch be compared at all.
    pub fn element(&mut self, array: NodeId, index: u64, width: u32) -> Option<Term> {
        if let Some(&term) = self.elements.get(&(array, index)) {
            return Some(term);
        }
        let term = self.fresh(width);
        self.elements.insert((array, index), term);
        Some(term)
    }

    /// Lower `root` under `env`. The memo below is created here and dies
    /// here: it is never consulted across two environments.
    pub fn lower(&mut self, root: NodeId, env: &Subst) -> Option<Term> {
        let mut memo = HashMap::new();
        self.walk(root, env, &mut memo)
    }

    fn walk(
        &mut self,
        id: NodeId,
        env: &Subst,
        memo: &mut HashMap<NodeId, Term>,
    ) -> Option<Term> {
        // The environment wins over the memo, always. A binding states
        // what this node means *for this call*.
        if let Some(term) = env.get(id) {
            return Some(term);
        }
        if let Some(term) = memo.get(&id) {
            return Some(*term);
        }
        let node = self.function.get_node(id)?.clone();
        let term = self.kind(&node.kind, &node.ty, id, env, memo)?;
        memo.insert(id, term);
        Some(term)
    }

    fn kind(
        &mut self,
        kind: &NodeKind,
        ty: &Type,
        id: NodeId,
        env: &Subst,
        memo: &mut HashMap<NodeId, Term>,
    ) -> Option<Term> {
        let width = self.width(ty);
        Some(match kind {
            NodeKind::Constant(data) => self.literal(data, width?)?,
            NodeKind::Parameter { .. } => {
                // A parameter is a free input, but only if it is a scalar:
                // a pointer or array standing here is not modelled, and
                // its elements are reached through `ArrayAccess` instead.
                let width = width?;
                match self.inputs.get(&id) {
                    Some(&term) => term,
                    None => {
                        let term = self.fresh(width);
                        self.inputs.insert(id, term);
                        term
                    }
                }
            }
            NodeKind::ArrayAccess { base, index } => {
                // The index must be concrete after substitution. A
                // symbolic index has no common key, and inventing one per
                // occurrence would make two sides of a proof that should
                // agree denote different memory.
                let key = self.constant_index(*index, env)?;
                let width = width?;
                self.element(*base, key, width)?
            }
            NodeKind::Convert {
                operand,
                target_ty,
                kind,
            } => {
                let a = self.walk(*operand, env, memo)?;
                let to = self.width(target_ty)?;
                match kind {
                    ConvertKind::ZeroExtend => self.bv.zero_ext(a, to),
                    ConvertKind::SignExtend => self.bv.sign_ext(a, to),
                    ConvertKind::Truncate => {
                        if to == 0 || to > self.bv.width(a) {
                            return None;
                        }
                        self.bv.extract(a, to - 1, 0)
                    }
                }
            }
            NodeKind::Add { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::add)?,
            NodeKind::Sub { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::sub)?,
            NodeKind::Mul { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::mul)?,
            NodeKind::And { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::and)?,
            NodeKind::Or { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::or)?,
            NodeKind::Xor { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::xor)?,
            NodeKind::Shl { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::shl)?,
            NodeKind::Shr { lhs, rhs } => {
                // Logical shift only. An arithmetic shift on a signed
                // operand is a different function, and lowering one as the
                // other is precisely the "nearly right operator" this
                // module refuses.
                if matches!(&self.function.get_node(*lhs)?.ty, Type::Integer { signed: true, .. }) {
                    return None;
                }
                self.binary(*lhs, *rhs, env, memo, Bv::lshr)?
            }
            NodeKind::Lt { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::ult)?,
            NodeKind::Le { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::ule)?,
            NodeKind::Gt { lhs, rhs } => {
                // `a > b` is `b < a`; the operands swap, the width does not.
                let b = self.walk(*rhs, env, memo)?;
                let a = self.walk(*lhs, env, memo)?;
                self.bv.ult(b, a)
            }
            NodeKind::Ge { lhs, rhs } => {
                let b = self.walk(*rhs, env, memo)?;
                let a = self.walk(*lhs, env, memo)?;
                self.bv.ule(b, a)
            }
            NodeKind::Eq { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::eq)?,
            NodeKind::Ne { lhs, rhs } => {
                let a = self.walk(*lhs, env, memo)?;
                let b = self.walk(*rhs, env, memo)?;
                let eq = self.bv.eq(a, b);
                self.bv.not(eq)
            }
            NodeKind::BoolAnd { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::and)?,
            NodeKind::BoolOr { lhs, rhs } => self.binary(*lhs, *rhs, env, memo, Bv::or)?,
            NodeKind::BoolNot { operand } => {
                let a = self.walk(*operand, env, memo)?;
                self.bv.not(a)
            }
            NodeKind::Select {
                cond,
                true_val,
                false_val,
            } => {
                let c = self.walk(*cond, env, memo)?;
                let t = self.walk(*true_val, env, memo)?;
                let f = self.walk(*false_val, env, memo)?;
                self.bv.ite(c, t, f)
            }
            // Everything else refuses. This is the closed whitelist: an
            // operator absent from it is one whose semantics this module
            // does not claim to know.
            _ => return None,
        })
    }

    /// The concrete index of an array access after substitution, or `None`
    /// when it is not concrete — in which case the access is refused
    /// rather than given a variable no other read could share.
    fn constant_index(&self, index: NodeId, env: &Subst) -> Option<u64> {
        if let Some(term) = env.get(index) {
            return match self.bv.node(term) {
                sir_mech::bv::Node::Const(value) => Some(value),
                _ => None,
            };
        }
        match &self.function.get_node(index)?.kind {
            NodeKind::Constant(data) => data.as_u64(),
            _ => None,
        }
    }

    fn binary(
        &mut self,
        lhs: NodeId,
        rhs: NodeId,
        env: &Subst,
        memo: &mut HashMap<NodeId, Term>,
        op: fn(&mut Bv, Term, Term) -> Term,
    ) -> Option<Term> {
        let a = self.walk(lhs, env, memo)?;
        let b = self.walk(rhs, env, memo)?;
        Some(op(&mut self.bv, a, b))
    }

    fn literal(&mut self, data: &ConstantData, width: u32) -> Option<Term> {
        match data {
            ConstantData::Bool(value) => Some(self.bv.constant(u64::from(*value), 1)),
            ConstantData::Integer {
                value,
                signed: false,
                ..
            } => {
                let parsed: u64 = value.parse().ok()?;
                Some(self.bv.constant(parsed, width))
            }
            ConstantData::Integer {
                value,
                signed: true,
                ..
            } => {
                // Two's complement: parse as signed and keep the low
                // `width` bits.
                let parsed: i128 = value.parse().ok()?;
                let masked = (parsed as u128 & mask_of(width)) as u64;
                Some(self.bv.constant(masked, width))
            }
            _ => None,
        }
    }

    /// The bitvector width of a type, or `None` for a type this module
    /// does not model.
    fn width(&self, ty: &Type) -> Option<u32> {
        let width = match ty {
            Type::Bool => 1,
            Type::BitVector { width } => *width as u32,
            Type::Integer { width, .. } => match width {
                IntegerWidth::I8 => 8,
                IntegerWidth::I16 => 16,
                IntegerWidth::I32 => 32,
                IntegerWidth::I64 => 64,
                IntegerWidth::I128 => return None,
            },
            _ => return None,
        };
        if width == 0 || width > MAX_WIDTH {
            return None;
        }
        Some(width)
    }
}

fn mask_of(width: u32) -> u128 {
    if width >= 128 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    }
}