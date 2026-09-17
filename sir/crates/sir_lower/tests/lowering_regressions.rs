//! Lowering regressions for the Gate 6A corpus findings.
//!
//! Every snippet is self-contained LLVM IR (no corpus dependency) so the
//! failures that batch_run surfaced — w07_all_min / h02_all_match /
//! n14_saturating_count (select literal arms typed as u64), w08/v07
//! (two loops half-lowered into a MissingReturn), x08/n03/n08 (loops with
//! a separate latch block dying on raw resolution errors) — are pinned by
//! `cargo test`.

use std::collections::HashSet;

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

/// A shared-exit shape the sequential composer must NOT accept: the
/// second loop's latch (`outer`) branches back into a block that is also
/// the first loop's exit, so the per-loop exit walk cannot stop at a
/// simple conditional guard. This must stay an explicit unsupported
/// refusal, never a half-lowered function without a Return (previously
/// caught only downstream as MissingReturn). The w08/p12 sequential
/// shapes DO lower — see `sequential_two_loops_lower_as_one_function`.
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

/// Sequential two-loop kernels lower WHOLE (re-landed 2026-09-17 once
/// the SIR→C emitter learned to compose sequential loops). The first
/// composition attempt emitted wrong native code and was reverted; the
/// emitter now namespaces carriers/outputs per loop and resolves
/// TupleExtract/FieldAccess to the producing loop, and the corpus
/// native differential gates this path.
#[test]
fn sequential_two_loops_lower_as_one_function() {
    let f = lower_function(TWO_LOOP_COUNT_SUM, "two")
        .expect("sequential two-loop function must lower");
    let loop_count = f
        .arena
        .iter()
        .filter(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }))
        .count();
    assert_eq!(loop_count, 2, "both self-latching loops must be present");
    assert!(
        f.return_node.is_some(),
        "the shared exit/return must be emitted exactly once"
    );
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

// ── Constant-extent buffer promotion (dynamic-extent recall slice) ──

/// Constant bound 96: every access is `b[i]` for the strict counted
/// loop `i = 0 .. 96`, so the pointer may be viewed as `[u8; 96]`.
const CONST_BOUND_POINTER: &str = r#"
define i64 @const_bound(ptr %b, i8 %key) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %acc2, %loop ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %hit = icmp eq i8 %e, %key
  %z = zext i1 %hit to i64
  %acc2 = add i64 %acc, %z
  %i2 = add i64 %i, 1
  %c = icmp eq i64 %i2, 96
  br i1 %c, label %exit, label %loop
exit:
  ret i64 %acc2
}
"#;

/// Runtime bound: the extent is not a constant, so no promotion.
const RUNTIME_BOUND_POINTER: &str = r#"
define i64 @runtime_bound(ptr %b, i64 %n) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %acc2, %loop ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %z = zext i8 %e to i64
  %acc2 = add i64 %acc, %z
  %i2 = add i64 %i, 1
  %c = icmp eq i64 %i2, %n
  br i1 %c, label %exit, label %loop
exit:
  ret i64 %acc2
}
"#;

/// A second access on a parameter index has no proven extent: the
/// whole parameter must stay a pointer (no fabricated extent).
const UNCOVERED_ACCESS_POINTER: &str = r#"
define i64 @uncovered(ptr %b, i64 %j) {
entry:
  br label %loop
loop:
  %i = phi i64 [ 0, %entry ], [ %i2, %loop ]
  %acc = phi i64 [ 0, %entry ], [ %acc2, %loop ]
  %p = getelementptr inbounds i8, ptr %b, i64 %i
  %e = load i8, ptr %p
  %q = getelementptr inbounds i8, ptr %b, i64 %j
  %e2 = load i8, ptr %q
  %z1 = zext i8 %e to i64
  %z2 = zext i8 %e2 to i64
  %s = add i64 %z1, %z2
  %acc2 = add i64 %acc, %s
  %i2 = add i64 %i, 1
  %c = icmp eq i64 %i2, 96
  br i1 %c, label %exit, label %loop
exit:
  ret i64 %acc2
}
"#;

fn param_type(func: &sir_nodes::Function, index: usize) -> sir_types::Type {
    func.params[index].ty.clone()
}

