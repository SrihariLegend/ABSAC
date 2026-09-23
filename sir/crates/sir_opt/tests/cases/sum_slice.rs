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

fn conditional_sum_over_pointer_unrolls() {
    let ll = std::fs::read_to_string("../../../gate6a/v16_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "s03_cond_sum_u8_64").expect("lower");
    let optimized = sir_opt::optimize_sum(&function).expect("conditional sum must optimize");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the conditional-sum loop"
    );
    assert!(
        optimized
            .arena
            .iter()
            .filter(|node| matches!(node.kind, sir_nodes::NodeKind::Select { .. }))
            .count()
            >= 64,
        "conditional map must be re-instantiated per index"
    );
}

fn runtime_bound_sum_uses_helper() {
    let ll = std::fs::read_to_string("../../../gate6a/v19b_fuzz_corpus.ll").expect("corpus");
    let function = sir_lower::lower_function(&ll, "f001").expect("lower");
    let optimized =
        sir_opt::optimize_runtime_sum(&function).expect("runtime sum must optimize");
    assert!(
        optimized.arena.iter().any(|node| matches!(
            &node.kind,
            sir_nodes::NodeKind::Intrinsic { name, .. } if name == "sir_sum_u32"
        )),
        "runtime sum must use the u32 helper"
    );
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "runtime sum loop must be replaced"
    );
}

/// A value-dependent conjunct of the termination (`i < 64 && flag`) only
/// bounds the loop: the flag can end it early, so the bound is not the
/// trip count and the unrolled sum would add elements the source loop
/// never reads. The same loop with a bare `Lt` termination is still
/// rewritten, so the refusal is attributable to the extra conjunct alone.
fn value_dependent_termination_conjunct_is_refused() {
    let guarded = r#"
define dso_local i64 @guarded_sum(ptr nocapture noundef readonly %0, ptr nocapture noundef readonly %1) local_unnamed_addr {
2:
  %g = icmp eq i64 64, 0
  br i1 %g, label %20, label %3
3:
  %i = phi i64 [ 0, %2 ], [ %i.next, %3 ]
  %s = phi i64 [ 0, %2 ], [ %s.next, %3 ]
  %ap = getelementptr inbounds i8, ptr %0, i64 %i
  %av = load i8, ptr %ap, align 1
  %av64 = zext i8 %av to i64
  %s.next = add i64 %s, %av64
  %fp = getelementptr inbounds i1, ptr %1, i64 %i
  %fv = load i1, ptr %fp, align 1
  %in = icmp ult i64 %i, 64
  %cont = and i1 %in, %fv
  %i.next = add nuw nsw i64 %i, 1
  br i1 %cont, label %3, label %20
20:
  %r = phi i64 [ 0, %2 ], [ %s.next, %3 ]
  ret i64 %r
}
"#;
    let function = sir_lower::lower_function(guarded, "guarded_sum")
        .expect("guarded sum loop must lower");
    let trips = sir_opt::family::with_facts(&function, |facts| {
        facts
            .loops
            .values()
            .map(|fact| fact.trip_count)
            .collect::<Vec<_>>()
    });
    assert_eq!(
        trips,
        vec![None],
        "a value-dependent guard must not yield a trip count"
    );
    for family in sir_opt::family::registry() {
        let applied =
            sir_opt::family::with_facts(&function, |facts| family.apply_one(&function, facts));
        assert!(
            applied.is_none(),
            "family {} must refuse a loop whose termination carries a value-dependent conjunct",
            family.name()
        );
    }

    let twin = r#"
define dso_local i64 @pure_sum(ptr nocapture noundef readonly %0) local_unnamed_addr {
2:
  %g = icmp eq i64 64, 0
  br i1 %g, label %20, label %3
3:
  %i = phi i64 [ 0, %2 ], [ %i.next, %3 ]
  %s = phi i64 [ 0, %2 ], [ %s.next, %3 ]
  %ap = getelementptr inbounds i8, ptr %0, i64 %i
  %av = load i8, ptr %ap, align 1
  %av64 = zext i8 %av to i64
  %s.next = add i64 %s, %av64
  %in = icmp ult i64 %i, 64
  %i.next = add nuw nsw i64 %i, 1
  br i1 %in, label %3, label %20
20:
  %r = phi i64 [ 0, %2 ], [ %s.next, %3 ]
  ret i64 %r
}
"#;
    let function =
        sir_lower::lower_function(twin, "pure_sum").expect("unguarded twin must lower");
    let (optimized, definition) = sir_opt::optimize_sum_with_definition(&function)
        .expect("the unguarded twin must still optimize");
    assert_eq!(definition, 400, "the unguarded twin is definition 400");
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "the unguarded twin's sum loop must be unrolled"
    );
}
