#[test]
fn fixed_extent_any_or_reduction_unrolls() {
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "p07_any_or_const_u32").expect("lower");
    let optimized = sir_opt::optimize_any(&function).expect("Any must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the reduction loop"
    );
    assert!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Or { .. }))
            .count()
            >= 63,
        "OR chain must cover the array"
    );
}
