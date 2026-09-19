#[test]
fn fixed_extent_all_and_chain_unrolls() {
    assert!(
        sir_opt::prove::and_laws(64),
        "the AND monoid laws must discharge"
    );
    let ll = std::fs::read_to_string("../../../gate6a/v6_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "p06_all_ne_const_u8").expect("lower");
    let optimized = sir_opt::optimize_all(&function).expect("All must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the All loop"
    );
    assert!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::And { .. }))
            .count()
            >= 64,
        "AND chain must cover the array"
    );
}
