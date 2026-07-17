use std::collections::{BTreeSet, HashMap, HashSet};

use sir_nodes::{Function, Node, NodeKind};
use sir_types::{NodeId, Type};

use crate::VerificationError;

/// A verifier for SIR graph invariants.
///
/// The `Verifier` performs seven checks on a `Function`:
///
/// 1. **SSA** — Every `NodeId` in the arena is unique (defensive check).
/// 2. **References** — Every `NodeId` referenced as an input exists in the arena.
/// 3. **Cycles** — The dependency graph is a DAG, except for Loop body nodes
///    which may reference their termination condition.
/// 4. **Types** — Each node's inputs match expected types per the type rules.
/// 5. **Return** — Exactly one `Return` node exists, with matching return type.
/// 6. **Parameters** — `Parameter` indices are valid and one-to-one.
/// 7. **Loops** — Loop termination is `Bool`, body nodes exist, carried/output counts match.
///
/// # Usage
///
/// ```ignore
/// let mut v = Verifier::new(&func);
/// if v.verify() {
///     println!("valid");
/// } else {
///     for err in v.errors() {
///         eprintln!("{err}");
///     }
/// }
/// ```
pub struct Verifier<'a> {
    func: &'a Function,
    errors: Vec<VerificationError>,
}

impl<'a> Verifier<'a> {
    /// Create a new verifier for the given function.
    pub fn new(func: &'a Function) -> Self {
        Self {
            func,
            errors: Vec::new(),
        }
    }

    /// Run all verification checks.
    ///
    /// Returns `true` if no errors were found, `false` otherwise.
    /// Even if one check fails, subsequent checks still run to collect
    /// all errors.
    pub fn verify(&mut self) -> bool {
        self.check_ssa();
        self.check_references();
        self.check_cycles();
        self.check_types();
        self.check_return();
        self.check_parameters();
        self.check_loops();
        self.errors.is_empty()
    }

    /// Return the list of errors found.
    pub fn errors(&self) -> &[VerificationError] {
        &self.errors
    }

    // ── Check 1: SSA ───────────────────────────────────────

    /// Verify that every `NodeId` appears exactly once in the arena.
    /// This is a defensive check — `BTreeMap` already guarantees key uniqueness.
    fn check_ssa(&mut self) {
        // BTreeMap ensures key uniqueness by construction.
        // We iterate to verify no internal inconsistencies exist.
        let mut seen = HashSet::new();
        for node in &self.func.arena {
            if !seen.insert(node.id) {
                // This should be unreachable with BTreeMap, but check anyway.
                self.errors.push(VerificationError::InvalidInput {
                    node: node.id,
                    kind: node.kind.clone(),
                    message: "duplicate NodeId detected (SSA violation)".to_string(),
                });
            }
        }
    }

    // ── Check 2: References ────────────────────────────────

    /// Verify that every `NodeId` referenced as an input exists in the arena.
    fn check_references(&mut self) {
        for node in &self.func.arena {
            for input_id in node.kind.input_nodes() {
                if !self.func.arena.contains(input_id) {
                    self.errors.push(VerificationError::DanglingReference {
                        node: node.id,
                        referenced: input_id,
                    });
                }
            }
        }
    }

    // ── Check 3: Cycles ────────────────────────────────────

    /// Check for cycles in the dependency graph using three-color DFS.
    ///
    /// Loop bodies are allowed to reference their own termination condition,
    /// which creates a legitimate cycle. All other cycles are errors.
    fn check_cycles(&mut self) {
        // Collect nodes that are allowed to reference their loop's termination.
        let allowed_back_edges = self.collect_loop_termination_refs();

        // Three-color DFS: 0=unvisited, 1=in_progress, 2=finished
        let mut color: HashMap<NodeId, u8> = HashMap::new();

        for node in &self.func.arena {
            color.entry(node.id).or_insert(0);
        }

        for node in &self.func.arena {
            if color.get(&node.id) == Some(&0) {
                let mut stack = vec![(node.id, 0usize)]; // (node, next_child_index)
                color.insert(node.id, 1); // in_progress

                while let Some((current, child_idx)) = stack.last_mut() {
                    let inputs = self.get_inputs(*current);
                    if *child_idx < inputs.len() {
                        let child = inputs[*child_idx];
                        *child_idx += 1;

                        match color.get(&child).copied() {
                            Some(1) => {
                                // Back edge detected — check if allowed.
                                let allowed = allowed_back_edges.contains(&(*current, child));
                                if !allowed {
                                    self.errors.push(VerificationError::CycleDetected(*current));
                                }
                            }
                            Some(0) => {
                                color.insert(child, 1);
                                stack.push((child, 0));
                            }
                            Some(2) => {} // Cross edge, fine.
                            None => {}    // Shouldn't happen after reference check.
                            _ => {}
                        }
                    } else {
                        // All children processed.
                        color.insert(*current, 2);
                        stack.pop();
                    }
                }
            }
        }
    }