#[test]
fn constant_bound_pointer_is_promoted_to_an_array_view() {
    let func = lower_function(CONST_BOUND_POINTER, "const_bound")
        .expect("constant-bound pointer loop must lower");
    assert_eq!(
        param_type(&func, 0),
        sir_types::Type::Array {
            element: Box::new(sir_types::Type::u8()),
            length: 96
        },
        "a fully-proven constant extent must promote the pointer view"
    );
}

#[test]
fn runtime_bound_pointer_is_not_promoted() {
    let func = lower_function(RUNTIME_BOUND_POINTER, "runtime_bound")
        .expect("runtime-bound pointer loop must lower");
    assert!(
        matches!(param_type(&func, 0), sir_types::Type::Pointer { .. }),
        "a runtime extent must never be fabricated into an array view"
    );
}

#[test]
fn uncovered_pointer_access_blocks_promotion() {
    let func = lower_function(UNCOVERED_ACCESS_POINTER, "uncovered")
        .expect("loop with an uncovered access must still lower");
    assert!(
        matches!(param_type(&func, 0), sir_types::Type::Pointer { .. }),
        "one uncovered access must keep the whole parameter a pointer"
    );
}

// ── Unguarded post-tested (do-while) carry-domain reconstruction ──

/// clang's stride-4 constant-bound form (v6 `n06_stride4_const`): a
/// self-loop with the test on the carry and a back-edge on true. The
/// body runs FIRST and the loop continues while `i < 60`, so the
/// executed values are 0,4,…,60 — the pre-tested SIR domain is
/// `i < 64`, not `i < 60`. Lowering the test as-is dropped the forced
/// final iteration (native differential: 48/48 mismatches).
const POST_TESTED_STRIDE4: &str = r#"
define i64 @stride4(ptr %b) {
entry:
  br label %3

2:
  ret i64 %9

3:
  %4 = phi i64 [ 0, %entry ], [ %10, %3 ]
  %5 = phi i64 [ 0, %entry ], [ %9, %3 ]
  %6 = getelementptr inbounds i8, ptr %b, i64 %4
  %7 = load i8, ptr %6
  %8 = zext i8 %7 to i64
  %9 = add i64 %5, %8
  %10 = add nuw nsw i64 %4, 4
  %11 = icmp ult i64 %4, 60
  br i1 %11, label %3, label %2
}
"#;

#[test]
fn post_tested_stride_loop_reconstructs_the_forced_iteration() {
    let func = lower_function(POST_TESTED_STRIDE4, "stride4")
        .expect("constant-step post-tested loop must lower");
    let loop_node = func
        .arena
        .iter()
        .find(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }))
        .expect("loop node");
    let sir_nodes::NodeKind::Loop { termination, .. } = &loop_node.kind else {
        unreachable!()
    };
    let term = func.get_node(*termination).expect("termination");
    match &term.kind {
        sir_nodes::NodeKind::Lt { rhs, .. } => {
            let bound = func.get_node(*rhs).expect("bound");
            match &bound.kind {
                sir_nodes::NodeKind::Constant(data) => assert_eq!(
                    data.as_u64(),
                    Some(64),
                    "the do-while `i < 60` stride-4 domain must reconstruct as `i < 64`"
                ),
                other => panic!("expected a constant bound, got {other:?}"),
            }
        }
        other => panic!("expected a reconstructed Lt termination, got {other:?}"),
    }
}

/// A post-tested loop whose step is not a positive constant cannot be
/// reconstructed; it must be refused loudly (never silently lowered as
/// pre-tested).
#[test]
fn post_tested_loop_with_runtime_step_is_refused() {
    let ir = POST_TESTED_STRIDE4.replace(
        "%10 = add nuw nsw i64 %4, 4",
        "%10 = add nuw nsw i64 %4, %step",
    );
    // Add the step parameter to the signature so the IR stays valid.
    let ir = ir.replace("define i64 @stride4(ptr %b) {", "define i64 @stride4(ptr %b, i64 %step) {");
    match lower_function(&ir, "stride4") {
        Ok(_) => panic!("a runtime-step post-tested loop must be refused"),
        Err(e) => assert!(
            e.contains("unguarded post-tested loop") || e.contains("unsupported"),
            "unexpected refusal: {e}"
        ),
    }
}

// ── Multi-loop whole-function promotion / candidate enabler ──

