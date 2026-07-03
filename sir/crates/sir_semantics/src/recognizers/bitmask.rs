use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::Type;

use sir_transform::constraints::Constraint;
use sir_transform::structures::SourceStructure;

use crate::region::RegionId;
use crate::structure::StructuralDescription;

/// Recognize bitmask patterns: integer types used as flag containers.
pub fn recognize_bitmask(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(RegionId, StructuralDescription)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let Type::Integer { width, .. } = &node.ty {
            let bits = width.bits();
            // Only flag integers up to 128 bits; larger widths are not bitmasks.
            if bits <= 128 && has_bitwise_operations(func, node.id) {
                let desc = StructuralDescription::new(
                    RegionId::new(0),
                    SourceStructure::BitMask { width: bits },
                )
                .with_constraint(Constraint::FixedLength(bits));

                results.push((RegionId::new(0), desc));
            }
        }
    }

    results
}

fn has_bitwise_operations(func: &Function, node_id: sir_types::NodeId) -> bool {
    for node in func.arena.iter() {
        use sir_nodes::NodeKind;
        match &node.kind {
            NodeKind::And { lhs, rhs }
            | NodeKind::Or { lhs, rhs }
            | NodeKind::Xor { lhs, rhs }
                if *lhs == node_id || *rhs == node_id =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}
