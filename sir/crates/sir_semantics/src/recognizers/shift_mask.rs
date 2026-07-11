use sir_analysis::facts::FactDatabase;
use sir_nodes::Function;
use sir_types::NodeId;

use crate::concepts::SemanticConcept;
use crate::region::RecognitionExplanation;
use sir_transform::structures::SourceStructure;

pub fn recognize_shift_mask(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(SemanticConcept, RecognitionExplanation, Vec<NodeId>)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Shr {
            lhs,
            rhs: shift_right_amt,
        } = &node.kind
        {
            if let Some(lhs_node) = func.get_node(*lhs) {
                if let sir_nodes::NodeKind::Shl {
                    lhs: inner_lhs,
                    rhs: shift_left_amt,
                } = &lhs_node.kind
                {
                    // Check if both shifts are by the same constant amount
                    if let (Some(r_amt_node), Some(l_amt_node)) = (
                        func.get_node(*shift_right_amt),
                        func.get_node(*shift_left_amt),
                    ) {
                        if let (
                            sir_nodes::NodeKind::Constant(r_c),
                            sir_nodes::NodeKind::Constant(l_c),
                        ) = (&r_amt_node.kind, &l_amt_node.kind)
                        {
                            if r_c == l_c {
                                results.push((
                                    SemanticConcept::ShiftMask,
                                    RecognitionExplanation {
                                        concept: SemanticConcept::ShiftMask,
                                        triggering_facts: vec![
                                            "Shift right follows shift left",
                                            "Shift amounts are identical constants",
                                        ],
                                    },
                                    vec![
                                        node.id,
                                        *lhs,
                                        *inner_lhs,
                                        *shift_right_amt,
                                        *shift_left_amt,
                                    ],
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

pub fn recognize_shift_mask_operator(
    func: &Function,
    _analysis: &FactDatabase,
) -> Vec<(
    crate::region::RegionId,
    crate::structure::StructuralDescription,
)> {
    let mut results = Vec::new();

    for node in func.arena.iter() {
        if let sir_nodes::NodeKind::Shr {
            lhs,
            rhs: shift_right_amt,
        } = &node.kind
        {
            if let Some(lhs_node) = func.get_node(*lhs) {
                if let sir_nodes::NodeKind::Shl {
                    rhs: shift_left_amt,
                    ..
                } = &lhs_node.kind
                {
                    if let (Some(r_amt_node), Some(l_amt_node)) = (
                        func.get_node(*shift_right_amt),
                        func.get_node(*shift_left_amt),
                    ) {
                        if let (
                            sir_nodes::NodeKind::Constant(r_c),
                            sir_nodes::NodeKind::Constant(l_c),
                        ) = (&r_amt_node.kind, &l_amt_node.kind)
                        {
                            if r_c == l_c {
                                let desc = crate::structure::StructuralDescription::new(
                                    crate::region::RegionId::new(0),
                                    SourceStructure::ShiftMaskOperator,
                                );
                                results.push((crate::region::RegionId::new(0), desc));
                            }
                        }
                    }
                }
            }
        }
    }

    results
}
