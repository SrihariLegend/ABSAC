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

    /// The term builder, for callers that assemble a candidate term to
    /// compare against a lowered original.
    pub fn bv_mut(&mut self) -> &mut Bv {
        &mut self.bv
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
                // Definedness, and it is not implied by the equality query.
                // A read outside the object is undefined however its value
                // is used: `a[N] & 0` is numerically null and would compare
                // equal to anything, so value equality alone would accept an
                // out-of-bounds read folded into a result. The extent must
                // be known and the index must be inside it.
                let length = match self.function.get_node(*base).map(|node| &node.ty) {
                    Some(Type::Array { length, .. }) => *length as u64,
                    // No declared extent: the read cannot be shown safe.
                    _ => return None,
                };
                if key >= length {
                    return None;
                }
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
            NodeKind::Shl { lhs, rhs } => {
                let a = self.walk(*lhs, env, memo)?;
                let b = self.walk(*rhs, env, memo)?;
                self.shift_amount(b, a)?;
                self.bv.shl(a, b)
            }
            NodeKind::Shr { lhs, rhs } => {
                // Logical shift only. An arithmetic shift on a signed
                // operand is a different function, and lowering one as the
                // other is precisely the "nearly right operator" this
                // module refuses.
                if matches!(&self.function.get_node(*lhs)?.ty, Type::Integer { signed: true, .. }) {
                    return None;
                }
                let a = self.walk(*lhs, env, memo)?;
                let b = self.walk(*rhs, env, memo)?;
                self.shift_amount(b, a)?;
                self.bv.lshr(a, b)
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

    /// A shift by at least the operand's width is undefined in the emitted
    /// language, and `Bv` totalises it, so the term language cannot be
    /// trusted to have noticed. A concrete amount is checked; a symbolic
    /// one is refused rather than assumed in range.
    fn shift_amount(&self, amount: Term, value: Term) -> Option<()> {
        let width = self.bv.width(value);
        match self.bv.node(amount) {
            sir_mech::bv::Node::Const(value) => (value < width as u64).then_some(()),
            _ => None,
        }
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

/// One loop, fully unrolled.
pub struct Unrolled {
    pub trip_count: u64,
    /// The final value of each carried variable, in the loop's own output
    /// order, so every live-out is covered and not only the slot a family
    /// happens to want.
    pub outputs: Vec<Term>,
}

/// Unroll a counted loop into terms, deriving each iteration's values from
/// the loop's own expressions and checking them against the structural
/// facts rather than taking those facts for granted.
///
/// The distinction matters and is the whole point of doing it this way. It
/// would be easy — and wrong — to read a trip count out of `sir_analysis`,
/// then manufacture the original's meaning by substituting the counter
/// with `0, 1, …, n-1`. That does not prove the substitution; it *assumes*
/// it while constructing the very term the proof is supposed to be about,
/// and it is the reason a shared mistake in an analysis fact would become a
/// shared mistake in the proof.
///
/// So instead: the counter's value at each step is computed from the loop's
/// own output expression for it, and the termination node is translated
/// and required to fold to a **constant** at every step — the same constant
/// throughout the iterations and a *different* one at the count. That last
/// condition is what validates the count against the actual exit
/// expression, and it is deliberately stated without assuming which
/// polarity the termination uses: "constant, then different" is the loop
/// running exactly `n` times either way.
pub fn unroll(
    translator: &mut Translator<'_>,
    function: &Function,
    loop_id: NodeId,
    outer: &Subst,
) -> Option<Unrolled> {
    let shape = sir_analysis::loop_shape::checked_counted_loop(function, loop_id)?;
    let node = function.get_node(loop_id)?;
    let NodeKind::Loop {
        body,
        termination,
        outputs,
        carried_inputs,
    } = &node.kind
    else {
        return None;
    };
    if carried_inputs.len() != outputs.len() || carried_inputs.is_empty() {
        return None;
    }
    // A nested loop is not modelled here and refuses rather than being
    // flattened silently into the outer unrolling.
    if body
        .iter()
        .any(|id| matches!(function.get_node(*id).map(|n| &n.kind), Some(NodeKind::Loop { .. })))
    {
        return None;
    }
    let counter_slot = carried_inputs.iter().position(|id| *id == shape.counter)?;

    // The values the carried variables hold on entry.
    let mut values: Vec<Term> = Vec::with_capacity(carried_inputs.len());
    for &carry in carried_inputs.iter() {
        values.push(translator.lower(carry, outer)?);
    }

    // One extra step: the count must be validated *at* the exit, so the
    // termination is evaluated once past the last iteration.
    let mut entry_termination: Option<u64> = None;
    for k in 0..=shape.trip_count {
        let mut env = outer.clone();
        for (slot, &carry) in carried_inputs.iter().enumerate() {
            env.bind(carry, values[slot]);
        }
        // The counter must be concrete on this iteration, or the indices
        // below are not, and nothing can be looked up.
        let counter_value = match translator.bv().node(values[counter_slot]) {
            sir_mech::bv::Node::Const(value) => value,
            _ => return None,
        };
        let expected = shape
            .counter_init
            .checked_add(shape.step.checked_mul(k)?)?;
        if counter_value != expected {
            // The loop's own step expression disagrees with the count the
            // structural extractor reported. One of the two is wrong and
            // this module does not get to pick.
            return None;
        }

        // The exit condition, translated and required to be constant.
        let term = translator.lower(*termination, &env)?;
        let exiting = match translator.bv().node(term) {
            sir_mech::bv::Node::Const(value) => value,
            _ => return None,
        };
        match entry_termination {
            None => entry_termination = Some(exiting),
            Some(value) if k == shape.trip_count => {
                if exiting == value {
                    // Still continuing at the count: the loop does not run
                    // the number of times claimed.
                    return None;
                }
            }
            Some(value) if exiting != value => return None,
            Some(_) => {}
        }

        if k == shape.trip_count {
            break;
        }
        // Simultaneous update: every right-hand side is evaluated against
        // the *same* pre-iteration values.
        let mut next = Vec::with_capacity(outputs.len());
        for &output in outputs.iter() {
            next.push(translator.lower(output, &env)?);
        }
        values = next;
    }

    Some(Unrolled {
        trip_count: shape.trip_count,
        outputs: values,
    })
}

fn mask_of(width: u32) -> u128 {
    if width >= 128 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    }
}