//! emit_c_diff — native differential execution of emitted C vs the
//! original LLVM IR.
//!
//! For every function in a `.ll` corpus: lower to SIR, run the optimizer,
//! emit C (H3 findings F6–F8 fixed), compile the emitted C and a renamed
//! copy of the original IR with clang, run both on identical random
//! inputs, and compare the return value and every pointer buffer.
//!
//! Usage:
//!   emit_c_diff <corpus.ll> [--cases N] [--workdir DIR] [--function NAME]
//!
//! Exit code 0 iff every lowered function was differential-clean.

use sir_benchmarks::emit;
use sir_lower::lower_function;
use sir_nodes::Function;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_types::Type;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Function names defined in the module (`define ... @name(`).
fn function_names(ll: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in ll.lines() {
        let line = line.trim();
        if !line.starts_with("define ") {
            continue;
        }
        let Some(at) = line.find('@') else { continue };
        let rest = &line[at + 1..];
        let Some(paren) = rest.find('(') else { continue };
        names.push(rest[..paren].to_string());
    }
    names
}

/// Internal globals declared `external` in the module: the driver must
/// define them so the renamed reference IR links.
fn external_globals(ll: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in ll.lines() {
        let line = line.trim();
        if !line.starts_with('@') || !line.contains("= external") {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let name = line[1..eq].trim().to_string();
        if !name.contains('"') && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// `(return type, parameter types)` for a definition, raw LLVM tokens.
fn ir_signature(ll: &str, name: &str) -> Option<(String, Vec<String>)> {
    let needle = format!("@{name}(");
    for line in ll.lines() {
        let line = line.trim();
        if !line.starts_with("define ") || !line.contains(&needle) {
            continue;
        }
        let at = line.find(&needle)?;
        let ret = line[..at].split_whitespace().last()?.to_string();
        let after = &line[at + needle.len()..];
        let mut depth = 0usize;
        let mut end = after.len();
        for (i, c) in after.char_indices() {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => {
                    end = i;
                    break;
                }
                ')' => depth -= 1,
                _ => {}
            }
        }
        let mut params = Vec::new();
        for part in after[..end].split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            params.push(part.split_whitespace().next()?.to_string());
        }
        return Some((ret, params));
    }
    None
}

/// C type for a raw LLVM scalar/pointer type token.
fn llvm_c_type(raw: &str) -> String {
    match raw {
        "void" => "void".to_string(),
        "i1" => "bool".to_string(),
        "i8" => "uint8_t".to_string(),
        "i16" => "uint16_t".to_string(),
        "i32" => "uint32_t".to_string(),
        "i64" => "uint64_t".to_string(),
        "float" => "float".to_string(),
        "double" => "double".to_string(),
        t if t.starts_with("ptr") => "const uint8_t *".to_string(),
        _ => "uint64_t".to_string(),
    }
}

fn is_pointer_ty(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Pointer { .. } | Type::Array { .. } | Type::Slice { .. }
    )
}

/// C cast expression type for a pointer-like SIR parameter (arrays decay
/// to element pointers so the cast is valid C).
fn pointer_cast(ty: &Type) -> String {
    match ty {
        Type::Array { element, .. } | Type::Slice { element } => {
            format!("const {} *", emit::c_type(element))
        }
        other => emit::c_type(other),
    }
}

