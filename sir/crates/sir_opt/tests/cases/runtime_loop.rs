fn runtime_bound_search_lowers_and_uses_memchr() {
    let ir = r#"
define dso_local i64 @naive_first_zero(ptr nocapture noundef readonly %0, i64 noundef %1) local_unnamed_addr {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %12, label %4
4:
  %5 = phi i64 [ %10, %9 ], [ 0, %2 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %5
  %7 = load i8, ptr %6, align 1
  %8 = icmp eq i8 %7, 0
  br i1 %8, label %12, label %9
9:
  %10 = add nuw i64 %5, 1
  %11 = icmp eq i64 %10, %1
  br i1 %11, label %12, label %4
12:
  %13 = phi i64 [ 0, %2 ], [ %1, %9 ], [ %5, %4 ]
  %14 = tail call i64 @llvm.umin.i64(i64 %13, i64 %1)
  ret i64 %14
}
"#;
    let function = sir_lower::lower_function(ir, "naive_first_zero")
        .expect("runtime-bound search loop must lower");
    let (optimized, definition) =
        sir_opt::optimize_search(&function).expect("runtime search must optimize");
    assert_eq!(definition, 403, "runtime search is memchr definition 403");
    assert!(
        optimized.arena.iter().any(|node| matches!(
            &node.kind,
            sir_nodes::NodeKind::Intrinsic { name, .. } if name == "sir_memchr"
        )),
        "runtime search must use the sir_memchr primitive"
    );
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the search loop"
    );
}

fn runtime_zero_test_loop_lowers() {
    let ir = r#"
define dso_local i32 @naive_popcount16(i16 noundef %0) local_unnamed_addr {
  %2 = icmp eq i16 %0, 0
  br i1 %2, label %10, label %3
3:
  %4 = phi i32 [ %8, %3 ], [ 0, %2 ]
  %5 = phi i16 [ %7, %3 ], [ %0, %2 ]
  %6 = add i16 %5, -1
  %7 = and i16 %6, %5
  %8 = add i32 %4, 1
  %9 = icmp eq i16 %7, 0
  br i1 %9, label %10, label %3
10:
  %11 = phi i32 [ 0, %2 ], [ %8, %3 ]
  ret i32 %11
}
"#;
    let function = sir_lower::lower_function(ir, "naive_popcount16")
        .expect("rotated zero-test loop must lower");
    let optimized =
        sir_opt::optimize_popcount(&function).expect("Kernighan popcount must optimize");
    assert!(
        optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Popcount { .. })),
        "optimized function must use Popcount"
    );
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Loop { .. })),
        "optimized function must not contain the Kernighan loop"
    );

    let ll = std::fs::read_to_string("../../../gate6a/v22_popcount64.ll").expect("corpus");
    let function =
        sir_lower::lower_function(&ll, "k01_kernighan_popcount_u64").expect("lower");
    let optimized =
        sir_opt::optimize_popcount(&function).expect("64-bit Kernighan must optimize");
    assert!(
        optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Popcount { .. })),
        "64-bit Kernighan must use Popcount"
    );
}

/// The SWAR bit-count chain is value-local: it replaces the fold
/// expression, not a loop, and it fires only when the three masks, the
/// fold constant and the fold shift are exactly the reference pattern.
/// The steps alone are refutable (`x = 0x80000000` steps to 16777216
/// where popcount is 1), so the fold is part of the object and a
/// deviating mask must refuse rather than guess.
fn swar_popcount_chain_rewrites_under_definition_8() {
    // Redis `redisPopcount`, per 32-bit word: three steps plus the
    // multiply fold.
    let multiply_ir = r#"
define dso_local i32 @swar32(i32 noundef %0) local_unnamed_addr {
  %1 = lshr i32 %0, 1
  %2 = and i32 %1, 1431655765
  %3 = sub i32 %0, %2
  %4 = lshr i32 %3, 2
  %5 = and i32 %4, 858993459
  %6 = and i32 %3, 858993459
  %7 = add i32 %5, %6
  %8 = lshr i32 %7, 4
  %9 = add i32 %8, %7
  %10 = and i32 %9, 252645135
  %11 = mul i32 %10, 16843009
  %12 = lshr i32 %11, 24
  ret i32 %12
}
"#;
    let function = sir_lower::lower_function(multiply_ir, "swar32")
        .expect("SWAR chain must lower");
    let (optimized, rewrites, definitions) =
        sir_opt::optimize(&function, &|_| true).expect("SWAR chain must optimize");
    assert_eq!(definitions, vec![8], "the SWAR chain is definition 8");
    assert_eq!(rewrites, 1, "one rewrite: the fold becomes a Popcount");
    assert!(
        optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Popcount { .. })),
        "the SWAR fold must be replaced by a Popcount"
    );
    assert!(
        !optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Mul { .. })),
        "the SWAR chain must not survive the rewrite"
    );

    // The same object with the fold spelled as a shift-add tree, so the
    // word carries no multiply at all.
    let shift_add_ir = r#"
