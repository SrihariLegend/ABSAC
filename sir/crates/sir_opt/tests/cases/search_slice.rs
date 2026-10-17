fn minus_one_sentinel_search_is_recognized_and_miss_translated() {
    // The conventional index-search convention returns -1 when the key is
    // absent, while the `return n` convention returns the bound. The
    // recognizer used to require the not-found value to BE the scan bound and
    // refused -1 outright. Both conventions must fire now, and -1 must
    // translate the helper's miss value (the bound) back to the sentinel.
    //
    // This is the shape that `clang -O3` compiles to a scalar byte loop: it
    // emits no memchr call and no vector instructions, because an early-exit
    // search has a data-dependent trip count.
    let ir = r#"
define dso_local i64 @find_first(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2) local_unnamed_addr {
  %4 = icmp sgt i64 %1, 0
  br i1 %4, label %5, label %13
5:
  %6 = phi i64 [ %11, %10 ], [ 0, %4 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %6
  %8 = load i8, ptr %7, align 1
  %9 = icmp eq i8 %8, %2
  br i1 %9, label %13, label %10
10:
  %11 = add nuw nsw i64 %6, 1
  %12 = icmp eq i64 %11, %1
  br i1 %12, label %13, label %5
13:
  %14 = phi i64 [ -1, %4 ], [ -1, %10 ], [ %6, %5 ]
  ret i64 %14
}
"#;
    let function =
        sir_lower::lower_function(ir, "find_first").expect("minus-one search must lower");
    let (optimized, definition) =
        sir_opt::optimize_search(&function).expect("minus-one search must optimize");
    assert_eq!(definition, 403, "runtime bound selects memchr 403");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "the search loop must be replaced"
    );
    assert!(
        optimized.arena.iter().any(|node| matches!(
            &node.kind,
            sir_nodes::NodeKind::Intrinsic { name, .. } if name == "sir_memchr"
        )),
        "the rewrite must use the sir_memchr primitive"
    );
    // The sentinel has to survive: the helper reports a miss as the scan
    // bound, so the rewrite must select the sentinel back over the helper's
    // result. Without this Select the code returns the bound where the source
    // returns -1 -- which is exactly what the referee caught at 483 of 500
    // cases when the sentinel was decoupled but not translated.
    assert!(
        optimized.arena.iter().any(|node| matches!(
            &node.kind,
            sir_nodes::NodeKind::Select { false_val, .. } if matches!(
                optimized.get_node(*false_val).map(|n| &n.kind),
                Some(sir_nodes::NodeKind::Intrinsic { name, .. }) if name == "sir_memchr"
            )
        )),
        "the miss value must be translated from the bound back to the sentinel"
    );
}

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
