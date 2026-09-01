use sir_semantics::binding::ProposalBinding;
use sir_semantics::structure::StructuralDescription;
use sir_transform::roles::RegionRoles;
use sir_types::NodeId;

use crate::error::RewriteError;

/// A transient execution object assembled by `RewriteEngine` at rewrite time.
///
/// Wraps the `StructuralDescription` (which carries `RegionRoles` assigned by
/// semantic recognition) and — for reduction regions — the derived
/// `ProposalBinding`: the exact role map + complete observable interface
/// + application frame. Recipes MUST consume the binding's role map;
/// scanning the source function to rediscover semantic roles after a
/// successful binding is forbidden (advisor invariant: once
/// ProposalBinding succeeds, no later stage scans the source function
/// to guess semantic roles).
///
/// Not persisted — assembled fresh for each rewrite.
#[derive(Clone, Debug)]
pub struct RewriteRegion {
    /// The structural description from semantic recognition.
    pub structural: StructuralDescription,
    /// The application binding (present when the canonical binder
    /// derived one for this region in THIS function version).
    pub binding: Option<ProposalBinding>,
}

impl RewriteRegion {
    pub fn new(structural: StructuralDescription) -> Self {
        Self {
            structural,
            binding: None,
        }
    }

    /// Attach the derived ProposalBinding (engine-side; the recipe
    /// never derives roles itself).
    pub fn with_binding(mut self, binding: ProposalBinding) -> Self {
        self.binding = Some(binding);
        self
    }

    /// The boolean array collection being iterated (e.g., `board` in BS001).
    pub fn collection(&self) -> Result<NodeId, RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::BooleanCollectionReduction { collection, .. } => {
                    return Ok(*collection)
                }
                RegionRoles::PredicateCollectionReduction { collection, .. } => {
                    return Ok(*collection)
                }
                RegionRoles::PositionSearch { collection, .. } => {
                    return collection.ok_or_else(|| RewriteError::MissingRole {
                        role: "collection".to_string(),
                    })
                }
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
                RegionRoles::PositionSearch { scalar, .. } => {
                    return scalar.ok_or_else(|| RewriteError::MissingRole {
                        role: "predicate_scalar".to_string(),
                    })
                }
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
        // Whole-loop patterns win over sub-expression results: for a
        // BitsetIteration loop, the region's result is the loop itself, not
        // the mask operation inside its body (which may appear earlier in the
        // roles vector).
        for role in &self.structural.roles {
            if let RegionRoles::SetIteration { result, .. } = role {
                return Ok(*result);
            }
        }
        // Wholesale permutations win over constituent shift/mask roles: the
        // recipe replaces the entire permutation result, never one of its
        // subexpressions (HD012 lesson).
        for role in &self.structural.roles {
            if let RegionRoles::BitPermutation { result, .. } = role {
                return Ok(*result);
            }
        }
        for role in &self.structural.roles {
            match role {
                RegionRoles::BooleanCollectionReduction { result, .. } => return Ok(*result),
                RegionRoles::PredicateCollectionReduction { result, .. } => return Ok(*result),
                RegionRoles::ArithmeticOperation { result, .. } => return Ok(*result),
                RegionRoles::PositionSearch { result, .. } => return Ok(*result),
                RegionRoles::MaskOperation { result, .. } => return Ok(*result),
                _ => {} // SetIteration/BitPermutation handled above; other roles carry no result.
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
                RegionRoles::ArithmeticOperation { operator_node, .. } => {
                    return Ok(*operator_node)
                }
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
                RegionRoles::BooleanCollectionReduction { accumulator, .. } => {
                    return Ok(*accumulator)
                }
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "accumulator".to_string(),
        })
    }

    /// The permutation's operand (the value being permuted) and its kind.
    pub fn permutation(
        &self,
    ) -> Result<(NodeId, sir_transform::roles::PermutationKind), RewriteError> {
        for role in &self.structural.roles {
            match role {
                RegionRoles::BitPermutation { operand, kind, .. } => {
                    return Ok((*operand, kind.clone()))
                }
                _ => {}
            }
        }
        Err(RewriteError::MissingRole {
            role: "BitPermutation".to_string(),
        })
    }
}
