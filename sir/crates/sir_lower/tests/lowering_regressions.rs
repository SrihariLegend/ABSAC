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