    /// Build a set of (node_id, termination_id) pairs that are allowed back-edges.
    /// Any node inside a Loop body may reference the Loop's termination.
    fn collect_loop_termination_refs(&self) -> HashSet<(NodeId, NodeId)> {
        let mut allowed = HashSet::new();

        for node in &self.func.arena {
            if let NodeKind::Loop {
                body,
                termination,
                outputs,
                carried_inputs,
            } = &node.kind
            {
                // All nodes in body, outputs, and carried_inputs may reference the termination.
                let all_nodes: BTreeSet<NodeId> = body
                    .iter()
                    .chain(outputs)
                    .chain(carried_inputs)
                    .copied()
                    .collect();
                for &n in &all_nodes {
                    allowed.insert((n, *termination));
                }
            }
        }

        allowed
    }

    /// Get the input NodeIds for a given node (for cycle checking).
    fn get_inputs(&self, id: NodeId) -> Vec<NodeId> {
        self.func
            .arena
            .get(id)
            .map(|n| n.kind.input_nodes())
            .unwrap_or_default()
    }

    // ── Check 4: Types ──────────────────────────────────────

    /// Type-check every node's inputs.
    fn check_types(&mut self) {
        for node in &self.func.arena {
            match &node.kind {
                NodeKind::Constant(_) | NodeKind::Parameter { .. } => {
                    // No inputs to check.
                }

                // Arithmetic binary: both operands must be the same numeric type.
                NodeKind::Add { lhs, rhs }
                | NodeKind::Sub { lhs, rhs }
                | NodeKind::Mul { lhs, rhs }
                | NodeKind::Div { lhs, rhs }
                | NodeKind::Rem { lhs, rhs } => {
                    let lt = self.node_type(*lhs);
                    let rt = self.node_type(*rhs);
                    if let (Some(lt), Some(rt)) = (&lt, &rt) {
                        if lt != rt {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 1,
                                expected: lt.clone(),
                                actual: rt.clone(),
                            });
                        }
                        if !lt.is_numeric() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::i32(), // placeholder
                                actual: lt.clone(),
                            });
                        }
                    }
                }

                // Arithmetic unary: operand must be numeric.
                NodeKind::Neg { operand } => {
                    if let Some(ty) = self.node_type(*operand) {
                        if !ty.is_numeric() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::i32(),
                                actual: ty,
                            });
                        }
                    }
                }

                // Bitwise binary: both must be integer, same type.
                NodeKind::And { lhs, rhs }
                | NodeKind::Or { lhs, rhs }
                | NodeKind::Xor { lhs, rhs } => {
                    if let (Some(lt), Some(rt)) = (self.node_type(*lhs), self.node_type(*rhs)) {
                        if !lt.is_integer_or_bitvector() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::i32(),
                                actual: lt.clone(),
                            });
                        }
                        if lt != rt {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 1,
                                expected: lt,
                                actual: rt,
                            });
                        }
                    }
                }

                // Shifts: lhs must be integer, rhs must be integer (any width).
                NodeKind::Shl { lhs, rhs }
                | NodeKind::Shr { lhs, rhs }
                | NodeKind::Rol { lhs, rhs }
                | NodeKind::Ror { lhs, rhs } => {
                    if let Some(ty) = self.node_type(*lhs) {
                        if !ty.is_integer_or_bitvector() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::i32(),
                                actual: ty,
                            });
                        }
                    }
                    if let Some(ty) = self.node_type(*rhs) {
                        if !ty.is_integer_or_bitvector() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 1,
                                expected: Type::i32(),
                                actual: ty,
                            });
                        }
                    }
                }

                // Bitwise unary: operand must be integer.
                NodeKind::Not { operand }
                | NodeKind::Popcount { operand }
                | NodeKind::LeadingZeros { operand }
                | NodeKind::TrailingZeros { operand } => {
                    if let Some(ty) = self.node_type(*operand) {
                        if !ty.is_integer_or_bitvector() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::i32(),
                                actual: ty,
                            });
                        }
                    }
                }

                // Pack: operand must be Array(Bool) or Slice(Bool);
                // output type must be BitVector with width matching the array length.
                NodeKind::Pack { array } => {
                    if let Some(ty) = self.node_type(*array) {
                        match &ty {
                            Type::Array { element, length } if **element == Type::Bool => {
                                // Verify output type is BitVector with matching width.
                                match &node.ty {
                                    Type::BitVector { width } if *width == *length => {}
                                    Type::BitVector { .. } => {
                                        self.errors.push(VerificationError::TypeMismatch {
                                            node: node.id,
                                            kind: node.kind.clone(),
                                            input_index: 0,
                                            expected: Type::BitVector { width: *length },
                                            actual: node.ty.clone(),
                                        });
                                    }
                                    other => {
                                        self.errors.push(VerificationError::TypeMismatch {
                                            node: node.id,
                                            kind: node.kind.clone(),
                                            input_index: 0,
                                            expected: Type::BitVector { width: *length },
                                            actual: other.clone(),
                                        });
                                    }
                                }
                            }
                            Type::Array { element, .. } | Type::Slice { element } => {
                                if **element != Type::Bool {
                                    self.errors.push(VerificationError::TypeMismatch {
                                        node: node.id,
                                        kind: node.kind.clone(),
                                        input_index: 0,
                                        expected: Type::Bool,
                                        actual: *element.clone(),
                                    });
                                }
                            }
                            _ => {
                                self.errors.push(VerificationError::TypeMismatch {
                                    node: node.id,
                                    kind: node.kind.clone(),
                                    input_index: 0,
                                    expected: Type::Array {
                                        element: Box::new(Type::Bool),
                                        length: 64,
                                    },
                                    actual: ty,
                                });
                            }
                        }
                    }
                }

                NodeKind::ArrayCmpMask {
                    array,
                    scalar,
                    op: _,
                } => {
                    // Type checks are primarily handled by the builder for now.
                    let _ = self.node_type(*array);
                    let _ = self.node_type(*scalar);
                }

                // Comparisons: operands must be same type. Result is Bool (checked by node.ty).
                NodeKind::Eq { lhs, rhs }
                | NodeKind::Ne { lhs, rhs }
                | NodeKind::Lt { lhs, rhs }
                | NodeKind::Le { lhs, rhs }
                | NodeKind::Gt { lhs, rhs }
                | NodeKind::Ge { lhs, rhs } => {
                    if let (Some(lt), Some(rt)) = (self.node_type(*lhs), self.node_type(*rhs)) {
                        if lt != rt {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 1,
                                expected: lt,
                                actual: rt,
                            });
                        }
                    }
                }

                // Boolean: operands must be Bool.
                NodeKind::BoolAnd { lhs, rhs } | NodeKind::BoolOr { lhs, rhs } => {
                    if let Some(ty) = self.node_type(*lhs) {
                        if !ty.is_bool() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::Bool,
                                actual: ty,
                            });
                        }
                    }
                    if let Some(ty) = self.node_type(*rhs) {
                        if !ty.is_bool() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 1,
                                expected: Type::Bool,
                                actual: ty,
                            });
                        }
                    }
                }

                NodeKind::BoolNot { operand } => {
                    if let Some(ty) = self.node_type(*operand) {
                        if !ty.is_bool() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::Bool,
                                actual: ty,
                            });
                        }
                    }
                }

                // Select: cond must be Bool, true/false must be same type.
                NodeKind::Select {
                    cond,
                    true_val,
                    false_val,
                } => {
                    if let Some(ty) = self.node_type(*cond) {
                        if !ty.is_bool() {
                            self.errors.push(VerificationError::SelectConditionNotBool {
                                node: node.id,
                                actual: ty,
                            });
                        }
                    }
                    if let (Some(tt), Some(ft)) =
                        (self.node_type(*true_val), self.node_type(*false_val))
                    {
                        if tt != ft {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 2,
                                expected: tt,
                                actual: ft,
                            });
                        }
                    }
                }

                // Load: ptr must be pointer-like.
                NodeKind::Load { ptr } => {
                    if let Some(ty) = self.node_type(*ptr) {
                        if !ty.is_pointer_like() {
                            self.errors
                                .push(VerificationError::InvalidPointerOperation {
                                    node: node.id,
                                    actual: ty,
                                });
                        }
                    }
                }

                // Store: ptr must be pointer-like.
                NodeKind::Store { ptr, .. } => {
                    if let Some(ty) = self.node_type(*ptr) {
                        if !ty.is_pointer_like() {
                            self.errors
                                .push(VerificationError::InvalidPointerOperation {
                                    node: node.id,
                                    actual: ty,
                                });
                        }
                    }
                }

                // Allocate: count must be integer.
                NodeKind::Allocate { count, .. } => {
                    if let Some(ty) = self.node_type(*count) {
                        if !ty.is_integer_or_bitvector() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::i32(),
                                actual: ty,
                            });
                        }
                    }
                }

                // Deallocate: ptr must be pointer-like.
                NodeKind::Deallocate { ptr } => {
                    if let Some(ty) = self.node_type(*ptr) {
                        if !ty.is_pointer_like() {
                            self.errors
                                .push(VerificationError::InvalidPointerOperation {
                                    node: node.id,
                                    actual: ty,
                                });
                        }
                    }
                }

                // FieldAccess: base should be a struct (structural check deferred).
                NodeKind::FieldAccess { base, .. } => {
                    let _ = base; // base existence checked by reference check
                }

                // ArrayAccess: index must be integer.
                NodeKind::ArrayAccess { base, index } => {
                    if let Some(ty) = self.node_type(*index) {
                        if !ty.is_integer_or_bitvector() {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 1,
                                expected: Type::i32(),
                                actual: ty,
                            });
                        }
                    }
                    let _ = base;
                }
                
                NodeKind::TupleExtract { tuple, index } => {
                    if let Some(ty) = self.node_type(*tuple) {
                        if !matches!(ty, Type::Tuple { .. }) {
                            self.errors.push(VerificationError::TypeMismatch {
                                node: node.id,
                                kind: node.kind.clone(),
                                input_index: 0,
                                expected: Type::Tuple { elements: vec![] },
                                actual: ty,
                            });
                        }
                    }
                    let _ = index;
                }

                // Calls: args existence checked by reference check.
                NodeKind::Call { .. }
                | NodeKind::Intrinsic { .. }
                | NodeKind::ExternalCall { .. } => {
                    // No structural type checks for calls in v0.1.
                }

                // Loop: checked separately in check_loops.
                NodeKind::Loop { .. } => {}

                // Iterator: collection existence checked by reference check.
                NodeKind::Iterator { .. } => {}

                // Return: checked in check_return.
                NodeKind::Return { .. } => {}
            }
        }
    }

    /// Get the type of a node, if it exists in the arena.
    fn node_type(&self, id: NodeId) -> Option<Type> {
        self.func.arena.get(id).map(|n| n.ty.clone())
    }

    // ── Check 5: Return ─────────────────────────────────────

    /// Verify exactly one Return node exists and its value type matches return_ty.
    fn check_return(&mut self) {
        let return_nodes: Vec<&Node> = self
            .func
            .arena
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Return { .. }))
            .collect();

        if return_nodes.is_empty() {
            self.errors.push(VerificationError::MissingReturn);
        } else if return_nodes.len() > 1 {
            self.errors.push(VerificationError::DuplicateReturn);
        } else {
            // Check the return value type.
            let ret = &return_nodes[0];
            if let NodeKind::Return { value } = &ret.kind {
                if let Some(val_ty) = self.node_type(*value) {
                    if val_ty != self.func.return_ty {
                        self.errors.push(VerificationError::ReturnTypeMismatch {
                            expected: self.func.return_ty.clone(),
                            actual: val_ty,
                        });
                    }
                }
            }
        }
    }

    // ── Check 6: Parameters ─────────────────────────────────

    /// Verify Parameter nodes have valid indices and are one-to-one with function params.
    fn check_parameters(&mut self) {
        let param_count = self.func.params.len();
        let mut seen_indices = HashSet::new();

        for node in &self.func.arena {
            if let NodeKind::Parameter { index } = &node.kind {
                if *index >= param_count {
                    self.errors.push(VerificationError::ParameterIndexMismatch {
                        param_index: *index,
                        expected_count: param_count,
                    });
                }
                if !seen_indices.insert(*index) {
                    self.errors.push(VerificationError::InvalidInput {
                        node: node.id,
                        kind: node.kind.clone(),
                        message: format!("duplicate parameter index {index}"),
                    });
                }
            }
        }

        // Check that every parameter slot has a node (optional: could be lenient).
        if seen_indices.len() != param_count && !seen_indices.is_empty() {
            // Some params are missing nodes — this is a minor issue, report it.
            for i in 0..param_count {
                if !seen_indices.contains(&i) {
                    self.errors.push(VerificationError::InvalidInput {
                        node: NodeId::new(i as u64),
                        kind: NodeKind::Parameter { index: i },
                        message: format!("parameter {i} declared but has no node in arena"),
                    });
                }
            }
        }
    }

    // ── Check 7: Loops ─────────────────────────────────────

    /// Verify loop invariants:
    /// - termination is Bool
    /// - body, output, and carried nodes exist
    /// - carried_inputs.len() == outputs.len()
    fn check_loops(&mut self) {
        for node in &self.func.arena {
            if let NodeKind::Loop {
                body,
                termination,
                outputs,
                carried_inputs,
            } = &node.kind
            {
                // Termination must be Bool.
                if let Some(term_ty) = self.node_type(*termination) {
                    if !term_ty.is_bool() {
                        self.errors.push(VerificationError::LoopTerminationNotBool {
                            loop_node: node.id,
                            actual: term_ty,
                        });
                    }
                }

                // Body nodes must exist (also caught by reference check).
                for &body_id in body {
                    if !self.func.arena.contains(body_id) {
                        self.errors.push(VerificationError::NodeNotFound(body_id));
                    }
                }

                // Output nodes must exist.
                for &out_id in outputs {
                    if !self.func.arena.contains(out_id) {
                        self.errors.push(VerificationError::NodeNotFound(out_id));
                    }
                }

                // Carried inputs must exist.
                for &carry_id in carried_inputs {
                    if !self.func.arena.contains(carry_id) {
                        self.errors.push(VerificationError::NodeNotFound(carry_id));
                    }
                }

                // carried_inputs.len() == outputs.len()
                if carried_inputs.len() != outputs.len() {
                    self.errors.push(VerificationError::LoopCarriedMismatch {
                        node: node.id,
                        carried_count: carried_inputs.len(),
                        output_count: outputs.len(),
                    });
                }
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_nodes::NodeKind as NK;
    use sir_types::Effects;

    fn i32_type() -> Type {
        Type::i32()
    }

    fn build_valid_simple() -> Function {
        let mut b = Builder::new(
            "simple",
            &[("x", i32_type()), ("y", i32_type())],
            i32_type(),
        );
        let x = b.parameter_index(0).unwrap();
        let y = b.parameter_index(1).unwrap();
        let sum = b.add(x, y, sir_types::Span::unknown()).unwrap();
        b.return_value(sum, sir_types::Span::unknown()).unwrap();
        b.build()
    }

    #[test]
    fn valid_function_passes() {
        let func = build_valid_simple();
        let mut v = Verifier::new(&func);
        assert!(v.verify());
        assert!(v.errors().is_empty());
    }

    #[test]
    fn missing_return_fails() {
        let func = Function::new("no_ret", i32_type());
        let mut v = Verifier::new(&func);
        assert!(!v.verify());
        assert!(v
            .errors()
            .iter()
            .any(|e| matches!(e, VerificationError::MissingReturn)));
    }

    #[test]
    fn return_type_mismatch_fails() {
        let mut b = Builder::new("bad_ret", &[("x", i32_type())], Type::Bool);
        let x = b.parameter_index(0).unwrap();
        b.return_value(x, sir_types::Span::unknown()).unwrap();
        let func = b.build();
        let mut v = Verifier::new(&func);
        assert!(!v.verify());
        assert!(v
            .errors()
            .iter()
            .any(|e| matches!(e, VerificationError::ReturnTypeMismatch { .. })));
    }

    #[test]
    fn cycle_outside_loop_fails() {
        // Create a cycle manually: node A references node B, B references A.
        let mut func = Function::new("cyclic", Type::i32());
        let id_a = NodeId::new(10);
        let id_b = NodeId::new(11);

        let node_a = sir_nodes::Node::new(
            id_a,
            NK::Add {
                lhs: id_b,
                rhs: id_b,
            },
            i32_type(),
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        let node_b = sir_nodes::Node::new(
            id_b,
            NK::Sub {
                lhs: id_a,
                rhs: id_a,
            },
            i32_type(),
            Effects::empty(),
            sir_types::Span::unknown(),
        );

        func.insert_node(node_a);
        func.insert_node(node_b);
        // Add a parameter to satisfy parameter check.
        let p = func.add_param("x", i32_type(), sir_types::Span::unknown());
        let ret = sir_nodes::Node::new(
            NodeId::new(12),
            NK::Return { value: p },
            Type::Unit,
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(ret);
        func.return_node = Some(NodeId::new(12));

        let mut v = Verifier::new(&func);
        assert!(!v.verify());
        assert!(v
            .errors()
            .iter()
            .any(|e| matches!(e, VerificationError::CycleDetected(_))));
    }

    #[test]
    fn dangling_reference_fails() {
        let mut func = Function::new("dangling", i32_type());
        let p = func.add_param("x", i32_type(), sir_types::Span::unknown());
        // Create a node referencing a non-existent node.
        let node = sir_nodes::Node::new(
            NodeId::new(10),
            NK::Add {
                lhs: p,
                rhs: NodeId::new(999), // nonexistent
            },
            i32_type(),
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(node);
        let ret = sir_nodes::Node::new(
            NodeId::new(11),
            NK::Return { value: p },
            Type::Unit,
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(ret);
        func.return_node = Some(NodeId::new(11));

        let mut v = Verifier::new(&func);
        assert!(!v.verify());
        assert!(v
            .errors()
            .iter()
            .any(|e| matches!(e, VerificationError::DanglingReference { .. })));
    }

    #[test]
    fn parameter_index_out_of_range_fails() {
        let mut func = Function::new("bad_param", i32_type());
        // Create a Parameter node with index beyond the param list.
        let node = sir_nodes::Node::new(
            NodeId::new(0),
            NK::Parameter { index: 5 },
            i32_type(),
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(node);
        let ret = sir_nodes::Node::new(
            NodeId::new(1),
            NK::Return {
                value: NodeId::new(0),
            },
            Type::Unit,
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(ret);
        func.return_node = Some(NodeId::new(1));

        let mut v = Verifier::new(&func);
        assert!(!v.verify());
        assert!(v
            .errors()
            .iter()
            .any(|e| matches!(e, VerificationError::ParameterIndexMismatch { .. })));
    }

    #[test]
    fn select_condition_must_be_bool() {
        let mut func = Function::new("bad_select", i32_type());
        let p = func.add_param("x", i32_type(), sir_types::Span::unknown());
        // Select with non-bool condition.
        let sel = sir_nodes::Node::new(
            NodeId::new(10),
            NK::Select {
                cond: p, // i32, NOT bool
                true_val: p,
                false_val: p,
            },
            i32_type(),
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(sel);
        let ret = sir_nodes::Node::new(
            NodeId::new(11),
            NK::Return {
                value: NodeId::new(10),
            },
            Type::Unit,
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(ret);
        func.return_node = Some(NodeId::new(11));

        let mut v = Verifier::new(&func);
        assert!(!v.verify());
        assert!(v
            .errors()
            .iter()
            .any(|e| matches!(e, VerificationError::SelectConditionNotBool { .. })));
    }

    #[test]
    fn loop_with_non_bool_termination_fails() {
        let mut func = Function::new("bad_loop", i32_type());
        let start = func.add_param("start", i32_type(), sir_types::Span::unknown());
        // Loop with termination = i32 (should be Bool).
        let loop_node = sir_nodes::Node::new(
            NodeId::new(20),
            NK::Loop {
                body: vec![],
                termination: start, // i32, not Bool
                outputs: vec![],
                carried_inputs: vec![],
            },
            i32_type(),
            Effects::READ_MEMORY | Effects::WRITE_MEMORY,
            sir_types::Span::unknown(),
        );
        func.insert_node(loop_node);
        let ret = sir_nodes::Node::new(
            NodeId::new(21),
            NK::Return {
                value: NodeId::new(20),
            },
            Type::Unit,
            Effects::empty(),
            sir_types::Span::unknown(),
        );
        func.insert_node(ret);
        func.return_node = Some(NodeId::new(21));

        let mut v = Verifier::new(&func);
        assert!(!v.verify());
        assert!(v
            .errors()
            .iter()
            .any(|e| matches!(e, VerificationError::LoopTerminationNotBool { .. })));
    }
}
