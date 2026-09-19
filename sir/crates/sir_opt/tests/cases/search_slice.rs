fn fixed_extent_inequality_search_is_retained_only_when_measured_faster() {
    let ll = std::fs::read_to_string("../../../gate6a/v9_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "p02_first_ne_key_u8_64").expect("lower");
    assert!(
        sir_opt::optimize_search(&function).is_none(),
        "the unrolled select chain is not retained at extent 64"
    );
    assert!(
        sir_opt::optimize_memchr(&function).is_none(),
        "inequality search must not use the memchr primitive"
    );

    // A fixed-extent search over a raw pointer has no array length in the
    // type; the constant bound supplies the extent.
    let ll = std::fs::read_to_string("../../../gate6a/v14_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "s01_search_a_while_accessing_b").expect("lower");
    let (optimized, definition) =
        sir_opt::optimize_search(&function).expect("pointer search must optimize");
    assert_eq!(definition, 402);
    assert!(
        optimized.arena.iter().any(|node| matches!(
            &node.kind,
            sir_nodes::NodeKind::Intrinsic { name, .. } if name == "sir_memchr"
        )),
        "pointer search must use the sir_memchr primitive"
    );
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "pointer search loop must be replaced"
    );
}
