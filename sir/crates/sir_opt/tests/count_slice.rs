#[test]
fn fixed_extent_cardinality_unrolls_predicates() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "p03_count_lt_const_u32").expect("lower");
    let optimized = sir_opt::optimize_count(&function).expect("cardinality must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the count loop"
    );
    assert_eq!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Lt { .. }))
            .count(),
        64,
        "every predicate must be re-instantiated"
    );
    assert!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Add { .. }))
            .count()
            >= 63,
        "indicator add chain must cover the array"
    );
}
