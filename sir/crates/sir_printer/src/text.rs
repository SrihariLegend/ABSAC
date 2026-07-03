use std::fmt::Write;

use sir_nodes::{Function, Module, Node, NodeKind};
use sir_types::NodeId;

/// A human-readable text printer for SIR graphs.
///
/// Supports two output modes:
/// - **Compact** — minimal format matching the spec example
/// - **Detailed** — shows IDs, types, spans, and effects
pub struct TextPrinter {
    compact: bool,
}

impl TextPrinter {
    /// Create a new text printer.
    ///
    /// When `compact` is true, uses the minimal format. Set to `false`
    /// for detailed output including types, IDs, and metadata.
    pub fn new(compact: bool) -> Self {
        Self { compact }
    }

    /// Print a Function to a String.
    pub fn function_to_string(&self, func: &Function) -> String {
        let mut buf = String::new();
        self.write_function(func, &mut buf).unwrap();
        buf
    }

    /// Print a Function to a writer.
    pub fn write_function(
        &self,
        func: &Function,
        w: &mut impl Write,
    ) -> std::fmt::Result {
        if self.compact {
            self.write_function_compact(func, w)
        } else {
            self.write_function_detailed(func, w)
        }
    }

    /// Print a single Node to a String.
    pub fn node_to_string(&self, node: &Node) -> String {
        let mut buf = String::new();
        if self.compact {
            write!(buf, "{}", node.kind.kind_name()).unwrap();
            match &node.kind {
                NodeKind::Parameter { index } => {
                    write!(buf, " {index}").unwrap();
                }
                NodeKind::Add { lhs, rhs }
                | NodeKind::Sub { lhs, rhs }
                | NodeKind::Mul { lhs, rhs }
                | NodeKind::Div { lhs, rhs }
                | NodeKind::Rem { lhs, rhs }
                | NodeKind::And { lhs, rhs }
                | NodeKind::Or { lhs, rhs }
                | NodeKind::Xor { lhs, rhs }
                | NodeKind::Shl { lhs, rhs }
                | NodeKind::Shr { lhs, rhs }
                | NodeKind::Eq { lhs, rhs }
                | NodeKind::Ne { lhs, rhs }
                | NodeKind::Lt { lhs, rhs }
                | NodeKind::Le { lhs, rhs }
                | NodeKind::Gt { lhs, rhs }
                | NodeKind::Ge { lhs, rhs } => {
                    write!(buf, " {lhs}, {rhs}").unwrap();
                }
                NodeKind::Return { value } => {
                    write!(buf, " {value}").unwrap();
                }
                NodeKind::Constant(data) => {
                    write!(buf, " {data}").unwrap();
                }
                NodeKind::Select {
                    cond,
                    true_val,
                    false_val,
                } => {
                    write!(buf, " {cond} ? {true_val} : {false_val}").unwrap();
                }
                _ => {}
            }
        } else {
            write!(buf, "{}: {} = {}", node.id, node.ty, node.kind).unwrap();
            if !node.effects.is_pure() {
                write!(buf, " [effects: {}]", node.effects).unwrap();
            }
            if !node.span.is_empty() {
                write!(buf, " @ {}", node.span).unwrap();
            }
        }
        buf
    }

    /// Print a Module to a String.
    pub fn module_to_string(&self, module: &Module) -> String {
        let mut buf = String::new();
        writeln!(buf, "Module {}", module.name).unwrap();
        for func in &module.functions {
            writeln!(buf).unwrap();
            self.write_function(func, &mut buf).unwrap();
        }
        buf
    }

    // ── Private helpers ─────────────────────────────────────

    fn write_function_compact(
        &self,
        func: &Function,
        w: &mut impl Write,
    ) -> std::fmt::Result {
        writeln!(w, "Function {}", func.name)?;
        for param in &func.params {
            writeln!(w, "Parameter {} ({})", param.name, param.ty)?;
        }
        // Print nodes in topological order: parameters first, then dataflow, then return
        let order = topological_order(func);
        for &id in &order {
            let node = func.get_node(id).unwrap();
            match &node.kind {
                NodeKind::Parameter { .. } => {} // already printed above
                NodeKind::Return { value } => {
                    writeln!(w, "Return {value}")?;
                }
                _ => {
                    write!(w, "{}", node.kind.kind_name())?;
                    self.write_node_compact_args(w, node)?;
                    // Show result type and ID
                    write!(w, " -> {} ({})", node.id, node.ty)?;
                    writeln!(w)?;
                }
            }
        }
        Ok(())
    }