fn build_driver(
    name: &str,
    func: &Function,
    ir_ret: &str,
    ir_params: &[String],
    cases: usize,
) -> String {
    let mut out = String::new();
    out.push_str("#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n#include <string.h>\n\n");

    // Reference prototype (renamed original IR).
    let ir_p: Vec<String> = ir_params
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} a{}", llvm_c_type(t), i))
        .collect();
    out.push_str(&format!(
        "extern {} {}__ref({});\n\n",
        llvm_c_type(ir_ret),
        name,
        ir_p.join(", ")
    ));
    // Emitted prototype (defined in the separately compiled emitted.c).
    let new_p: Vec<String> = func
        .params
        .iter()
        .map(|p| emit::c_type(&p.ty))
        .collect();
    let new_params = if new_p.is_empty() {
        "void".to_string()
    } else {
        new_p.join(", ")
    };
    out.push_str(&format!(
        "extern {} {}({});\n\n",
        emit::c_type(&func.return_ty),
        name,
        new_params
    ));

    out.push_str(
        "static uint64_t val(int t, int k) {\n\
         \x20   static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};\n\
         \x20   static const uint64_t keys[] = {1,2,3,5,7,11,13,16,31};\n\
         \x20   if (k == 0) return lens[t % 11];\n\
         \x20   return keys[(t * 3 + k * 5) % 9];\n\
         }\n\n",
    );

    out.push_str("int main(void) {\n");
    out.push_str("    static _Alignas(64) uint8_t bufA[8][2048];\n");
    out.push_str("    static _Alignas(64) uint8_t bufB[8][2048];\n");
    out.push_str("    uint64_t mismatches = 0;\n");
    out.push_str(&format!("    for (int t = 0; t < {cases}; t++) {{\n"));
    out.push_str("        uint64_t seed = 0x9E3779B97F4A7C15ULL ^ (uint64_t)t;\n");
    out.push_str(
        "        for (int b = 0; b < 8; b++) for (int i = 0; i < 2048; i++) {\n\
         \x20           seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;\n\
         \x20           bufA[b][i] = (uint8_t)seed; bufB[b][i] = bufA[b][i];\n\
         \x20       }\n",
    );

    // Scalar declarations + argument lists.
    let mut ref_args: Vec<String> = Vec::new();
    let mut new_args: Vec<String> = Vec::new();
    let mut ptr_slot = 0usize;
    let mut global_slots: Vec<usize> = Vec::new();
    let mut scalar_index = 0usize;
    for (i, p) in func.params.iter().enumerate() {
        if i >= ir_params.len() {
            // Extra SIR parameter (implicit global array); no IR counterpart.
            new_args.push(format!(
                "({})bufB[{}]",
                pointer_cast(&p.ty),
                ptr_slot
            ));
            global_slots.push(ptr_slot);
            ptr_slot += 1;
            continue;
        }
        if is_pointer_ty(&p.ty) {
            let cast = pointer_cast(&p.ty);
            ref_args.push(format!("({})bufA[{}]", llvm_c_type(&ir_params[i]), ptr_slot));
            new_args.push(format!("({})bufB[{}]", cast, ptr_slot));
            ptr_slot += 1;
        } else {
            let cty = emit::c_type(&p.ty);
            out.push_str(&format!(
                "        {cty} s{scalar_index} = ({cty})val(t, {scalar_index});\n"
            ));
            ref_args.push(format!("({})s{}", llvm_c_type(&ir_params[i]), scalar_index));
            new_args.push(format!("s{scalar_index}"));
            scalar_index += 1;
        }
    }
    // Implicit global arrays: the reference reads its own zero-initialized
    // definition, so the SIR global buffer must be zeroed too.
    for &slot in &global_slots {
        out.push_str(&format!("        memset(bufA[{slot}], 0, 2048);\n"));
        out.push_str(&format!("        memset(bufB[{slot}], 0, 2048);\n"));
    }

    let ret_ty = emit::c_type(&func.return_ty);
    if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        out.push_str(&format!("        {}__ref({});\n", name, ref_args.join(", ")));
        out.push_str(&format!("        {}({});\n", name, new_args.join(", ")));
    } else {
        out.push_str(&format!(
            "        {} r_ref = {}__ref({});\n",
            llvm_c_type(ir_ret),
            name,
            ref_args.join(", ")
        ));
        out.push_str(&format!(
            "        {} r_new = {}({});\n",
            ret_ty,
            name,
            new_args.join(", ")
        ));
        out.push_str(
            "        if ((uint64_t)r_ref != (uint64_t)r_new) {\n\
             \x20           if (mismatches < 5) printf(\"EMITCASE ",
        );
        out.push_str(&format!(
            "{} t=%d ref=%llu new=%llu\\n\", t, (unsigned long long)r_ref, (unsigned long long)r_new);\n",
            name
        ));
        out.push_str("            mismatches++;\n        }\n");
    }
    out.push_str(&format!(
        "        for (int b = 0; b < {ptr_slot}; b++) if (memcmp(bufA[b], bufB[b], 2048)) {{\n\
         \x20           if (mismatches < 5) printf(\"EMITBUF {name} t=%d slot=%d\\n\", t, b);\n\
         \x20           mismatches++;\n\
         \x20       }}\n"
    ));
    out.push_str("    }\n");
    out.push_str(&format!(
        "    printf(\"EMITDIFF {name} cases={cases} mismatches=%llu\\n\", (unsigned long long)mismatches);\n"
    ));
    out.push_str("    return mismatches != 0;\n}\n");
    out
}

/// Run the optimizer with stdout suppressed (it prints debug traces).
fn optimize(func: &Function) -> sir_optimizer::OptimizationResult {
    let dev_null = fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .expect("/dev/null");
    let null_fd = std::os::fd::AsRawFd::as_raw_fd(&dev_null);
    let saved_fd = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(null_fd, 1);
    }
    let config = OptimizerConfig::default();
    let registry = default_registry();
    let optimizer = Optimizer::new(config, registry);
    let result = optimizer.optimize(func);
    unsafe {
        libc::dup2(saved_fd, 1);
        libc::close(saved_fd);
    }
    result
}

enum Outcome {
    Clean { cases: usize, rewrites: usize },
    Mismatch { cases: usize, mismatches: usize, rewrites: usize },
    Refused { reason: String },
    HarnessError { reason: String },
}

