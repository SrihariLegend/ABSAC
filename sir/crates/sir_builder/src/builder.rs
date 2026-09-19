use sir_nodes::{Function, Node, NodeKind, Param};
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

use crate::BuildError;

/// A type-safe builder for constructing SIR functions.
///
/// The `Builder` wraps a `Function` under construction and provides
/// methods for creating nodes with automatic type checking and effect
/// computation. All node-creation methods return `Result<NodeId, BuildError>`.
///
/// # Node IDs
///
/// NodeIds are auto-generated via an internal counter. The counter starts
/// after the function parameters (if any).
///
/// # Example (conceptual)
///
/// ```text
/// let mut b = Builder::new("add", &[("a", Type::i32()), ("b", Type::i32())], Type::i32());
/// let a = b.parameter_index(0).unwrap();
/// let b_param = b.parameter_index(1).unwrap();
/// let sum = b.add(a, b_param, Span::unknown())?;
/// b.return_value(sum, Span::unknown())?;
/// let func = b.build();
/// ```
pub struct Builder {
    func: Function,
    next_id: u64,
    escaping_geps: std::collections::HashSet<String>,
}

impl Builder {
    /// Create a new builder for a function.
    ///
    /// Creates `Parameter` nodes for each param, stores them in the arena,
    /// and returns the builder ready for further construction.
    pub fn new(name: impl Into<String>, params: &[(&str, Type)], return_ty: Type) -> Self {
        let mut func = Function::new(name, return_ty);
        let mut next_id = 0u64;

        for (name, ty) in params {
            let id = NodeId::new(next_id);
            next_id += 1;

            let node = Node::new(
                id,
                NodeKind::Parameter {
                    index: func.params.len(),
                },
                ty.clone(),
                Effects::empty(),
                Span::unknown(),
            );
            func.params.push(Param::new(*name, ty.clone()));
            func.arena.insert(node);
        }

        Self {
            func,
            next_id,
            escaping_geps: std::collections::HashSet::new(),
        }
    }

    /// Record GEP results that must stay pointer-valued (used by phis,
    /// other GEPs, selects, calls, or comparisons) rather than folded
    /// into an immediate memory access.
    pub fn set_escaping_geps(&mut self, names: std::collections::HashSet<String>) {
        self.escaping_geps = names;
    }

    /// Whether a GEP result is used as a pointer value.
    pub fn gep_escapes(&self, name: &str) -> bool {
        self.escaping_geps.contains(name)
    }

    // ── Accessors ───────────────────────────────────────────

    /// Get the `NodeId` for a parameter by its index.
    pub fn parameter_index(&self, index: usize) -> Option<NodeId> {
        if index < self.func.params.len() {
            Some(NodeId::new(index as u64))
        } else {
            None
        }
    }
    /// Get a reference to the internal function (for reading).
    pub fn function(&self) -> &Function {
        &self.func
    }

    // ── Finalization ────────────────────────────────────────

    /// Finalize construction and return the `Function`.
    pub fn build(self) -> Function {
        self.func
    }

    // ── Internal helpers ────────────────────────────────────

    fn next_node_id(&mut self) -> NodeId {
        let id = NodeId::new(self.next_id);
        self.next_id += 1;
        id
    }

    fn get_node(&self, id: NodeId) -> Result<&Node, BuildError> {
        self.func.arena.get(id).ok_or(BuildError::NodeNotFound(id))
    }

    fn get_type(&self, id: NodeId) -> Result<Type, BuildError> {
        self.get_node(id).map(|n| n.ty.clone())
    }

    /// Expect that `id` has exactly type `expected`.
    fn expect_type(&self, id: NodeId, expected: &Type) -> Result<(), BuildError> {
        let actual = self.get_type(id)?;
        if actual == *expected {
            Ok(())
        } else {
            Err(BuildError::TypeMismatch {
                node: id,
                expected: expected.clone(),
                actual,
            })
        }
    }

