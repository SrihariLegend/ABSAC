// The sum vertical slice must recognize, prove, and unroll a
// fixed-extent raw-element sum from the corpus.

fn fixed_extent_sum_unrolls() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "p04_sum_u8_const_u64").expect("lower");
    assert!(
        function
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "fixture must contain a loop"
    );

    let optimized = sir_opt::optimize_sum(&function).expect("sum slice must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the sum loop"
    );

    let accesses = optimized
        .arena
        .iter()
        .filter(|node| matches!(node.kind, sir_nodes::NodeKind::ArrayAccess { .. }))
        .count();
    let adds = optimized
        .arena
        .iter()
        .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Add { .. }))
        .count();
    assert_eq!(accesses, 128, "every element must be materialized");
    assert!(adds >= 127, "unrolled sum must build the add chain");
}

fn mapped_sum_reinstantiates_map_per_index() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "p10_map_then_sum_const").expect("lower");
    let optimized = sir_opt::optimize_sum(&function).expect("mapped sum must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the mapped-sum loop"
    );
    assert_eq!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Xor { .. }))
            .count(),
        64,
        "the map must be re-instantiated per index"
    );
    assert_eq!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::ArrayAccess { .. }))
            .count(),
        64,
        "every mapped element must be materialized"
    );
}
