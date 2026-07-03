//! Analysis dependency graph.
//!
//! Each analysis declares what other analyses it depends on.
//! The AnalysisManager uses this to compute analyses in dependency order.
//! All dependency edges point from consumer → producer (the analysis
//! that needs the result → the analysis that produces it).

use std::any::TypeId;

/// Return the TypeId of the given analysis type.
pub fn type_id_of<A: 'static>() -> TypeId {
    TypeId::of::<A>()
}

/// Dependencies for each analysis (consumer → [producers]).
///
/// An analysis depends on another if it reads that analysis's facts
/// from the FactDatabase. The manager guarantees that producers
/// run before consumers.
///
/// In v0.1, most analyses are self-contained (use `graph.rs` directly).
/// Dependencies will grow as analyses start reading each other's facts.
pub fn dependencies_of(analysis_id: TypeId) -> Vec<TypeId> {
    // UseDef: fundamental — no dependencies.
    // Dominance: uses predecessor_map from graph.rs, not use_def facts.
    // Constants: builds its own user map (will switch to use_def in v0.2).
    // Purity: uses dataflow_inputs from graph.rs.
    // Ranges: self-contained interval arithmetic.
    // Alias: self-contained allocation-site tracking.
    // Escape: builds its own reverse map.
    // Loops: self-contained.
    // ValueNumbering: self-contained hash computation.
    //
    // In v0.1, all analyses are independent. Future phases will add edges.
    let _ = analysis_id;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_id_is_deterministic() {
        let a = type_id_of::<String>();
        let b = type_id_of::<String>();
        assert_eq!(a, b);
    }

    #[test]
    fn no_dependencies_in_v0_1() {
        // All analyses are self-contained in v0.1.
        let deps = dependencies_of(type_id_of::<String>());
        assert!(deps.is_empty());
    }
}