    /// Create a node, insert it into the arena, return its NodeId.
    fn alloc_node(&mut self, kind: NodeKind, ty: Type, effects: Effects, span: Span) -> NodeId {
        let id = self.next_node_id();
        let node = Node::new(id, kind, ty, effects, span);
        self.func.arena.insert(node);
        id
    }

    // ── Effects computation ─────────────────────────────────

    /// Compute effects automatically from a NodeKind.
    fn compute_effects(kind: &NodeKind) -> Effects {
        match kind {
            NodeKind::Load { .. } => Effects::READ_MEMORY,
            NodeKind::Store { .. } => Effects::WRITE_MEMORY,
            NodeKind::Allocate { .. } => Effects::ALLOCATE,
            NodeKind::Deallocate { .. } => Effects::WRITE_MEMORY,
            NodeKind::Iterator { .. } => Effects::READ_MEMORY,
            NodeKind::Call { .. } => Effects::READ_MEMORY | Effects::WRITE_MEMORY,
            NodeKind::Loop { .. } => {
                // Loops inherit effects from their body which is computed during builder construction.
                Effects::empty()
            }
            // Pure by default
            _ => Effects::empty(),
        }
    }

    /// Type-check that a node has a numeric type and return it.
    fn expect_numeric(&self, id: NodeId) -> Result<Type, BuildError> {
        let ty = self.get_type(id)?;
        if ty.is_numeric() {
            Ok(ty)
        } else {
            Err(BuildError::TypeMismatch {
                node: id,
                expected: Type::i32(), // placeholder message
                actual: ty,
            })
        }
    }

    /// Type-check that two nodes have the same type and return it.
    fn expect_same_type(&self, lhs: NodeId, rhs: NodeId) -> Result<Type, BuildError> {
        let lt = self.get_type(lhs)?;
        let rt = self.get_type(rhs)?;
        if lt == rt {
            Ok(lt)
        } else {
            Err(BuildError::TypeMismatch {
                node: rhs,
                expected: lt,
                actual: rt,
            })
        }
    }

    /// Expect that id has an integer type (for bitwise operations).
    fn expect_integer(&self, id: NodeId) -> Result<Type, BuildError> {
        let ty = self.get_type(id)?;
        if ty.is_integer() {
            Ok(ty)
        } else {
            Err(BuildError::TypeMismatch {
                node: id,
                expected: Type::Integer {
                    width: sir_types::IntegerWidth::I32,
                    signed: true,
                    overflow: sir_types::OverflowBehavior::Wrapping,
                }, // placeholder
                actual: ty,
            })
        }
    }

    fn expect_integer_or_bitvector(&self, id: NodeId) -> Result<Type, BuildError> {
        let ty = self.get_type(id)?;
        if ty.is_integer_or_bitvector() {
            Ok(ty)
        } else {
            Err(BuildError::TypeMismatch {
                node: id,
                expected: Type::Integer {
                    width: sir_types::IntegerWidth::I32,
                    signed: true,
                    overflow: sir_types::OverflowBehavior::Wrapping,
                }, // placeholder
                actual: ty,
            })
        }
    }

    // ── Generic node creation ───────────────────────────────
    /// Add extra effects to an existing node (e.g., marking a load as volatile).
    pub fn add_effects(&mut self, id: NodeId, extra: Effects) {
        if let Some(node) = self.func.arena.get_mut(id) {
            node.effects |= extra;
        }
    }

    /// Record an LLVM operation flag (nsw/nuw) on a node as metadata.
    ///
    /// `nsw`/`nuw` mean overflow produces POISON, not wrapping. Dropping
    /// these flags would authorize vector reassociation of signed sums
    /// whose behavior differs on overflow (Gate 6A-v2 finding X02).
    /// The metadata is consumed by the reduction certificate's
    /// integer-semantics check.
    pub fn set_overflow_flag(&mut self, id: NodeId, flag: &str) {
        if let Some(node) = self.func.arena.get_mut(id) {
            node.metadata.insert("llvm.overflow", flag);
        }
    }

