use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::Type;

use sir_transform::constraints::Constraint;
use sir_transform::structures::SourceStructure;

use crate::region::RegionId;
use crate::structure::StructuralDescription;

/// Recognize boolean array patterns: Array<bool> with known length.
pub fn recognize_boolean_array(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(RegionId, StructuralDescription)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let Type::Array { element, length } = &node.ty {
            if matches!(element.as_ref(), &Type::Bool) {
                let desc = StructuralDescription::new(
                    RegionId::new(0), // merged later
                    SourceStructure::BooleanArray { length: *length },
                )
                .with_constraint(Constraint::FixedLength(*length));

                results.push((RegionId::new(0), desc));
            }
        }
        if let sir_nodes::NodeKind::ArrayCmpMask { .. } = &node.kind {
            if let Type::BitVector { width } = &node.ty {
                let desc = StructuralDescription::new(
                    RegionId::new(0),
                    SourceStructure::DynamicBooleanSequence { length: *width },
                )
                .with_constraint(Constraint::FixedLength(*width));

                results.push((RegionId::new(0), desc));
            }
        }
    }

    results
}