fn run_function(
    ll: &str,
    name: &str,
    cases: usize,
    workdir: &Path,
) -> Outcome {
    let func = match lower_function(ll, name) {
        Ok(f) => f,
        Err(e) => {
            return Outcome::Refused {
                reason: e.lines().last().unwrap_or(&e).trim().to_string(),
            }
        }
    };
    let Some((ir_ret, ir_params)) = ir_signature(ll, name) else {
        return Outcome::HarnessError {
            reason: "could not parse IR signature".to_string(),
        };
    };
    let optimized = optimize(&func);
    let c_source = emit::emit_c(&optimized.function);

    let dir: PathBuf = workdir.join(name);
    if fs::create_dir_all(&dir).is_err() {
        return Outcome::HarnessError {
            reason: format!("cannot create {}", dir.display()),
        };
    }
    let ref_ll = ll.replace(&format!("@{name}("), &format!("@{name}__ref("));
    let driver = build_driver(name, &optimized.function, &ir_ret, &ir_params, cases);

    // Define any external globals the module references.
    let mut driver = driver;
    if !external_globals(ll).is_empty() {
        let mut defs = String::new();
        for g in external_globals(ll) {
            defs.push_str(&format!("_Alignas(64) uint8_t {g}[4096];\n"));
        }
        let pos = driver.find("int main").unwrap_or(driver.len());
        driver.insert_str(pos, &defs);
    }

    let emitted_path = dir.join("emitted.c");
    let ref_path = dir.join("ref.ll");
    let driver_path = dir.join("driver.c");
    let exe_path = dir.join("exe");
    if fs::write(&emitted_path, &c_source).is_err()
        || fs::write(&ref_path, &ref_ll).is_err()
        || fs::write(&driver_path, &driver).is_err()
    {
        return Outcome::HarnessError {
            reason: "cannot write harness sources".to_string(),
        };
    }

    let compile = Command::new("clang")
        .args([
            "-O1",
            "-w",
            "-o",
            exe_path.to_str().unwrap(),
            driver_path.to_str().unwrap(),
            emitted_path.to_str().unwrap(),
            ref_path.to_str().unwrap(),
        ])
        .output();
    let compile = match compile {
        Ok(o) => o,
        Err(e) => {
            return Outcome::HarnessError {
                reason: format!("clang spawn: {e}"),
            }
        }
    };
    if !compile.status.success() {
        return Outcome::HarnessError {
            reason: format!(
                "clang: {}",
                String::from_utf8_lossy(&compile.stderr)
                    .lines()
                    .next()
                    .unwrap_or("")
            ),
        };
    }

    let run = match Command::new(&exe_path).output() {
        Ok(o) => o,
        Err(e) => {
            return Outcome::HarnessError {
                reason: format!("run: {e}"),
            }
        }
    };
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let summary = stdout
        .lines()
        .find(|l| l.starts_with("EMITDIFF "))
        .map(|s| s.to_string());
    match summary {
        Some(line) => {
            let mismatches = line
                .split("mismatches=")
                .nth(1)
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(usize::MAX);
            if mismatches == 0 {
                Outcome::Clean {
                    cases,
                    rewrites: optimized.rewrites_applied,
                }
            } else {
                Outcome::Mismatch {
                    cases,
                    mismatches,
                    rewrites: optimized.rewrites_applied,
                }
            }
        }
        None => Outcome::HarnessError {
            reason: format!(
                "no summary (status {:?}): {}",
                run.status.code(),
                stdout.lines().next().unwrap_or("")
            ),
        },
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: emit_c_diff <corpus.ll> [--cases N] [--workdir DIR] [--function NAME]");
        std::process::exit(2);
    }
    let corpus = &args[1];
    let mut cases = 24usize;
    let mut workdir = std::env::temp_dir().join("emit_c_diff");
    let mut only: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--cases" if i + 1 < args.len() => {
                cases = args[i + 1].parse().unwrap_or(24);
                i += 2;
            }
            "--workdir" if i + 1 < args.len() => {
                workdir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--function" if i + 1 < args.len() => {
                only = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    let ll = match fs::read_to_string(corpus) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("cannot read {corpus}: {e}");
            std::process::exit(2);
        }
    };

    let mut clean = 0usize;
    let mut mismatched = 0usize;
    let mut refused = 0usize;
    let mut errors = 0usize;
    for name in function_names(&ll) {
        if let Some(only) = &only {
            if &name != only {
                continue;
            }
        }
        match run_function(&ll, &name, cases, &workdir) {
            Outcome::Clean { cases, rewrites } => {
                println!("NATIVE {name}: clean cases={cases} rewrites={rewrites}");
                clean += 1;
            }
            Outcome::Mismatch {
                cases,
                mismatches,
                rewrites,
            } => {
                println!(
                    "NATIVE {name}: MISMATCH cases={cases} mismatches={mismatches} rewrites={rewrites}"
                );
                mismatched += 1;
            }
            Outcome::Refused { reason } => {
                println!("NATIVE {name}: lower_refused ({reason})");
                refused += 1;
            }
            Outcome::HarnessError { reason } => {
                println!("NATIVE {name}: HARNESS_ERROR ({reason})");
                errors += 1;
            }
        }
    }
    println!(
        "EMITC_SUMMARY clean={clean} mismatched={mismatched} lower_refused={refused} harness_errors={errors}"
    );
    std::process::exit(if mismatched == 0 && errors == 0 { 0 } else { 1 });
}