/// Constant-bound two-loop kernel (v6 `p08_two_loops_const` shape):
/// loop1 counts `buf[i] == key` to 48, loop2 sums `buf[i]` to 48, and
/// the return combines both. Both accesses are inside a strict counted
/// loop, so the buffer must promote to `[u8; 48]`. Regression: the
/// sequential composer used to re-emit loop1's body as straight-line
/// code while lowering loop2 — that duplicate `ArrayAccess` outside any
/// loop blocked the promotion, so the whole-function regions produced
/// no contexts and no candidates.
const TWO_CONST_LOOPS: &str = r#"
define i64 @two_const(ptr %0, i8 %1) {
  br label %3

3:
  %4 = phi i64 [ 0, %2 ], [ %11, %3 ]
  %5 = phi i64 [ 0, %2 ], [ %10, %3 ]
  %6 = getelementptr inbounds i8, ptr %0, i64 %4
  %7 = load i8, ptr %6
  %8 = icmp eq i8 %7, %1
  %9 = zext i1 %8 to i64
  %10 = add i64 %5, %9
  %11 = add nuw nsw i64 %4, 1
  %12 = icmp eq i64 %11, 48
  br i1 %12, label %15, label %3

13:
  %14 = xor i64 %21, %10
  ret i64 %14

15:
  %16 = phi i64 [ %22, %15 ], [ 0, %3 ]
  %17 = phi i64 [ %21, %15 ], [ 0, %3 ]
  %18 = getelementptr inbounds i8, ptr %0, i64 %16
  %19 = load i8, ptr %18
  %20 = zext i8 %19 to i64
  %21 = add i64 %17, %20
  %22 = add nuw nsw i64 %16, 1
  %23 = icmp eq i64 %22, 48
  br i1 %23, label %13, label %15
}
"#;

#[test]
fn constant_two_loops_promote_and_do_not_duplicate_the_body() {
    let func = lower_function(TWO_CONST_LOOPS, "two_const")
        .expect("constant two-loop function must lower");
    assert_eq!(
        func.params[0].ty,
        sir_types::Type::Array {
            element: Box::new(sir_types::Type::u8()),
            length: 48
        },
        "both loop accesses are inside proven constant extents, so the \
         buffer must promote (the old composer's duplicate blocked this)"
    );
    let loop_bodies: HashSet<sir_types::NodeId> = func
        .arena
        .iter()
        .filter_map(|n| match &n.kind {
            sir_nodes::NodeKind::Loop { body, .. } => Some(body.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect();
    let outside_accesses = func
        .arena
        .iter()
        .filter(|n| matches!(n.kind, sir_nodes::NodeKind::ArrayAccess { .. }))
        .filter(|n| !loop_bodies.contains(&n.id))
        .count();
    assert!(
        outside_accesses <= 1,
        "at most the exit-walk's constant-index access may sit outside a \
         loop; the composer must not re-emit loop1's body (got {outside_accesses})"
    );
}

/// clang's early-return search CFG (header/latch/merge) must lower into
/// the canonical found-flag SIR loop instead of being refused, and the
/// buffer must promote (the counter bound is a conjunct of the
/// termination `!found && i < 48`).
const EARLY_EXIT_SEARCH: &str = r#"
define i64 @first_set(ptr %0) {
  br label %2

2:
  %3 = phi i64 [ 0, %1 ], [ %8, %7 ]
  %4 = getelementptr inbounds i8, ptr %0, i64 %3
  %5 = load i8, ptr %4
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %7, label %10

7:
  %8 = add nuw nsw i64 %3, 1
  %9 = icmp eq i64 %8, 48
  br i1 %9, label %10, label %2

10:
  %11 = phi i64 [ 48, %7 ], [ %3, %2 ]
  ret i64 %11
}
"#;

#[test]
fn early_exit_search_lowers_to_a_found_flag_loop() {
    let func = lower_function(EARLY_EXIT_SEARCH, "first_set")
        .expect("the header/latch/merge search CFG must lower");
    assert_eq!(
        func.params[0].ty,
        sir_types::Type::Array {
            element: Box::new(sir_types::Type::u8()),
            length: 48
        },
        "the counter bound is a termination conjunct; promotion must see it"
    );
    let loops = func
        .arena
        .iter()
        .filter(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }))
        .count();
    assert_eq!(loops, 1, "the early exit is modeled inside one SIR loop");
    assert!(func.return_node.is_some(), "the index must be returned");
}
