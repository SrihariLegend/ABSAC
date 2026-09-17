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

use sir_builder::Builder;
use sir_types::{Span, Type};

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
    compile_and_run_emitted(&emitted, main_body, func)
}

/// Compile an already-emitted function together with a driver, run it,
/// and return the driver's stdout.
fn compile_and_run_emitted(emitted: &str, main_body: &str, tag: &str) -> Option<String> {
    if !clang_available() {
        eprintln!("clang unavailable — skipping native emitter test");
        return None;
    }
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "emit_c_native_{}_{}",
        std::process::id(),
        tag
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
    let optimized = optimize_suppressed(&lowered);
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

/// Instruction-selection intrinsics must execute as their semantic
/// operations, not as the emitter's old silent `0`.
#[test]
fn instruction_selection_intrinsics_execute_natively() {
    let cases: Vec<(&str, Vec<u32>, Vec<u32>)> = vec![
        (
            "blsr",
            vec![0x0C, 0, 0xFFFF_FFFF],
            vec![0x08, 0, 0xFFFF_FFFE],
        ),
        ("blsi", vec![0x0C, 0, 0x8000_0000], vec![0x04, 0, 0x8000_0000]),
        (
            "blsmsk",
            vec![0x0C, 0, 0xFFFF_FFFF],
            // x ^ (x - 1): 0 gives all-ones; all-ones gives 1.
            vec![0x07, 0xFFFF_FFFF, 0x01],
        ),
        ("bswap", vec![0x1122_3344], vec![0x4433_2211]),
        ("rbit", vec![0x0000_000B], vec![0xD000_0000]),
    ];
    for (intrinsic, inputs, expected) in cases {
        let mut b = Builder::new(intrinsic, &[("x", Type::u32())], Type::u32());
        let x = b.parameter_index(0).unwrap();
        let r = b
            .intrinsic(
                intrinsic,
                &[x],
                Type::u32(),
                sir_types::Effects::empty(),
                Span::unknown(),
            )
            .unwrap();
        b.return_value(r, Span::unknown()).unwrap();
        let emitted = sir_benchmarks::emit::emit_c(&b.build());
        let mut main = String::new();
        for input in &inputs {
            main.push_str(&format!(
                "    printf(\"%llu\\n\", (unsigned long long){intrinsic}(0x{input:08X}u));\n"
            ));
        }
        let Some(stdout) = compile_and_run_emitted(&emitted, &main, intrinsic) else {
            return;
        };
        let got: Vec<String> = stdout.lines().map(str::to_string).collect();
        let want: Vec<String> = expected.iter().map(u32::to_string).collect();
        assert_eq!(got, want, "{intrinsic} native results");
    }
}

fn rotate_function(name: &str, left: bool) -> sir_nodes::Function {
    let mut b = Builder::new(
        name,
        &[("x", Type::u32()), ("k", Type::u32())],
        Type::u32(),
    );
    let x = b.parameter_index(0).unwrap();
    let k = b.parameter_index(1).unwrap();
    let r = if left {
        b.rol(x, k, Span::unknown()).unwrap()
    } else {
        b.ror(x, k, Span::unknown()).unwrap()
    };
    b.return_value(r, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rotates_execute_natively_with_width_relative_amounts() {
    for (name, left, calls, expected) in [
        (
            "rotl",
            true,
            vec![(0x8000_0001u32, 1u32), (0x1234_5678, 8), (0xABCD_1234, 0), (0x8000_0001, 32)],
            vec![0x0000_0003u32, 0x3456_7812, 0xABCD_1234, 0x8000_0001],
        ),
        (
            "rotr",
            false,
            vec![(0x0000_0003u32, 1u32), (0x1234_5678, 8), (0xABCD_1234, 0), (0x0000_0003, 32)],
            vec![0x8000_0001u32, 0x7812_3456, 0xABCD_1234, 0x0000_0003],
        ),
    ] {
        let emitted = sir_benchmarks::emit::emit_c(&rotate_function(name, left));
        let mut main = String::new();
        for (x, k) in &calls {
            main.push_str(&format!(
                "    printf(\"%llu\\n\", (unsigned long long){name}(0x{x:08X}u, {k}u));\n"
            ));
        }
        let Some(stdout) = compile_and_run_emitted(&emitted, &main, name) else {
            return;
        };
        let got: Vec<String> = stdout.lines().map(str::to_string).collect();
        let want: Vec<String> = expected.iter().map(u32::to_string).collect();
        assert_eq!(got, want, "{name} native results");
    }
}

/// Serializes stdout redirection: libtest runs tests in parallel, and
/// two concurrent save/dup2/restore sequences can leave fd 1 pointing at
/// /dev/null for the rest of the process (swallowing later results).
static STDOUT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn optimize_suppressed(func: &sir_nodes::Function) -> sir_optimizer::OptimizationResult {
    let _guard = STDOUT_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dev_null = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("/dev/null");
    let null_fd = std::os::fd::AsRawFd::as_raw_fd(&dev_null);
    let saved_fd = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(null_fd, 1);
    }
    let result = sir_optimizer::Optimizer::new(
        sir_optimizer::OptimizerConfig::default(),
        sir_rewrite::registry::default_registry(),
    )
    .optimize(func);
    unsafe {
        libc::dup2(saved_fd, 1);
        libc::close(saved_fd);
    }
    result
}

/// Build one mask-algebra pattern as SIR (u64).
fn mask_pattern_function(name: &str) -> sir_nodes::Function {
    let ty = Type::u64();
    let mut b = Builder::new(name, &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let one = b.constant(sir_types::ConstantData::u64(1), ty.clone(), Span::unknown());
    let root = match name {
        "clear_bit" => {
            let sub = b.sub(x, one, Span::unknown()).unwrap();
            b.bit_and(x, sub, Span::unknown()).unwrap()
        }
        "isolate_bit" => {
            let neg = b.neg(x, Span::unknown()).unwrap();
            b.bit_and(x, neg, Span::unknown()).unwrap()
        }
        "isolate_clear_bit" => {
            let not = b.bit_not(x, Span::unknown()).unwrap();
            let add = b.add(x, one, Span::unknown()).unwrap();
            b.bit_and(not, add, Span::unknown()).unwrap()
        }
        "set_clear_bit" => {
            let add = b.add(x, one, Span::unknown()).unwrap();
            b.bit_or(x, add, Span::unknown()).unwrap()
        }
        other => panic!("unknown pattern {other}"),
    };
    b.return_value(root, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rewritten_mask_algebra_executes_natively() {
    let cases: [(&str, &str, &[&str]); 4] = [
        (
            "clear_bit",
            "    printf(\"%llu\\n\", (unsigned long long)clear_bit(0x0CULL));\n\
             \x20   printf(\"%llu\\n\", (unsigned long long)clear_bit(0ULL));\n\
             \x20   printf(\"%llu\\n\", (unsigned long long)clear_bit(0xF0ULL));\n",
            &["8", "0", "224"],
        ),
        (
            "isolate_bit",
            "    printf(\"%llu\\n\", (unsigned long long)isolate_bit(0x0CULL));\n\
             \x20   printf(\"%llu\\n\", (unsigned long long)isolate_bit(0ULL));\n\
             \x20   printf(\"%llu\\n\", (unsigned long long)isolate_bit(0x8000000000000000ULL));\n",
            &["4", "0", "9223372036854775808"],
        ),
        (
            "isolate_clear_bit",
            "    printf(\"%llu\\n\", (unsigned long long)isolate_clear_bit(0ULL));\n\
             \x20   printf(\"%llu\\n\", (unsigned long long)isolate_clear_bit(0xBULL));\n",
            &["1", "4"],
        ),
        (
            "set_clear_bit",
            "    printf(\"%llu\\n\", (unsigned long long)set_clear_bit(0ULL));\n\
             \x20   printf(\"%llu\\n\", (unsigned long long)set_clear_bit(0xBULL));\n",
            &["1", "15"],
        ),
    ];
    for (name, main, expected) in cases {
        if !clang_available() {
            return;
        }
        let func = mask_pattern_function(name);
        let optimized = optimize_suppressed(&func);
        assert_eq!(
            optimized.rewrites_applied, 1,
            "{name}: the mask-algebra rewrite must be authorized"
        );
        let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
        let Some(stdout) = compile_and_run_emitted(&emitted, main, &format!("mask_{name}")) else {
            return;
        };
        let got: Vec<&str> = stdout.lines().collect();
        assert_eq!(got, expected, "{name}: rewritten native results");
    }
}

/// HD004 shape: swap the low two bytes of a u32 via masks and shifts.
fn byte_swap_16_function() -> sir_nodes::Function {
    let ty = Type::u32();
    let mut b = Builder::new("byte_swap_16", &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let mask = b.constant(sir_types::ConstantData::u32(0xFF), ty.clone(), Span::unknown());
    let eight = b.constant(sir_types::ConstantData::u32(8), ty.clone(), Span::unknown());
    let low = b.bit_and(x, mask, Span::unknown()).unwrap();
    let low_shifted = b.shl(low, eight, Span::unknown()).unwrap();
    let high = b.shr(x, eight, Span::unknown()).unwrap();
    let high_masked = b.bit_and(high, mask, Span::unknown()).unwrap();
    let res = b.bit_or(low_shifted, high_masked, Span::unknown()).unwrap();
    b.return_value(res, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rewritten_byte_swap_executes_natively() {
    if !clang_available() {
        return;
    }
    let optimized = optimize_suppressed(&byte_swap_16_function());
    assert_eq!(
        optimized.rewrites_applied, 1,
        "the byte-swap rewrite must be authorized"
    );
    let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
    assert!(
        emitted.contains("__builtin_bswap32"),
        "the rewrite must select the bswap intrinsic:\n{emitted}"
    );
    let main = "    printf(\"%llu\\n\", (unsigned long long)byte_swap_16(0x11223344u));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)byte_swap_16(0xAABBu));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)byte_swap_16(0xFFu));\n";
    let Some(stdout) = compile_and_run_emitted(&emitted, main, "byte_swap_16") else {
        return;
    };
    // Low two bytes swapped, high bytes dropped:
    // 0x3344 -> 0x4433 (17459), 0xAABB -> 0xBBAA (48042), 0xFF -> 0xFF00 (65280).
    let got: Vec<&str> = stdout.lines().collect();
    assert_eq!(got, vec!["17459", "48042", "65280"]);
}

/// HD005 shape: reverse the low 8 bits of a u32 with a three-stage swap
/// network.
fn reverse_bits_8_function() -> sir_nodes::Function {
    let ty = Type::u32();
    let mut b = Builder::new("reverse_bits_8", &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let c = |v: u32| sir_types::ConstantData::u32(v);
    let m1 = b.constant(c(0x55), ty.clone(), Span::unknown());
    let m2 = b.constant(c(0x33), ty.clone(), Span::unknown());
    let m3 = b.constant(c(0x0F), ty.clone(), Span::unknown());
    let one = b.constant(c(1), ty.clone(), Span::unknown());
    let two = b.constant(c(2), ty.clone(), Span::unknown());
    let four = b.constant(c(4), ty.clone(), Span::unknown());
    let stage = |b: &mut Builder, x, mask, amount| {
        let shr = b.shr(x, amount, Span::unknown()).unwrap();
        let low = b.bit_and(shr, mask, Span::unknown()).unwrap();
        let and = b.bit_and(x, mask, Span::unknown()).unwrap();
        let shl = b.shl(and, amount, Span::unknown()).unwrap();
        b.bit_or(low, shl, Span::unknown()).unwrap()
    };
    let x1 = stage(&mut b, x, m1, one);
    let x2 = stage(&mut b, x1, m2, two);
    let res = stage(&mut b, x2, m3, four);
    b.return_value(res, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rewritten_bit_reverse_executes_natively() {
    if !clang_available() {
        return;
    }
    let optimized = optimize_suppressed(&reverse_bits_8_function());
    assert_eq!(
        optimized.rewrites_applied, 1,
        "the bit-reverse rewrite must be authorized"
    );
    let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
    assert!(
        emitted.contains("__sir_rbit"),
        "the rewrite must select the rbit intrinsic:\n{emitted}"
    );
    let main = "    printf(\"%llu\\n\", (unsigned long long)reverse_bits_8(0x01u));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)reverse_bits_8(0x0Fu));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)reverse_bits_8(0xA5u));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)reverse_bits_8(0u));\n";
    let Some(stdout) = compile_and_run_emitted(&emitted, main, "reverse_bits_8") else {
        return;
    };
    // reverse8(0x01)=0x80=128, reverse8(0x0F)=0xF0=240,
    // reverse8(0xA5)=0xA5=165 (palindrome), 0 -> 0.
    let got: Vec<&str> = stdout.lines().collect();
    assert_eq!(got, vec!["128", "240", "165", "0"]);
}

/// `(x << 4) >> 4` over u32 (ShiftMask shape).
fn shift_mask_function() -> sir_nodes::Function {
    let ty = Type::u32();
    let mut b = Builder::new("shift_mask_4", &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let four = b.constant(sir_types::ConstantData::u32(4), ty.clone(), Span::unknown());
    let shl = b.shl(x, four, Span::unknown()).unwrap();
    let shr = b.shr(shl, four, Span::unknown()).unwrap();
    b.return_value(shr, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rewritten_shift_mask_executes_natively() {
    if !clang_available() {
        return;
    }
    let optimized = optimize_suppressed(&shift_mask_function());
    assert_eq!(
        optimized.rewrites_applied, 1,
        "the shift-mask rewrite must be authorized"
    );
    let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
    let main = "    printf(\"%llu\\n\", (unsigned long long)shift_mask_4(0xFFFFFFFFu));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)shift_mask_4(0x12345678u));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)shift_mask_4(0xFu));\n";
    let Some(stdout) = compile_and_run_emitted(&emitted, main, "shift_mask_4") else {
        return;
    };
    // (x << 4) >> 4 == x & 0x0FFFFFFF:
    // 0xFFFFFFFF -> 0x0FFFFFFF (268435455), 0x12345678 -> 0x02345678
    // (36984440), 0xF -> 0xF (15).
    let got: Vec<&str> = stdout.lines().collect();
    assert_eq!(got, vec!["268435455", "36984440", "15"]);
}

fn zero_count_function(name: &str, leading: bool) -> sir_nodes::Function {
    let ty = Type::u64();
    let mut b = Builder::new(name, &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let r = if leading {
        b.leading_zeros(x, Span::unknown()).unwrap()
    } else {
        b.trailing_zeros(x, Span::unknown()).unwrap()
    };
    b.return_value(r, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn zero_count_conventions_execute_natively() {
    for (name, leading, calls, expected) in [
        (
            "tz",
            false,
            vec!["0ULL", "0x10ULL", "1ULL"],
            vec!["64", "4", "0"],
        ),
        (
            "lz",
            true,
            vec!["0ULL", "1ULL", "0x8000000000000000ULL"],
            vec!["64", "63", "0"],
        ),
    ] {
        if !clang_available() {
            return;
        }
        let emitted = sir_benchmarks::emit::emit_c(&zero_count_function(name, leading));
        let mut main = String::new();
        for input in &calls {
            main.push_str(&format!(
                "    printf(\"%llu\\n\", (unsigned long long){name}({input}));\n"
            ));
        }
        let Some(stdout) = compile_and_run_emitted(&emitted, &main, name) else {
            return;
        };
        let got: Vec<&str> = stdout.lines().collect();
        assert_eq!(got, expected, "{name} zero-count conventions");
    }
}

#[test]
fn rewritten_zero_count_loops_execute_natively() {
    for (name, leading, calls, expected) in [
        (
            "tz_loop",
            false,
            vec!["0ULL", "0x10ULL", "1ULL"],
            vec!["64", "4", "0"],
        ),
        (
            "lz_loop",
            true,
            vec!["0ULL", "1ULL", "0x8000000000000000ULL"],
            vec!["64", "63", "0"],
        ),
    ] {
        if !clang_available() {
            return;
        }
        let func = zero_count_loop_function(name, leading);
        let optimized = optimize_suppressed(&func);
        assert_eq!(
            optimized.rewrites_applied, 1,
            "{name}: the loop-to-intrinsic rewrite must be authorized"
        );
        let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
        let helper = if leading { "__sir_clz" } else { "__sir_ctz" };
        assert!(
            emitted.contains(helper),
            "{name}: expected {helper} in the emitted C:\n{emitted}"
        );
        let mut main = String::new();
        for input in &calls {
            main.push_str(&format!(
                "    printf(\"%llu\\n\", (unsigned long long){name}({input}));\n"
            ));
        }
        let Some(stdout) = compile_and_run_emitted(&emitted, &main, name) else {
            return;
        };
        let got: Vec<&str> = stdout.lines().collect();
        assert_eq!(got, expected, "{name} zero-count loop results");
    }
}

fn pow2_function(name: &str, constant: u32, divide: bool) -> sir_nodes::Function {
    let ty = Type::u32();
    let mut b = Builder::new(name, &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(sir_types::ConstantData::u32(constant), ty.clone(), Span::unknown());
    let node = if divide {
        b.div(x, c, Span::unknown()).unwrap()
    } else {
        b.rem(x, c, Span::unknown()).unwrap()
    };
    b.return_value(node, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rewritten_pow2_division_executes_natively() {
    for (name, constant, divide, calls, expected) in [
        (
            "mod16",
            16u32,
            false,
            vec!["0xFFFFFFFFu", "0x12345678u", "0xFu"],
            vec!["15", "8", "15"],
        ),
        (
            "div8",
            8u32,
            true,
            vec!["0xFFFFFFFFu", "0x12345678u", "0xFu"],
            vec!["536870911", "38177487", "1"],
        ),
    ] {
        if !clang_available() {
            return;
        }
        let optimized = optimize_suppressed(&pow2_function(name, constant, divide));
        assert_eq!(
            optimized.rewrites_applied, 1,
            "{name}: the power-of-two rewrite must be authorized"
        );
        let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
        let mut main = String::new();
        for input in &calls {
            main.push_str(&format!(
                "    printf(\"%llu\\n\", (unsigned long long){name}({input}));\n"
            ));
        }
        let Some(stdout) = compile_and_run_emitted(&emitted, &main, name) else {
            return;
        };
        let got: Vec<&str> = stdout.lines().collect();
        assert_eq!(got, expected, "{name} native results");
    }
}

/// PS001 found-flag forward search over a bool[64] array.
fn first_set_bit_function() -> sir_nodes::Function {
    let u64ty = Type::u64();
    let arr_ty = Type::Array {
        element: Box::new(Type::Bool),
        length: 64,
    };
    let mut b = Builder::new("array_find_first", &[("board", arr_ty)], u64ty.clone());
    let board = b.parameter_index(0).unwrap();
    let i_init = b.constant(sir_types::ConstantData::u64(0), u64ty.clone(), Span::unknown());
    let one = b.constant(sir_types::ConstantData::u64(1), u64ty.clone(), Span::unknown());
    let limit = b.constant(sir_types::ConstantData::u64(64), u64ty.clone(), Span::unknown());
    let found_init = b.constant(sir_types::ConstantData::Bool(false), Type::Bool, Span::unknown());
    let index_init = b.constant(sir_types::ConstantData::u64(64), u64ty.clone(), Span::unknown());
    let elem = b.array_access(board, i_init, Type::Bool, Span::unknown()).unwrap();
    let new_found = b.bool_or(found_init, elem, Span::unknown()).unwrap();
    let not_found_yet = b.bool_not(found_init, Span::unknown()).unwrap();
    let is_first = b.bool_and(elem, not_found_yet, Span::unknown()).unwrap();
    let new_index = b.select(is_first, i_init, index_init, Span::unknown()).unwrap();
    let i_next = b.add(i_init, one, Span::unknown()).unwrap();
    let not_found = b.bool_not(found_init, Span::unknown()).unwrap();
    let in_bounds = b.lt(i_init, limit, Span::unknown()).unwrap();
    let cond = b.bool_and(not_found, in_bounds, Span::unknown()).unwrap();
    let loop_node = b
        .r#loop(
            &[elem, new_found, not_found_yet, is_first, new_index, i_next, not_found, in_bounds, cond],
            cond,
            &[new_found, new_index, i_next],
            &[found_init, index_init, i_init],
            Type::Tuple {
                elements: vec![Type::Bool, u64ty.clone(), u64ty],
            },
            Span::unknown(),
        )
        .unwrap();
    let res = b.field_access(loop_node, "1", Type::u64(), Span::unknown()).unwrap();
    b.return_value(res, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rewritten_forward_bitscan_executes_natively() {
    if !clang_available() {
        return;
    }
    let optimized = optimize_suppressed(&first_set_bit_function());
    assert_eq!(
        optimized.rewrites_applied, 1,
        "the forward bitscan rewrite must be authorized"
    );
    let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
    assert!(
        emitted.contains("__sir_ctz"),
        "the rewrite must select TrailingZeros:\n{emitted}"
    );
    let main = "    bool board[64] = {0};\n\
                \x20   board[5] = true;\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)array_find_first(board));\n\
                \x20   bool empty[64] = {0};\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)array_find_first(empty));\n";
    let Some(stdout) = compile_and_run_emitted(&emitted, main, "array_find_first") else {
        return;
    };
    // First set bit at index 5; empty array returns the length sentinel 64.
    let got: Vec<&str> = stdout.lines().collect();
    assert_eq!(got, vec!["5", "64"]);
}

/// `(x << 3) | (x >> (32 - 3))` with constant amounts (rotate left).
fn rotate_left_const_function() -> sir_nodes::Function {
    let ty = Type::u32();
    let mut b = Builder::new("rotl3", &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let three = b.constant(sir_types::ConstantData::u32(3), ty.clone(), Span::unknown());
    let width = b.constant(sir_types::ConstantData::u32(32), ty.clone(), Span::unknown());
    let diff = b.sub(width, three, Span::unknown()).unwrap();
    let shl = b.shl(x, three, Span::unknown()).unwrap();
    let shr = b.shr(x, diff, Span::unknown()).unwrap();
    let res = b.bit_or(shl, shr, Span::unknown()).unwrap();
    b.return_value(res, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn rewritten_constant_rotate_executes_natively() {
    if !clang_available() {
        return;
    }
    let optimized = optimize_suppressed(&rotate_left_const_function());
    assert_eq!(
        optimized.rewrites_applied, 1,
        "a constant-amount rotate must be authorized by the concrete proof"
    );
    let emitted = sir_benchmarks::emit::emit_c(&optimized.function);
    assert!(
        emitted.contains("__sir_rotl"),
        "the rewrite must select Rol:\n{emitted}"
    );
    let main = "    printf(\"%llu\\n\", (unsigned long long)rotl3(0x80000001u));\n\
                \x20   printf(\"%llu\\n\", (unsigned long long)rotl3(0x12345678u));\n";
    let Some(stdout) = compile_and_run_emitted(&emitted, main, "rotl3") else {
        return;
    };
    // rol32(0x80000001, 3) = 0x0C = 12; rol32(0x12345678, 3) = 0x91A2B3C0.
    let got: Vec<&str> = stdout.lines().collect();
    assert_eq!(got, vec!["12", "2443359168"]);
}

/// The ps003/ps004 scan loops: `while (x & 1) == 0 { x >>= 1; n += 1 }`
/// (trailing zeros) and the MSB-down variant (leading zeros).
fn zero_count_loop_function(name: &str, leading: bool) -> sir_nodes::Function {
    let ty = Type::u64();
    let mut b = Builder::new(name, &[("value", ty.clone())], ty.clone());
    let value = b.parameter_index(0).unwrap();
    let n_init = b.constant(sir_types::ConstantData::u64(0), ty.clone(), Span::unknown());
    let one = b.constant(sir_types::ConstantData::u64(1), ty.clone(), Span::unknown());
    let zero = b.constant(sir_types::ConstantData::u64(0), ty.clone(), Span::unknown());
    let probe = if leading {
        b.constant(
            sir_types::ConstantData::u64(1 << 63),
            ty.clone(),
            Span::unknown(),
        )
    } else {
        one
    };
    // Leading: probe walks down from the MSB. Trailing: probe is 1 and
    // the value itself is the carried/shifted state.
    let carried_init = if leading { probe } else { value };
    let bit = b.bit_and(value, probe, Span::unknown()).unwrap();
    let cond = b.eq(bit, zero, Span::unknown()).unwrap();
    let next = b.shr(carried_init, one, Span::unknown()).unwrap();
    let n_next = b.add(n_init, one, Span::unknown()).unwrap();
    let loop_node = b
        .r#loop(
            &[bit, cond, next, n_next],
            cond,
            &[next, n_next],
            &[carried_init, n_init],
            Type::Tuple {
                elements: vec![ty.clone(), ty.clone()],
            },
            Span::unknown(),
        )
        .unwrap();
    let res = b.field_access(loop_node, "1", ty, Span::unknown()).unwrap();
    b.return_value(res, Span::unknown()).unwrap();
    b.build()
}
