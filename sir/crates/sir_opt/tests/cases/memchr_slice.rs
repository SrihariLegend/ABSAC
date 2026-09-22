fn fixed_extent_byte_search_uses_library_primitive() {
    assert!(
        sir_opt::prove::first_match_law(),
        "the first-match scan equivalence must discharge"
    );
    let ll = std::fs::read_to_string("../../../gate6a/v8_corpus.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "p04_first_eq_key_u8_64").expect("lower");
    let optimized = sir_opt::optimize_memchr(&function).expect("memchr must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the search loop"
    );
    assert!(
        optimized.arena.iter().any(|node| matches!(
            &node.kind,
            sir_nodes::NodeKind::Intrinsic { name, .. } if name == "sir_memchr"
        )),
        "optimized function must use the sir_memchr primitive"
    );
}
