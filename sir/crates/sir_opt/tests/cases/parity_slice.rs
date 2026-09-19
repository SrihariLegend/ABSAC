fn fixed_extent_xor_reduction_unrolls() {
    assert!(
        sir_opt::prove::xor_laws(64),
        "the XOR monoid laws must discharge"
    );
    let ll = std::fs::read_to_string("../../../gate6a/v19_fuzz_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "f038").expect("lower");
    let optimized = sir_opt::optimize_parity(&function).expect("parity must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the parity loop"
    );
    assert_eq!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::ArrayAccess { .. }))
            .count(),
        96,
        "every predicate element must be materialized"
    );
    assert!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Xor { .. }))
            .count()
            >= 95,
        "XOR chain must cover the extent"
    );
}