define dso_local i32 @swar32_sa(i32 noundef %0) local_unnamed_addr {
  %1 = lshr i32 %0, 1
  %2 = and i32 %1, 1431655765
  %3 = sub i32 %0, %2
  %4 = lshr i32 %3, 2
  %5 = and i32 %4, 858993459
  %6 = and i32 %3, 858993459
  %7 = add i32 %5, %6
  %8 = lshr i32 %7, 4
  %9 = add i32 %8, %7
  %10 = and i32 %9, 252645135
  %11 = shl i32 %10, 8
  %12 = add i32 %10, %11
  %13 = shl i32 %10, 16
  %14 = add i32 %12, %13
  %15 = shl i32 %10, 24
  %16 = add i32 %14, %15
  %17 = lshr i32 %16, 24
  ret i32 %17
}
"#;
    let function = sir_lower::lower_function(shift_add_ir, "swar32_sa")
        .expect("shift-add spelling must lower");
    let (optimized, _, definitions) =
        sir_opt::optimize(&function, &|_| true).expect("shift-add spelling must optimize");
    assert_eq!(definitions, vec![8], "both fold spellings are definition 8");
    assert!(
        optimized
            .arena
            .iter()
            .any(|node| matches!(node.kind, sir_nodes::NodeKind::Popcount { .. })),
        "the shift-add fold must be replaced by a Popcount"
    );

    // Negative controls. One bit of the first mask and one unit of the
    // fold shift each name a different function, and the difference is
    // not something the recognizer may guess at.
    for (label, ir) in [
        (
            "mask 0x55555554",
            multiply_ir.replace("1431655765", "1431655764"),
        ),
        (
            "fold shift 23",
            multiply_ir.replacen("lshr i32 %11, 24", "lshr i32 %11, 23", 1),
        ),
    ] {
        let function = sir_lower::lower_function(&ir, "swar32")
            .unwrap_or_else(|error| panic!("{label} must lower: {error}"));
        assert!(
            sir_opt::optimize_popcount(&function).is_none(),
            "a SWAR chain with {label} must be refused, not guessed at"
        );
    }
}

