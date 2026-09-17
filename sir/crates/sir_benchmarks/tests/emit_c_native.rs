//! Native execution regressions for the SIR → C loop emitter (H3 F6–F8).
//!
//! Each fixture is lowered, emitted as C, compiled with clang and executed;
//! the printed values are compared against hand-computed semantics. These
//! tests fail (rather than skip) only when clang is unavailable.
//!
//! Findings pinned here:
//!   F6 post-loop emission order / undeclared loop-termination node,
//!   F7 buffer element-width typing (opaque `ptr` → `*u16`, not `*u8`),
//!   F8 loop-control polarity/entry-guard (pre-tested SIR loops; the
//!      reconstructed `Lt(carry, bound)` domain, not the successor test).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn clang_available() -> bool {
    Command::new("clang")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Lower + emit + compile + run; returns the driver's stdout.
fn native_stdout(ir: &str, func: &str, main_body: &str) -> Option<String> {
    if !clang_available() {
        eprintln!("clang unavailable — skipping native emitter test");
        return None;
    }
    let lowered = sir_lower::lower_function(ir, func)
        .unwrap_or_else(|e| panic!("{func} must lower: {e}"));
    let emitted = sir_benchmarks::emit::emit_c(&lowered);
    assert!(
        !emitted.trim().is_empty(),
        "emitter produced no C for {func}"
    );

    let dir: PathBuf = std::env::temp_dir().join(format!(
        "emit_c_native_{}_{}",
        std::process::id(),
        func
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    let driver = format!(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n\n{emitted}\n\
         int main(void) {{\n{main_body}\n    return 0;\n}}\n"
    );
    let driver_path = dir.join("driver.c");
    let exe_path = dir.join("exe");
    fs::write(&driver_path, driver).expect("write driver");

    let compile = Command::new("clang")
        .args([
            "-O1",
            "-w",
            "-o",
            exe_path.to_str().unwrap(),
            driver_path.to_str().unwrap(),
        ])
        .output()
        .expect("clang spawn");
    assert!(
        compile.status.success(),
        "emitted C must compile:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&exe_path).output().expect("run");
    assert!(
        run.status.success(),
        "driver failed: status {:?}",
        run.status.code()
    );
    Some(String::from_utf8_lossy(&run.stdout).to_string())
}

fn check_lines(stdout: Option<String>, expected: &[&str]) {
    let Some(stdout) = stdout else { return };
    let got: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        got, expected,
        "native execution mismatch:\nstdout: {stdout:?}"
    );
}

/// Lower + optimize + emit + compile + run (rewrite path included).
fn native_stdout_optimized(
    ir: &str,
    func: &str,
    main_body: &str,
) -> Option<(String, usize)> {
    if !clang_available() {
        eprintln!("clang unavailable — skipping native emitter test");
        return None;
    }
    let lowered = sir_lower::lower_function(ir, func)
        .unwrap_or_else(|e| panic!("{func} must lower: {e}"));
    let optimized = {
        let dev_null = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("/dev/null");
        let null_fd = std::os::fd::AsRawFd::as_raw_fd(&dev_null);
        let saved_fd = unsafe { libc::dup(1) };
        unsafe { libc::dup2(null_fd, 1); }
        let result = sir_optimizer::Optimizer::new(
            sir_optimizer::OptimizerConfig::default(),
            sir_rewrite::registry::default_registry(),
        )
        .optimize(&lowered);
        unsafe {
            libc::dup2(saved_fd, 1);
            libc::close(saved_fd);
        }
        result
    };
    assert!(
        optimized.rewrites_applied > 0,
        "{func} must exercise the rewrite path"
    );
    let emitted = sir_benchmarks::emit::emit_c(&optimized.function);

    let dir: PathBuf = std::env::temp_dir().join(format!(
        "emit_c_native_rw_{}_{}",
        std::process::id(),
        func
    ));
    fs::create_dir_all(&dir).expect("temp dir");
    let driver = format!(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n\n{emitted}\n\
         int main(void) {{\n{main_body}\n    return 0;\n}}\n"
    );
    let driver_path = dir.join("driver.c");
    let exe_path = dir.join("exe");
    fs::write(&driver_path, driver).expect("write driver");
    let compile = Command::new("clang")
        .args([
            "-O1",
            "-w",
            "-o",
            exe_path.to_str().unwrap(),
            driver_path.to_str().unwrap(),
        ])
        .output()
        .expect("clang spawn");
    assert!(
        compile.status.success(),
        "rewritten emitted C must compile:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new(&exe_path).output().expect("run");
    assert!(run.status.success(), "driver failed");
    Some((
        String::from_utf8_lossy(&run.stdout).to_string(),
        optimized.rewrites_applied,
    ))
}

/// u16 buffer + entry guard + counted rotation (p01 shape).
const U16_COUNT: &str = r#"
define i64 @count_ge_u16(ptr nocapture noundef readonly %0, i64 noundef %1, i16 noundef zeroext %2) {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6
4:
  %5 = phi i64 [ 0, %2 ], [ %13, %6 ]
  ret i64 %5
6:
  %7 = phi i64 [ %14, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i16, ptr %0, i64 %7
  %10 = load i16, ptr %9, align 2
  %11 = icmp uge i16 %10, %2
  %12 = zext i1 %11 to i64
  %13 = add i64 %8, %12
  %14 = add nuw i64 %7, 1
  %15 = icmp eq i64 %14, %1
  br i1 %15, label %4, label %6
}
"#;

#[test]
fn u16_count_emits_pretested_typed_loop() {
    let main = r#"
    static const uint16_t buf[8] = {256, 1, 9, 2, 7, 0, 300, 8};
    printf("%llu\n", (unsigned long long)count_ge_u16(buf, 8, 7));
    printf("%llu\n", (unsigned long long)count_ge_u16(buf, 0, 7));
    printf("%llu\n", (unsigned long long)count_ge_u16(buf, 1, 7));
"#;
    // n=8: 256,9,7,300,8 ≥ 7 → 5. n=0 must not read the buffer (F8).
    // n=1 would be 1 for the u16 element but 0 through the old *u8 view (F7).
    check_lines(native_stdout(U16_COUNT, "count_ge_u16", main), &["5", "0", "1"]);
}

/// Stride-2 pre-tested loop (n06 shape): `for (i = 0; i < n; i += 2)`.
const STRIDE2: &str = r#"
define i64 @stride2(ptr nocapture noundef readonly %0, i64 noundef %1) {
  %3 = icmp eq i64 %1, 0
  br i1 %3, label %4, label %6
4:
  %5 = phi i64 [ 0, %2 ], [ %12, %6 ]
  ret i64 %5
6:
  %7 = phi i64 [ %13, %6 ], [ 0, %2 ]
  %8 = phi i64 [ %12, %6 ], [ 0, %2 ]
  %9 = getelementptr inbounds i8, ptr %0, i64 %7
  %10 = load i8, ptr %9, align 1
  %11 = zext i8 %10 to i64
  %12 = add i64 %8, %11
  %13 = add nuw i64 %7, 2
  %14 = icmp ult i64 %13, %1
  br i1 %14, label %6, label %4
}
"#;

#[test]
fn stride2_loop_is_pretested_and_uses_the_carry_domain() {
    let main = r#"
    static const uint8_t buf[8] = {1, 2, 3, 4, 5, 6, 7, 8};
    printf("%llu\n", (unsigned long long)stride2(buf, 5));
    printf("%llu\n", (unsigned long long)stride2(buf, 8));
    printf("%llu\n", (unsigned long long)stride2(buf, 0));
"#;
    // 1+3+5=9; 1+3+5+7=16; n=0 must not read the buffer.
    check_lines(native_stdout(STRIDE2, "stride2", main), &["9", "16", "0"]);
}

/// Runtime stride (n07 shape): the successor test must be rebuilt onto
/// the carry domain or the last element is dropped.
const DYNAMIC_STRIDE: &str = r#"
define i64 @dynamic_stride(ptr nocapture noundef readonly %0, i64 noundef %1, i64 noundef %2) {
entry:
  %4 = icmp eq i64 %1, 0
  br i1 %4, label %5, label %7
5:
  %6 = phi i64 [ 0, %entry ], [ %13, %7 ]
  ret i64 %6
7:
  %8 = phi i64 [ %14, %7 ], [ 0, %entry ]
  %9 = phi i64 [ %13, %7 ], [ 0, %entry ]
  %10 = getelementptr inbounds i8, ptr %0, i64 %8
  %11 = load i8, ptr %10, align 1
  %12 = zext i8 %11 to i64
  %13 = add i64 %9, %12
  %14 = add i64 %8, %2
  %15 = icmp ult i64 %14, %1
  br i1 %15, label %7, label %5
}
"#;

#[test]
fn dynamic_stride_keeps_the_final_element() {
    let main = r#"
    static const uint8_t buf[6] = {10, 20, 30, 40, 50, 60};
    printf("%llu\n", (unsigned long long)dynamic_stride(buf, 6, 4));
    printf("%llu\n", (unsigned long long)dynamic_stride(buf, 5, 3));
    printf("%llu\n", (unsigned long long)dynamic_stride(buf, 0, 2));
"#;
    // step 4 over n=6: 10+50=60; step 3 over n=5: 10+40=50; n=0: 0.
    check_lines(
        native_stdout(DYNAMIC_STRIDE, "dynamic_stride", main),
        &["60", "50", "0"],
    );
}

/// h4c01 shape: `any` over a global array. The Any recipe rewrites the
/// loop to `ArrayCmpMask(...) != 0`; this pins native execution of that
/// committed rewrite (bitvector emission was unexercised before).
const ANY_GLOBAL: &str = r#"
@g_any = external local_unnamed_addr global [96 x i8], align 16

define dso_local range(i64 0, 2) i64 @any_or_global() {
  br label %4

1:
  %2 = icmp ne i8 %9, 0
  %3 = zext i1 %2 to i64
  ret i64 %3

4:
  %5 = phi i64 [ 0, %0 ], [ %10, %4 ]
  %6 = phi i8 [ 0, %0 ], [ %9, %4 ]
  %7 = getelementptr inbounds [96 x i8], ptr @g_any, i64 0, i64 %5
  %8 = load i8, ptr %7, align 1
  %9 = or i8 %8, %6
  %10 = add nuw nsw i64 %5, 1
  %11 = icmp eq i64 %10, 96
  br i1 %11, label %1, label %4
}
"#;

#[test]
fn rewritten_any_global_mask_runs_natively() {
    let main = r#"
    uint8_t zeros[96] = {0};
    uint8_t nonzero[96] = {0};
    nonzero[77] = 1;
    printf("%llu\n", (unsigned long long)any_or_global(zeros));
    printf("%llu\n", (unsigned long long)any_or_global(nonzero));
    printf("%llu\n", (unsigned long long)(nonzero[0] = 5, any_or_global(nonzero)));
"#;
    let Some((stdout, rewrites)) = native_stdout_optimized(ANY_GLOBAL, "any_or_global", main)
    else {
        return;
    };
    assert!(rewrites > 0, "the Any recipe must fire on this shape");
    check_lines(Some(stdout), &["0", "1", "1"]);
}
