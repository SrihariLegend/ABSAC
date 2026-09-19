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
}
