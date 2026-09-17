//! Lowering regressions for the Gate 6A corpus findings.
//!
//! Every snippet is self-contained LLVM IR (no corpus dependency) so the
//! failures that batch_run surfaced — w07_all_min / h02_all_match /
//! n14_saturating_count (select literal arms typed as u64), w08/v07
//! (two loops half-lowered into a MissingReturn), x08/n03/n08 (loops with
//! a separate latch block dying on raw resolution errors) — are pinned by
//! `cargo test`.

use sir_lower::lower_function;

/// A canonical single-block sum scan. Guard: the recognized loop shape must
/// keep lowering after the control-flow gates are added.
const SUM_SCAN: &str = r#"
define i64 @sum_scan(ptr %b, i64 %n) {
entry:
  %c0 = icmp eq i64 %n, 0
  br i1 %c0, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %s = phi i64 [ 0, %entry ], [ %s2, %loop ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %z = zext i8 %e to i64
  %s2 = add i64 %s, %z
  %i2 = add i64 %i, 1
  %d = icmp eq i64 %i2, %n
  br i1 %d, label %exit, label %loop
exit:
  %r = phi i64 [ 0, %entry ], [ %s2, %loop ]
  ret i64 %r
}
"#;

/// w07_all_min shape: an i8 sticky accumulator whose update select carries
/// a literal arm (`i8 0`) while the other arm is the real i8 accumulator.
/// Regression: the literal used to be typed u64, refusing the loop with a
/// TypeMismatch in `builder.select`.
const I8_SELECT_LITERAL: &str = r#"
define i8 @all_below(ptr %b, i64 %n, i8 %lim) {
entry:
  %c0 = icmp eq i64 %n, 0
  br i1 %c0, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %acc = phi i8 [ 1, %entry ], [ %sel, %loop ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %below = icmp ult i8 %e, %lim
  %sel = select i1 %below, i8 0, i8 %acc
  %i2 = add i64 %i, 1
  %d = icmp eq i64 %i2, %n
  br i1 %d, label %exit, label %loop
exit:
  %r = phi i8 [ 1, %entry ], [ %sel, %loop ]
  ret i8 %r
}
"#;

/// h02_all_match shape: literal in the FALSE arm, real arm i32.
const I32_SELECT_LITERAL: &str = r#"
define i32 @all_match(ptr %b, i64 %n, i8 %x) {
entry:
  %c0 = icmp eq i64 %n, 0
  br i1 %c0, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %acc = phi i32 [ 1, %entry ], [ %sel, %loop ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %eq = icmp eq i8 %e, %x
  %sel = select i1 %eq, i32 %acc, i32 0
  %i2 = add i64 %i, 1
  %d = icmp eq i64 %i2, %n
  br i1 %d, label %exit, label %loop
exit:
  %r = phi i32 [ 1, %entry ], [ %sel, %loop ]
  ret i32 %r
}
"#;

/// n14_saturating_count shape: an i1 select whose false arm is the literal
/// `false` — never resolvable before the Boolean-literal parser.
const I1_BOOL_LITERAL: &str = r#"
define i64 @saturating_count(ptr %b, i64 %n, i8 %x) {
entry:
  %c0 = icmp eq i64 %n, 0
  br i1 %c0, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %c = phi i64 [ 0, %entry ], [ %c2, %loop ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %hit = icmp eq i8 %e, %x
  %below = icmp ult i64 %c, 1000
  %add = select i1 %hit, i1 %below, i1 false
  %z = zext i1 %add to i64
  %c2 = add i64 %c, %z
  %i2 = add i64 %i, 1
  %d = icmp eq i64 %i2, %n
  br i1 %d, label %exit, label %loop
exit:
  %r = phi i64 [ 0, %entry ], [ %c2, %loop ]
  ret i64 %r
}
"#;

/// w08_two_reductions / v07_count_then_sum shape: TWO single-block loops
/// sharing an exit CFG. The lowerer can only emit one loop; this must be an
/// explicit unsupported refusal, never a half-lowered function without a
/// Return (previously caught only downstream as MissingReturn).
const TWO_LOOPS_SHARED_EXIT: &str = r#"
define i64 @two_loops(ptr %b, i64 %n, i8 %x) {
entry:
  br label %inner_exit
inner_exit:
  %t0 = phi i64 [ 0, %entry ], [ %n0, %inner ]
  %c0 = icmp eq i64 %n, 0
  br i1 %c0, label %outer, label %inner
inner:
  %i = phi i64 [ 0, %inner_exit ], [ %i2, %inner ]
  %s = phi i64 [ 0, %inner_exit ], [ %s2, %inner ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %hit = icmp eq i8 %e, %x
  %z = zext i1 %hit to i64
  %s2 = add i64 %s, %z
  %i2 = add i64 %i, 1
  %d1 = icmp eq i64 %i2, %n
  br i1 %d1, label %outer, label %inner
outer:
  %a = phi i64 [ 0, %inner_exit ], [ %a2, %outer ]
  %j = phi i64 [ 0, %inner_exit ], [ %j2, %outer ]
  %e2 = load i8, ptr %b
  %z2 = zext i8 %e2 to i64
  %a2 = add i64 %a, %z2
  %j2 = add i64 %j, 1
  %d2 = icmp eq i64 %j2, %n
  br i1 %d2, label %fin, label %outer
fin:
  %r = phi i64 [ 0, %inner_exit ], [ %a2, %outer ]
  ret i64 %r
}
"#;

/// x08_early_exit_write / n03 / n08 shape: a loop whose back-edge comes
/// from a SEPARATE latch block (header phi names the latch, not itself).
/// The single-block detector misses it; this must be an explicit refusal,
/// not a raw "cannot resolve gep index" mid-instruction.
const SEPARATE_LATCH_LOOP: &str = r#"
define i64 @early_exit(ptr %b, i64 %n, i8 %x) {
entry:
  %c0 = icmp eq i64 %n, 0
  br i1 %c0, label %exit, label %hdr
hdr:
  %i = phi i64 [ 0, %entry ], [ %i2, %latch ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %f = icmp eq i8 %e, %x
  br i1 %f, label %exit, label %latch
latch:
  %i2 = add i64 %i, 1
  %d = icmp eq i64 %i2, %n
  br i1 %d, label %exit, label %hdr
exit:
  %r = phi i64 [ 0, %entry ], [ %i, %hdr ], [ %n, %latch ]
  ret i64 %r
}
"#;

fn lowers_ok(text: &str, name: &str) -> sir_nodes::Function {
    match lower_function(text, name) {
        Ok(f) => f,
        Err(e) => panic!("expected {name} to lower, got: {e}"),
    }
}

fn refuses_cleanly(text: &str, name: &str, needle: &str) {
    match lower_function(text, name) {
        Ok(_) => panic!("expected {name} to be refused as unsupported"),
        Err(e) => assert!(
            e.contains(needle),
            "refusal for {name} should cite '{needle}', got: {e}"
        ),
    }
}

#[test]
fn canonical_sum_scan_still_lowers() {
    let f = lowers_ok(SUM_SCAN, "sum_scan");
    assert!(f.arena.len() > 8, "expected a real loop body, got {}", f.arena.len());
}

#[test]
fn i8_select_literal_arm_lowers_and_is_typed_i8() {
    let f = lowers_ok(I8_SELECT_LITERAL, "all_below");
    // Find the select and confirm both integer arms carry the i8 the LLVM
    // IR declares (the pre-fix behavior typed the literal `0` as u64 and
    // refused the whole loop).
    let select_nodes: Vec<&sir_nodes::Node> = f
        .arena
        .nodes()
        .values()
        .filter(|n| matches!(n.kind, sir_nodes::NodeKind::Select { .. }))
        .collect();
    assert_eq!(select_nodes.len(), 1, "expected exactly one select");
    if let sir_nodes::NodeKind::Select { true_val, false_val, .. } = &select_nodes[0].kind {
        for arm in [*true_val, *false_val] {
            let ty = f.arena.get(arm).expect("arm present").ty.clone();
            assert_eq!(ty, sir_types::Type::u8(), "select arm must be i8");
        }
    }
}

#[test]
fn i32_select_literal_arm_lowers() {
    let f = lowers_ok(I32_SELECT_LITERAL, "all_match");
    let selects: Vec<&sir_nodes::Node> = f
        .arena
        .nodes()
        .values()
        .filter(|n| matches!(n.kind, sir_nodes::NodeKind::Select { .. }))
        .collect();
    assert_eq!(selects.len(), 1);
}

#[test]
fn i1_false_literal_select_lowers() {
    let f = lowers_ok(I1_BOOL_LITERAL, "saturating_count");
    let selects: Vec<&sir_nodes::Node> = f
        .arena
        .nodes()
        .values()
        .filter(|n| matches!(n.kind, sir_nodes::NodeKind::Select { .. }))
        .collect();
    assert_eq!(selects.len(), 1, "bool select must lower to a Select node");
}

#[test]
fn two_loops_sharing_exit_cfg_refused_cleanly() {
    refuses_cleanly(TWO_LOOPS_SHARED_EXIT, "two_loops", "multiple loops");
}

#[test]
fn separate_latch_loop_refused_cleanly() {
    refuses_cleanly(SEPARATE_LATCH_LOOP, "early_exit", "separate latch block");
}

/// F10 (H4b, 2026-09-16): a global array's SIR parameter used to be
/// hardcoded to `Array<u8, 256>`, so any global whose declared extent was
/// not 256 made application binding refuse with "counted loop does not
/// cover the complete collection extent". The GEP source element type
/// (`inbounds [64 x i8]`) must be used instead.
const GLOBAL_EXTENT_64: &str = r#"
define i64 @global_or_64() {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %acc = phi i8 [ 0, %entry ], [ %acc2, %loop ]
  %p = getelementptr inbounds [64 x i8], ptr @g64, i64 0, i64 %i
  %e = load i8, ptr %p
  %acc2 = or i8 %acc, %e
  %i2 = add nuw nsw i64 %i, 1
  %d = icmp eq i64 %i2, 64
  br i1 %d, label %exit, label %loop
exit:
  %nz = icmp ne i8 %acc2, 0
  %z = zext i1 %nz to i64
  ret i64 %z
}
"#;

#[test]
fn global_array_extent_follows_the_gep_source_type() {
    let f = lowers_ok(GLOBAL_EXTENT_64, "global_or_64");
    let extents: Vec<usize> = f
        .params
        .iter()
        .filter_map(|p| match &p.ty {
            sir_types::Type::Array { length, .. } => Some(*length),
            _ => None,
        })
        .collect();
    assert_eq!(
        extents,
        vec![64],
        "global array parameter must carry the GEP-declared extent 64, not the historical 256"
    );
}

/// Gate 6B fusion enabler: a sequential-loop kernel (count + sum over the
/// same buffer) must outline into one single-loop region per reduction,
/// each of which lowers on its own with the original parameters in their
/// original positions.
const TWO_LOOP_COUNT_SUM: &str = r#"
define dso_local i64 @two(ptr nocapture noundef readonly %0, i64 noundef %1, i8 noundef zeroext %2, i8 noundef zeroext %3) local_unnamed_addr #0 {
  %5 = icmp eq i64 %1, 0
  br i1 %5, label %6, label %9

6:                                                ; preds = %9, %4
  %7 = phi i64 [ 0, %4 ], [ %17, %9 ]
  %8 = icmp eq i64 %1, 0
  br i1 %8, label %20, label %23

9:                                                ; preds = %4, %9
  %10 = phi i64 [ %18, %9 ], [ 0, %4 ]
  %11 = phi i64 [ %17, %9 ], [ 0, %4 ]
  %12 = getelementptr inbounds i8, ptr %0, i64 %10
  %13 = load i8, ptr %12, align 1, !tbaa !5
  %14 = and i8 %13, %2
  %15 = icmp eq i8 %14, %3
  %16 = zext i1 %15 to i64
  %17 = add i64 %11, %16
  %18 = add nuw i64 %10, 1
  %19 = icmp eq i64 %18, %1
  br i1 %19, label %6, label %9, !llvm.loop !8

20:                                               ; preds = %23, %6
  %21 = phi i64 [ 0, %6 ], [ %29, %23 ]
  %22 = xor i64 %21, %7
  ret i64 %22

23:                                               ; preds = %6, %23
  %24 = phi i64 [ %30, %23 ], [ 0, %6 ]
  %25 = phi i64 [ %29, %23 ], [ 0, %6 ]
  %26 = getelementptr inbounds i8, ptr %0, i64 %24
  %27 = load i8, ptr %26, align 1, !tbaa !5
  %28 = zext i8 %27 to i64
  %29 = add i64 %25, %28
  %30 = add nuw i64 %24, 1
  %31 = icmp eq i64 %30, %1
  br i1 %31, label %20, label %23, !llvm.loop !11
}
"#;

#[test]
fn sequential_loops_outline_and_lower_per_region() {
    let regions = sir_lower::extract_loop_regions(TWO_LOOP_COUNT_SUM, "two")
        .expect("two-loop function must outline");
    assert_eq!(regions.len(), 2, "one region per self-latch loop");
    for (k, r) in regions.iter().enumerate() {
        let f = lower_function(&r.text, &r.name)
            .unwrap_or_else(|e| panic!("region {k} must lower: {e}"));
        assert_eq!(f.params.len(), r.params.len(), "region params preserved");
        assert_eq!(r.original_params, 4, "original params keep their positions");
        assert!(
            r.dep_sources.is_empty(),
            "independent loops over the same buffer must not report dependencies"
        );
    }
}

// ── F6–F8 emitter-closure regressions (native loop emission) ─────────

/// H3 F7: an opaque `ptr` parameter indexed as `i16` must lower to a
/// `*u16` SIR parameter, not the historical `*u8` byte view. Otherwise
/// the interpreter and the C emitter both read 8-bit elements.
const U16_COUNT: &str = r#"
define i64 @count_ge_u16(ptr nocapture noundef readonly %0, i64 noundef %1, i16 noundef zeroext %2) {
entry:
  %c0 = icmp eq i64 %1, 0
  br i1 %c0, label %exit, label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %acc2, %loop ]
  %p = getelementptr inbounds i16, ptr %0, i64 %i
  %e = load i16, ptr %p
  %ge = icmp uge i16 %e, %2
  %z = zext i1 %ge to i64
  %acc2 = add i64 %acc, %z
  %i2 = add nuw i64 %i, 1
  %d = icmp eq i64 %i2, %1
  br i1 %d, label %exit, label %loop
exit:
  %r = phi i64 [ 0, %entry ], [ %acc2, %loop ]
  ret i64 %r
}
"#;

/// The same pointer viewed as bytes and as `i16` — the byte view wins
/// (never guess an element width through an aliased pointer).
const MIXED_VIEW: &str = r#"
define i8 @mixed(ptr %p, i64 %i) {
entry:
  %q = getelementptr inbounds i16, ptr %p, i64 %i
  %a = load i16, ptr %q
  br label %done
done:
  %r = getelementptr inbounds i8, ptr %p, i64 %i
  %b = load i8, ptr %r
  %x = trunc i16 %a to i8
  %y = add i8 %x, %b
  ret i8 %y
}
"#;

/// A store through the parameter itself makes it mutable.
const STORE_DIRECT: &str = r#"
define i16 @store_direct(ptr %out, i16 %v) {
entry:
  store i16 %v, ptr %out
  br label %done
done:
  ret i16 %v
}
"#;

#[test]
fn opaque_pointer_param_pointee_follows_gep_element_type() {
    let f = lowers_ok(U16_COUNT, "count_ge_u16");
    match &f.params[0].ty {
        sir_types::Type::Pointer { pointee, mutable } => {
            assert_eq!(
                **pointee,
                sir_types::Type::u16(),
                "the buffer parameter must be *u16, not the opaque-pointer byte default"
            );
            assert!(!mutable, "read-only buffer stays const");
        }
        other => panic!("expected pointer param, got {other:?}"),
    }
}

#[test]
fn mixed_element_views_keep_the_byte_pointer() {
    let f = lowers_ok(MIXED_VIEW, "mixed");
    match &f.params[0].ty {
        sir_types::Type::Pointer { pointee, mutable } => {
            assert_eq!(
                **pointee,
                sir_types::Type::u8(),
                "conflicting element views must fall back to the byte view"
            );
            assert!(!mutable, "a read-only mixed view stays const");
        }
        other => panic!("expected pointer param, got {other:?}"),
    }
}

#[test]
fn store_through_the_parameter_marks_it_mutable() {
    let f = lowers_ok(STORE_DIRECT, "store_direct");
    match &f.params[0].ty {
        sir_types::Type::Pointer { pointee, mutable } => {
            assert_eq!(**pointee, sir_types::Type::u16());
            assert!(*mutable, "a store through the pointer marks it mutable");
        }
        other => panic!("expected pointer param, got {other:?}"),
    }
}

/// H3 F8: `for (i = 0; i < n; i += 2)` (guarded successor-tested
/// rotation) must lower to the pre-checked `Lt(carry, n)` domain, not
/// the successor test `Lt(carry + 2, n)` — under pre-tested SIR loop
/// semantics the successor form skips the first iteration.
const STRIDE2_SCAN: &str = r#"
define i64 @stride2(ptr nocapture noundef readonly %0, i64 noundef %1) {
entry:
  %g = icmp eq i64 %1, 0
  br i1 %g, label %exit0, label %loop
exit0:
  ret i64 0
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %s = phi i64 [ 0, %entry ], [ %s2, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %e = load i8, ptr %p
  %z = zext i8 %e to i64
  %s2 = add i64 %s, %z
  %i2 = add i64 %i, 2
  %c = icmp ult i64 %i2, %1
  br i1 %c, label %loop, label %exit
exit:
  %r = phi i64 [ %s2, %loop ]
  ret i64 %r
}
"#;

#[test]
fn successor_tested_stride_loop_rebuilds_the_carry_domain() {
    let f = lowers_ok(STRIDE2_SCAN, "stride2");
    let loop_node = f
        .arena
        .iter()
        .find(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }))
        .expect("a Loop node");
    let sir_nodes::NodeKind::Loop {
        termination,
        carried_inputs,
        ..
    } = &loop_node.kind
    else {
        unreachable!()
    };
    let term = f.get_node(*termination).expect("termination node");
    match &term.kind {
        sir_nodes::NodeKind::Lt { lhs, .. } => assert!(
            carried_inputs.contains(lhs),
            "termination must compare the carried counter itself, not its successor"
        ),
        other => panic!("expected Lt(carry, bound), got {other:?}"),
    }
}

/// Without the pre-loop `n == 0` exit there is no way to represent a
/// source do-while under pre-tested SIR loops: refuse loudly.
const UNGUARDED_DO_WHILE: &str = r#"
define i64 @unguarded(ptr %0, i64 %1) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %s = phi i64 [ 0, %entry ], [ %s2, %loop ]
  %p = getelementptr inbounds i8, ptr %0, i64 %i
  %e = load i8, ptr %p
  %z = zext i8 %e to i64
  %s2 = add i64 %s, %z
  %i2 = add i64 %i, 1
  %c = icmp ult i64 %i2, %1
  br i1 %c, label %loop, label %exit
exit:
  ret i64 %s2
}
"#;

#[test]
fn unguarded_successor_do_while_is_refused() {
    refuses_cleanly(UNGUARDED_DO_WHILE, "unguarded", "unguarded successor-tested loop");
}
