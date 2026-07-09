use std::collections::BTreeSet;

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
        Self {
            structural,
        }
    }

    /// The boolean array collection being iterated (e.g., `board` in BS001).
    pub fn collection(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::BooleanCollectionReduction { collection, .. }) => Ok(*collection),
            Some(RegionRoles::PredicateCollectionReduction { collection, .. }) => Ok(*collection),
            _ => Err(RewriteError::MissingRole {
                role: "collection".to_string(),
            }),
        }
    }

    pub fn predicate_scalar(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::PredicateCollectionReduction { scalar, .. }) => Ok(*scalar),
            _ => Err(RewriteError::MissingRole {
                role: "predicate_scalar".to_string(),
            }),
        }
    }

    pub fn predicate_op_node(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::PredicateCollectionReduction { operator, .. }) => Ok(*operator),
            _ => Err(RewriteError::MissingRole {
                role: "predicate_op_node".to_string(),
            }),
        }
    }

    /// The final count/result produced by the region.
    pub fn result(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::BooleanCollectionReduction { result, .. }) => Ok(*result),
            Some(RegionRoles::PredicateCollectionReduction { result, .. }) => Ok(*result),
            Some(RegionRoles::ArithmeticOperation { result, .. }) => Ok(*result),
            _ => Err(RewriteError::MissingRole {
                role: "result".to_string(),
            }),
        }
    }

    /// The operator node.
    pub fn operator_node(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::ArithmeticOperation { operator_node, .. }) => Ok(*operator_node),
            _ => Err(RewriteError::MissingRole {
                role: "operator_node".to_string(),
            }),
        }
    }

    /// The left operand.
    pub fn lhs(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::ArithmeticOperation { lhs, .. }) => Ok(*lhs),
            _ => Err(RewriteError::MissingRole {
                role: "lhs".to_string(),
            }),
        }
    }

    /// The right operand.
    pub fn rhs(&self) -> Result<NodeId, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::ArithmeticOperation { rhs, .. }) => Ok(*rhs),
            _ => Err(RewriteError::MissingRole {
                role: "rhs".to_string(),
            }),
        }
    }

    /// The accumulator node, if one exists.
    pub fn accumulator(&self) -> Result<Option<NodeId>, RewriteError> {
        match &self.structural.roles {
            Some(RegionRoles::BooleanCollectionReduction {
                accumulator, ..
            }) => Ok(*accumulator),
            _ => Err(RewriteError::MissingRole {
                role: "accumulator".to_string(),
            }),
        }
    }
}
