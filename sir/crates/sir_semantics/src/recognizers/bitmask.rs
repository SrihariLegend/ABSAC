use std::collections::HashMap;

use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::{NodeId, Type};

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

    // Build reverse-use map: which nodes appear as lhs/rhs of And/Or/Xor?
    // This avoids O(n²) repeated arena scans for each integer-typed node.
    let bitwise_use_map = build_bitwise_use_map(func);

    for node in func.arena.iter() {
        if let Type::Integer { width, .. } = &node.ty {
            let bits = width.bits();
            // Only flag integers up to 128 bits; larger widths are not bitmasks.
            if bits <= 128 && *bitwise_use_map.get(&node.id).unwrap_or(&false) {
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

/// Build a map from NodeId to whether it appears as lhs/rhs of And/Or/Xor.
/// O(n) — single arena scan instead of O(n²) repeated scans.
fn build_bitwise_use_map(func: &Function) -> HashMap<NodeId, bool> {
    let mut map: HashMap<NodeId, bool> = HashMap::new();

    for node in func.arena.iter() {
        use sir_nodes::NodeKind;
        match &node.kind {
            NodeKind::And { lhs, rhs } | NodeKind::Or { lhs, rhs } | NodeKind::Xor { lhs, rhs } => {
                map.insert(*lhs, true);
                map.insert(*rhs, true);
            }
            _ => {}
        }
    }

    map
}