    // ── Value nodes ─────────────────────────────────────────

    /// Create a constant node.
    pub fn constant(&mut self, data: ConstantData, ty: Type, span: Span) -> NodeId {
        self.alloc_node(NodeKind::Constant(data), ty, Effects::empty(), span)
    }

    /// Address of a named global, modeled as a byte-addressed base.
    pub fn global(&mut self, name: impl Into<String>, span: Span) -> NodeId {
        let ty = Type::Pointer {
            pointee: Box::new(Type::u8()),
            mutable: true,
        };
        self.alloc_node(
            NodeKind::Global { name: name.into() },
            ty,
            Effects::empty(),
            span,
        )
    }

    /// Address of a fixed-size stack buffer.
    pub fn local_alloca(
        &mut self,
        name: impl Into<String>,
        size: usize,
        align: usize,
        span: Span,
    ) -> NodeId {
        let ty = Type::Pointer {
            pointee: Box::new(Type::u8()),
            mutable: true,
        };
        self.alloc_node(
            NodeKind::LocalAlloca {
                name: name.into(),
                size,
                align,
            },
            ty,
            Effects::empty(),
            span,
        )
    }

    // ── Arithmetic (binary) ─────────────────────────────────

    fn binary_arith(
        &mut self,
        make_kind: fn(NodeId, NodeId) -> NodeKind,
        lhs: NodeId,
        rhs: NodeId,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        let ty = self.expect_same_type(lhs, rhs)?;
        self.expect_numeric(lhs)?;
        let kind = make_kind(lhs, rhs);
        let effects = Self::compute_effects(&kind);
        Ok(self.alloc_node(kind, ty, effects, span))
    }