    fn write_node_compact_args(
        &self,
        w: &mut impl Write,
        node: &Node,
    ) -> std::fmt::Result {
        match &node.kind {
            NodeKind::Constant(data) => write!(w, " {data}"),
            NodeKind::Add { lhs, rhs }
            | NodeKind::Sub { lhs, rhs }
            | NodeKind::Mul { lhs, rhs }
            | NodeKind::Div { lhs, rhs }
            | NodeKind::Rem { lhs, rhs }
            | NodeKind::And { lhs, rhs }
            | NodeKind::Or { lhs, rhs }
            | NodeKind::Xor { lhs, rhs }
            | NodeKind::Shl { lhs, rhs }
            | NodeKind::Shr { lhs, rhs }
            | NodeKind::Rol { lhs, rhs }
            | NodeKind::Ror { lhs, rhs }
            | NodeKind::Eq { lhs, rhs }
            | NodeKind::Ne { lhs, rhs }
            | NodeKind::Lt { lhs, rhs }
            | NodeKind::Le { lhs, rhs }
            | NodeKind::Gt { lhs, rhs }
            | NodeKind::Ge { lhs, rhs }
            | NodeKind::BoolAnd { lhs, rhs }
            | NodeKind::BoolOr { lhs, rhs } => write!(w, " {lhs}, {rhs}"),
            NodeKind::Neg { operand }
            | NodeKind::Not { operand }
            | NodeKind::Popcount { operand }
            | NodeKind::LeadingZeros { operand }
            | NodeKind::TrailingZeros { operand }
            | NodeKind::BoolNot { operand } => write!(w, " {operand}"),
            NodeKind::Select {
                cond,
                true_val,
                false_val,
            } => write!(w, " {cond} ? {true_val} : {false_val}"),
            NodeKind::Load { ptr } => write!(w, " {ptr}"),
            NodeKind::Store { ptr, value } => write!(w, " {ptr}, {value}"),
            NodeKind::Allocate { ty, count } => write!(w, " {ty}, {count}"),
            NodeKind::FieldAccess { base, field } => write!(w, " {base}.{field}"),
            NodeKind::ArrayAccess { base, index } => write!(w, " {base}[{index}]"),
            NodeKind::Call { callee, args } => {
                write!(w, " {callee}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(w, ", ")?;
                    }
                    write!(w, "{arg}")?;
                }
                write!(w, ")")
            }
            _ => write!(w, " ..."),
        }
    }

    fn write_function_detailed(
        &self,
        func: &Function,
        w: &mut impl Write,
    ) -> std::fmt::Result {
        writeln!(
            w,
            "Function {} (params: [{}], returns: {})",
            func.name,
            func.params
                .iter()
                .map(|p| format!("{}: {}", p.name, p.ty))
                .collect::<Vec<_>>()
                .join(", "),
            func.return_ty
        )?;

        for node in &func.arena {
            let effects_str = if node.effects.is_pure() {
                String::new()
            } else {
                format!(" (effects: {})", node.effects)
            };
            let span_str = if node.span.is_empty() {
                String::new()
            } else {
                format!(" @ {}", node.span)
            };
            writeln!(
                w,
                "  {}: {} = {}{}{}",
                node.id, node.ty, node.kind, effects_str, span_str
            )?;
        }
        Ok(())
    }
}

/// Compute a topological order of nodes for printing.
/// Parameters come first, then dataflow order, return comes last.
fn topological_order(func: &Function) -> Vec<NodeId> {
    let mut order = Vec::new();
    let all_ids: std::collections::BTreeSet<NodeId> =
        func.arena.nodes().keys().copied().collect();

    // Start with parameters (they have no inputs).
    for &id in &all_ids {
        if let Some(node) = func.get_node(id) {
            if matches!(node.kind, NodeKind::Parameter { .. }) {
                order.push(id);
            }
        }
    }

    // Simple DFS-based ordering for the rest.
    let mut visited: std::collections::BTreeSet<NodeId> = order.iter().copied().collect();
    let mut return_id = None;

    for &id in &all_ids {
        if !visited.contains(&id) {
            let mut stack = vec![id];
            let mut path: Vec<NodeId> = Vec::new();
            while let Some(current) = stack.pop() {
                if visited.contains(&current) {
                    continue;
                }

                // Check if this is a return node (save for last).
                if let Some(node) = func.get_node(current) {
                    if matches!(node.kind, NodeKind::Return { .. }) {
                        return_id = Some(current);
                        visited.insert(current);
                        continue;
                    }
                }

                visited.insert(current);
                path.push(current);

                // Push inputs to process first
                if let Some(node) = func.get_node(current) {
                    for input in node.kind.input_nodes() {
                        if !visited.contains(&input) {
                            stack.push(input);
                        }
                    }
                }
            }
            order.extend(path.into_iter().rev());
        }
    }

    // Append the return node last.
    if let Some(rid) = return_id {
        order.push(rid);
    }

    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_builder::Builder;
    use sir_types::Type;

    #[test]
    fn compact_print_add_function() {
        let mut b = Builder::new("add", &[("a", Type::i32()), ("b", Type::i32())], Type::i32());
        let a = b.parameter_index(0).unwrap();
        let b_ = b.parameter_index(1).unwrap();
        let sum = b.add(a, b_, sir_types::Span::unknown()).unwrap();
        b.return_value(sum, sir_types::Span::unknown()).unwrap();
        let func = b.build();

        let printer = TextPrinter::new(true);
        let output = printer.function_to_string(&func);
        assert!(output.contains("Function add"));
        assert!(output.contains("Parameter a"));
        assert!(output.contains("Parameter b"));
        assert!(output.contains("Add"));
        assert!(output.contains("Return"));
    }

    #[test]
    fn detailed_print_add_function() {
        let mut b = Builder::new("add", &[("a", Type::i32()), ("b", Type::i32())], Type::i32());
        let a = b.parameter_index(0).unwrap();
        let b_ = b.parameter_index(1).unwrap();
        let sum = b.add(a, b_, sir_types::Span::unknown()).unwrap();
        b.return_value(sum, sir_types::Span::unknown()).unwrap();
        let func = b.build();

        let printer = TextPrinter::new(false);
        let output = printer.function_to_string(&func);
        assert!(output.contains("Function add (params: [a:"));
        assert!(output.contains("Add"));
        assert!(output.contains("Return"));
        // Detailed mode includes node IDs
        assert!(output.contains("%"));
    }
}