/// A Kernighan loop whose source carries an effect must be refused. The
/// store shape is a live miscompile if rewritten: `Builder::r#loop` types
/// the loop node from its body, so the loop-level guard sees the store,
/// and the rewrite would delete the loop while the non-pure store
/// survives as a single straight-line write. The call shape leaves the
/// loop node itself pure (the call is a carried input defined in the
/// preheader), so only the per-node guard over `carried_inputs` sees it.
fn effectful_kernighan_loops_are_refused() {
    let store_ir = r#"
define dso_local i32 @pc_store(i64 noundef %0, ptr nocapture noundef writeonly %1) local_unnamed_addr {
  %3 = icmp eq i64 %0, 0
  br i1 %3, label %11, label %4
4:
  %5 = phi i32 [ %9, %4 ], [ 0, %2 ]
  %6 = phi i64 [ %8, %4 ], [ %0, %2 ]
  store i32 %5, ptr %1, align 4
  %7 = add i64 %6, -1
  %8 = and i64 %7, %6
  %9 = add i32 %5, 1
  %10 = icmp eq i64 %8, 0
  br i1 %10, label %11, label %4
11:
  %12 = phi i32 [ 0, %2 ], [ %9, %4 ]
  ret i32 %12
}
"#;
    let function = sir_lower::lower_function(store_ir, "pc_store").expect("lower");
    assert!(
        function.arena.iter().any(|node| {
            matches!(&node.kind, sir_nodes::NodeKind::Loop { body, .. }
                if body.iter().any(|id| matches!(
                    function.get_node(*id).map(|inner| &inner.kind),
                    Some(sir_nodes::NodeKind::Store { .. })
                )))
        }),
        "fixture must lower the store into the Kernighan loop body"
    );
    assert!(
        sir_opt::optimize_popcount(&function).is_none(),
        "a Kernighan loop carrying a store must be refused, not rewritten to Popcount"
    );

    let call_ir = r#"
declare i64 @ext(i64 noundef)
define dso_local i32 @pc_call(i64 noundef %0) local_unnamed_addr {
  %2 = call i64 @ext(i64 noundef %0)
  %3 = icmp eq i64 %2, 0
  br i1 %3, label %11, label %4
4:
  %5 = phi i32 [ %9, %4 ], [ 0, %1 ]
  %6 = phi i64 [ %8, %4 ], [ %2, %1 ]
  %7 = add i64 %6, -1
  %8 = and i64 %7, %6
  %9 = add i32 %5, 1
  %10 = icmp eq i64 %8, 0
  br i1 %10, label %11, label %4
11:
  %12 = phi i32 [ 0, %1 ], [ %9, %4 ]
  ret i32 %12
}
"#;
    let function = sir_lower::lower_function(call_ir, "pc_call").expect("lower");
    assert!(
        function.arena.iter().any(|node| {
            matches!(&node.kind, sir_nodes::NodeKind::Loop { carried_inputs, .. }
                if node.effects.is_pure()
                    && carried_inputs.iter().any(|id| {
                        function.get_node(*id).is_some_and(|inner| {
                            inner.effects.contains(sir_types::Effects::WRITE_MEMORY)
                        })
                    }))
        }),
        "fixture must carry an effectful value into an otherwise pure loop"
    );
    assert!(
        sir_opt::optimize_popcount(&function).is_none(),
        "a Kernighan loop carrying an effectful value must be refused, not rewritten to Popcount"
    );
}

/// A loop whose *condition* carries effects: the call is defined in the
/// preheader and used directly as the branch condition, so it is a member
/// of neither `body`, `outputs`, nor `carried_inputs`. The loop node
/// stays pure (nothing in the body reads or writes), so the loop-level
/// guard cannot fire and the per-node guard over the three collections
/// cannot see the call. Every family must still refuse the shape on its
/// own recognizer pins (trip count, termination kind, bound guard): the
/// same loop with a pure `Lt` bound in place of the call is accepted
/// (runtime-sum, definition 404), so the refusal is attributable to the
/// condition node alone.
fn effectful_loop_conditions_are_refused() {
    let ir = r#"
declare i1 @ext(i32 noundef)
define dso_local i64 @cond_call(ptr nocapture noundef readonly %0) local_unnamed_addr {
2:
  %c = call i1 @ext(i32 noundef 0)
  %3 = icmp eq i64 64, 0
  br i1 %3, label %13, label %4
4:
  %5 = phi i64 [ 0, %2 ], [ %11, %4 ]
  %6 = phi i64 [ 0, %2 ], [ %10, %4 ]
  %7 = getelementptr inbounds i8, ptr %0, i64 %5
  %8 = load i8, ptr %7, align 1
  %9 = zext i8 %8 to i64
  %10 = add i64 %6, %9
  %11 = add nuw nsw i64 %5, 1
  br i1 %c, label %4, label %13
13:
  %14 = phi i64 [ 0, %2 ], [ %10, %4 ]
  ret i64 %14
}
"#;
    let function = sir_lower::lower_function(ir, "cond_call")
        .expect("a preheader call used as the loop condition must lower");
    assert!(
        function.arena.iter().any(|node| {
            matches!(&node.kind, sir_nodes::NodeKind::Loop { termination, .. }
                if node.effects.is_pure()
                    && function.get_node(*termination).is_some_and(|inner| {
                        inner.ty == sir_types::Type::Bool
                            && !(inner.effects - sir_types::Effects::READ_MEMORY).is_empty()
                    }))
        }),
        "fixture must lower to a pure loop whose Bool condition carries effects"
    );
    for family in sir_opt::family::registry() {
        let applied =
            sir_opt::family::with_facts(&function, |facts| family.apply_one(&function, facts));
        assert!(
            applied.is_none(),
            "family {} must refuse a loop whose condition carries effects",
            family.name()
        );
    }
}
