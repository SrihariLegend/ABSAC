use sir_semantics::structure::StructuralDescription;
use sir_transform::roles::RegionRoles;
use sir_types::NodeId;

use crate::error::RewriteError;

/// A transient execution object assembled by `RewriteEngine` at rewrite time.
///
/// Wraps the `StructuralDescription` (which carries `RegionRoles` assigned by
/// semantic recognition) and adds the set of nodes outside the region that
/// consume region-produced values.
///
/// Not persisted — assembled fresh for each rewrite.
#[derive(Clone, Debug)]
pub struct RewriteRegion {
    /// The structural description from semantic recognition.
    pub structural: StructuralDescription,
}

impl RewriteRegion {
    pub fn new(structural: StructuralDescription) -> Self {
        Self { structural }
    }

    /// The boolean array collection being iterated (e.g., `board` in BS001).
    pub fn collection(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::BooleanCollectionReduction { collection, .. } => return Ok(*collection),
                RegionRoles::PredicateCollectionReduction { collection, .. } => return Ok(*collection),
                RegionRoles::PositionSearch { collection, .. } => return collection.ok_or_else(|| RewriteError::MissingRole {
                        role: "collection".to_string(),
                    }),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "collection".to_string(),
        })
    }

    pub fn predicate_scalar(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::PredicateCollectionReduction { scalar, .. } => return Ok(*scalar),
                RegionRoles::PositionSearch { scalar, .. } => return scalar.ok_or_else(|| RewriteError::MissingRole {
                        role: "predicate_scalar".to_string(),
                    }),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "predicate_scalar".to_string(),
        })
    }

    pub fn predicate_op_node(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::PredicateCollectionReduction { operator, .. } => return Ok(*operator),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "predicate_op_node".to_string(),
        })
    }

    /// The final count/result produced by the region.
    pub fn result(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::BooleanCollectionReduction { result, .. } => return Ok(*result),
                RegionRoles::PredicateCollectionReduction { result, .. } => return Ok(*result),
                RegionRoles::ArithmeticOperation { result, .. } => return Ok(*result),
                RegionRoles::PositionSearch { result, .. } => return Ok(*result),
                RegionRoles::MaskOperation { result, .. } => return Ok(*result),
                RegionRoles::SetIteration { result, .. } => return Ok(*result),
            }
        }
        Err(RewriteError::MissingRole {
            role: "result".to_string(),
        })
    }

    /// Extract the mask operation's operand and result.
    pub fn mask_operation(&self) -> Result<(NodeId, NodeId), RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::MaskOperation { operand, result } => return Ok((*operand, *result)),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "MaskOperation".to_string(),
        })
    }

    /// The operator node.
    pub fn operator_node(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::ArithmeticOperation { operator_node, .. } => return Ok(*operator_node),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "operator_node".to_string(),
        })
    }

    /// The left operand.
    pub fn lhs(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::ArithmeticOperation { lhs, .. } => return Ok(*lhs),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "lhs".to_string(),
        })
    }

    /// The right operand.
    pub fn rhs(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::ArithmeticOperation { rhs, .. } => return Ok(*rhs),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "rhs".to_string(),
        })
    }

    /// The accumulator node, if one exists.
    pub fn accumulator(&self) -> Result<Option<NodeId>, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::BooleanCollectionReduction { accumulator, .. } => return Ok(*accumulator),
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "accumulator".to_string(),
        })
    }
}