    pub fn add(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_arith(|l, r| NodeKind::Add { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn sub(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_arith(|l, r| NodeKind::Sub { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn mul(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_arith(|l, r| NodeKind::Mul { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn div(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_arith(|l, r| NodeKind::Div { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn rem(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_arith(|l, r| NodeKind::Rem { lhs: l, rhs: r }, lhs, rhs, span)
    }

    // ── Arithmetic (unary) ──────────────────────────────────

    pub fn neg(&mut self, operand: NodeId, span: Span) -> Result<NodeId, BuildError> {
        let ty = self.expect_numeric(operand)?;
        Ok(self.alloc_node(NodeKind::Neg { operand }, ty, Effects::empty(), span))
    }

    // ── Bitwise (binary) ────────────────────────────────────

    fn binary_bitwise(
        &mut self,
        make_kind: fn(NodeId, NodeId) -> NodeKind,
        lhs: NodeId,
        rhs: NodeId,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        let ty = self.expect_same_type(lhs, rhs)?;
        self.expect_integer(lhs)?;
        let kind = make_kind(lhs, rhs);
        let effects = Self::compute_effects(&kind);
        Ok(self.alloc_node(kind, ty, effects, span))
    }

    pub fn bit_and(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_bitwise(|l, r| NodeKind::And { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn bit_or(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_bitwise(|l, r| NodeKind::Or { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn bit_xor(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.binary_bitwise(|l, r| NodeKind::Xor { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn shl(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_integer(lhs)?;
        self.expect_integer(rhs)?;
        let ty = self.get_type(lhs)?;
        Ok(self.alloc_node(NodeKind::Shl { lhs, rhs }, ty, Effects::empty(), span))
    }

    pub fn shr(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_integer(lhs)?;
        self.expect_integer(rhs)?;
        let ty = self.get_type(lhs)?;
        Ok(self.alloc_node(NodeKind::Shr { lhs, rhs }, ty, Effects::empty(), span))
    }

    pub fn rol(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_integer(lhs)?;
        self.expect_integer(rhs)?;
        let ty = self.get_type(lhs)?;
        Ok(self.alloc_node(NodeKind::Rol { lhs, rhs }, ty, Effects::empty(), span))
    }

    pub fn ror(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_integer(lhs)?;
        self.expect_integer(rhs)?;
        let ty = self.get_type(lhs)?;
        Ok(self.alloc_node(NodeKind::Ror { lhs, rhs }, ty, Effects::empty(), span))
    }

    // ── Bitwise (unary) ─────────────────────────────────────

    fn unary_bitwise(
        &mut self,
        make_kind: fn(NodeId) -> NodeKind,
        operand: NodeId,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        let ty = self.expect_integer(operand)?;
        Ok(self.alloc_node(make_kind(operand), ty, Effects::empty(), span))
    }

    pub fn bit_not(&mut self, operand: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.unary_bitwise(|o| NodeKind::Not { operand: o }, operand, span)
    }

    pub fn popcount(&mut self, operand: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_integer_or_bitvector(operand)?;
        Ok(self.alloc_node(
            NodeKind::Popcount { operand },
            Type::i32(),
            Effects::empty(),
            span,
        ))
    }

    pub fn leading_zeros(&mut self, operand: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_integer_or_bitvector(operand)?;
        Ok(self.alloc_node(
            NodeKind::LeadingZeros { operand },
            Type::i32(),
            Effects::empty(),
            span,
        ))
    }

    pub fn trailing_zeros(&mut self, operand: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_integer_or_bitvector(operand)?;
        Ok(self.alloc_node(
            NodeKind::TrailingZeros { operand },
            Type::i32(),
            Effects::empty(),
            span,
        ))
    }
    /// Create a Pack node: packs a boolean array into a bitvector.
    /// The operand must be an Array(Bool) or Slice(Bool) type.
    pub fn array_cmp_mask(
        &mut self,
        array: NodeId,
        scalar: NodeId,
        op: sir_nodes::CmpOperator,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        let ty = self.get_type(array)?;
        let scalar_ty = self.get_type(scalar)?;

        let width = match &ty {
            Type::Array { element, length } if **element == scalar_ty => *length,
            Type::Slice { element } if **element == scalar_ty => 0,
            _ => {
                return Err(BuildError::TypeMismatch {
                    node: array,
                    expected: Type::Array {
                        element: Box::new(scalar_ty),
                        length: 0,
                    },
                    actual: ty,
                })
            }
        };

        Ok(self.alloc_node(
            NodeKind::ArrayCmpMask { array, scalar, op },
            Type::BitVector { width },
            Effects::empty(),
            span,
        ))
    }

    pub fn pack(&mut self, array: NodeId, span: Span) -> Result<NodeId, BuildError> {
        let ty = self.get_type(array)?;
        let width = match &ty {
            Type::Array { element, length } if **element == Type::Bool => *length,
            Type::Slice { element } if **element == Type::Bool => {
                // Slices need a dynamic width — use 0 as placeholder.
                // The verifier will accept Slice(Bool) → BitVector{width: 0}.
                0
            }
            _ => {
                return Err(BuildError::TypeMismatch {
                    node: array,
                    expected: Type::Array {
                        element: Box::new(Type::Bool),
                        length: 64,
                    },
                    actual: ty,
                });
            }
        };
        let bv_ty = Type::BitVector { width };
        Ok(self.alloc_node(NodeKind::Pack { array }, bv_ty, Effects::empty(), span))
    }

    /// Create a Convert node: zero-extend, sign-extend, or truncate.
    pub fn convert(
        &mut self,
        operand: NodeId,
        target_ty: Type,
        kind: sir_nodes::ConvertKind,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        let _ = self.get_node(operand)?;
        Ok(self.alloc_node(NodeKind::Convert { operand, target_ty: target_ty.clone(), kind }, target_ty, Effects::empty(), span))
    }

    // ── Comparisons ─────────────────────────────────────────

    fn comparison(
        &mut self,
        make_kind: fn(NodeId, NodeId) -> NodeKind,
        lhs: NodeId,
        rhs: NodeId,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        self.expect_same_type(lhs, rhs)?;
        let kind = make_kind(lhs, rhs);
        Ok(self.alloc_node(kind, Type::Bool, Effects::empty(), span))
    }

    pub fn eq(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.comparison(|l, r| NodeKind::Eq { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn ne(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.comparison(|l, r| NodeKind::Ne { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn lt(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.comparison(|l, r| NodeKind::Lt { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn le(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.comparison(|l, r| NodeKind::Le { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn gt(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.comparison(|l, r| NodeKind::Gt { lhs: l, rhs: r }, lhs, rhs, span)
    }

    pub fn ge(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.comparison(|l, r| NodeKind::Ge { lhs: l, rhs: r }, lhs, rhs, span)
    }

    // ── Boolean ─────────────────────────────────────────────

    pub fn bool_and(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_type(lhs, &Type::Bool)?;
        self.expect_type(rhs, &Type::Bool)?;
        Ok(self.alloc_node(
            NodeKind::BoolAnd { lhs, rhs },
            Type::Bool,
            Effects::empty(),
            span,
        ))
    }

    pub fn bool_or(&mut self, lhs: NodeId, rhs: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_type(lhs, &Type::Bool)?;
        self.expect_type(rhs, &Type::Bool)?;
        Ok(self.alloc_node(
            NodeKind::BoolOr { lhs, rhs },
            Type::Bool,
            Effects::empty(),
            span,
        ))
    }

    pub fn bool_not(&mut self, operand: NodeId, span: Span) -> Result<NodeId, BuildError> {
        self.expect_type(operand, &Type::Bool)?;
        Ok(self.alloc_node(
            NodeKind::BoolNot { operand },
            Type::Bool,
            Effects::empty(),
            span,
        ))
    }

    // ── Select ──────────────────────────────────────────────

    pub fn select(
        &mut self,
        cond: NodeId,
        true_val: NodeId,
        false_val: NodeId,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        self.expect_type(cond, &Type::Bool)?;
        let ty = self.expect_same_type(true_val, false_val)?;
        Ok(self.alloc_node(
            NodeKind::Select {
                cond,
                true_val,
                false_val,
            },
            ty,
            Effects::empty(),
            span,
        ))
    }

    // ── Memory ──────────────────────────────────────────────

    pub fn load(&mut self, ptr: NodeId, ty: Type, span: Span) -> Result<NodeId, BuildError> {
        let ptr_ty = self.get_type(ptr)?;
        if !ptr_ty.is_pointer_like() {
            return Err(BuildError::TypeMismatch {
                node: ptr,
                expected: Type::Pointer {
                    pointee: Box::new(Type::Unit),
                    mutable: true,
                },
                actual: ptr_ty,
            });
        }
        Ok(self.alloc_node(NodeKind::Load { ptr }, ty, Effects::READ_MEMORY, span))
    }

    pub fn store(&mut self, ptr: NodeId, value: NodeId, span: Span) -> Result<NodeId, BuildError> {
        let ptr_ty = self.get_type(ptr)?;
        let array_lvalue = self
            .function()
            .get_node(ptr)
            .is_some_and(|node| matches!(node.kind, NodeKind::ArrayAccess { .. }));
        if !(ptr_ty.is_pointer_like() || array_lvalue) {
            return Err(BuildError::TypeMismatch {
                node: ptr,
                expected: Type::Pointer {
                    pointee: Box::new(Type::Unit),
                    mutable: true,
                },
                actual: ptr_ty,
            });
        }
        if array_lvalue {
            let value_ty = self.get_type(value)?;
            if value_ty != ptr_ty {
                return Err(BuildError::TypeMismatch {
                    node: value,
                    expected: ptr_ty,
                    actual: value_ty,
                });
            }
        }
        Ok(self.alloc_node(
            NodeKind::Store { ptr, value },
            Type::Unit,
            Effects::WRITE_MEMORY,
            span,
        ))
    }
    pub fn field_access(
        &mut self,
        base: NodeId,
        field: impl Into<String>,
        field_ty: Type,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        // We check that base exists, but can't structurally verify the field
        // without knowing the struct layout at build time.
        let _ = self.get_node(base)?;
        Ok(self.alloc_node(
            NodeKind::FieldAccess {
                base,
                field: field.into(),
            },
            field_ty,
            Effects::empty(),
            span,
        ))
    }

    pub fn array_access(
        &mut self,
        base: NodeId,
        index: NodeId,
        element_ty: Type,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        self.expect_integer(index)?;
        let _ = self.get_node(base)?;
        Ok(self.alloc_node(
            NodeKind::ArrayAccess { base, index },
            element_ty,
            Effects::empty(),
            span,
        ))
    }

    /// Advance a pointer by `index` elements of `pointee`.
    pub fn pointer_offset(
        &mut self,
        base: NodeId,
        index: NodeId,
        pointee: Type,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        self.expect_integer(index)?;
        let base_ty = self.get_type(base)?;
        if !base_ty.is_pointer_like() {
            return Err(BuildError::TypeMismatch {
                node: base,
                expected: Type::Pointer {
                    pointee: Box::new(Type::Unit),
                    mutable: true,
                },
                actual: base_ty,
            });
        }
        let mutable = match base_ty {
            Type::Pointer { mutable, .. } => mutable,
            _ => true,
        };
        Ok(self.alloc_node(
            NodeKind::PointerOffset { base, index },
            Type::Pointer {
                pointee: Box::new(pointee),
                mutable,
            },
            Effects::empty(),
            span,
        ))
    }

    pub fn tuple_extract(
        &mut self,
        tuple: NodeId,
        index: usize,
        element_ty: Type,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        let _ = self.get_node(tuple)?;
        Ok(self.alloc_node(
            NodeKind::TupleExtract { tuple, index },
            element_ty,
            Effects::empty(),
            span,
        ))
    }

    // ── Calls ───────────────────────────────────────────────
    pub fn intrinsic(
        &mut self,
        name: impl Into<String>,
        args: &[NodeId],
        ty: Type,
        effects: Effects,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        Ok(self.alloc_node(
            NodeKind::Intrinsic {
                name: name.into(),
                args: args.to_vec(),
            },
            ty,
            effects,
            span,
        ))
    }
    // ── Loops ───────────────────────────────────────────────

    pub fn r#loop(
        &mut self,
        body: &[NodeId],
        termination: NodeId,
        outputs: &[NodeId],
        carried_inputs: &[NodeId],
        output_ty: Type,
        span: Span,
    ) -> Result<NodeId, BuildError> {
        self.expect_type(termination, &Type::Bool)?;

        let mut loop_effects = Effects::empty();

        // Verify all body/output/carried nodes exist and accumulate body effects
        for &id in body.iter().chain(outputs).chain(carried_inputs) {
            let node = self.get_node(id)?;
            if body.contains(&id) {
                loop_effects |= node.effects;
            }
        }

        Ok(self.alloc_node(
            NodeKind::Loop {
                body: body.to_vec(),
                termination,
                outputs: outputs.to_vec(),
                carried_inputs: carried_inputs.to_vec(),
            },
            output_ty,
            loop_effects,
            span,
        ))
    }
    // ── Control flow ────────────────────────────────────────

    pub fn return_value(&mut self, value: NodeId, span: Span) -> Result<NodeId, BuildError> {
        if self.func.return_node.is_some() {
            return Err(BuildError::DuplicateReturn);
        }
        let id = self.alloc_node(
            NodeKind::Return { value },
            Type::Unit,
            Effects::empty(),
            span,
        );
        self.func.return_node = Some(id);
        Ok(id)
    }
}
