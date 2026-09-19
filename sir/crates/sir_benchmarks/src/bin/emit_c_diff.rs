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
        let Some(paren) = rest.find('(') else {
            continue;
        };
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
    out.push_str(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n#include <string.h>\n\n",
    );

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
    let new_p: Vec<String> = func.params.iter().map(|p| emit::c_type(&p.ty)).collect();
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
            new_args.push(format!("({})bufB[{}]", pointer_cast(&p.ty), ptr_slot));
            global_slots.push(ptr_slot);
            ptr_slot += 1;
            continue;
        }
        if is_pointer_ty(&p.ty) {
            let cast = pointer_cast(&p.ty);
            ref_args.push(format!(
                "({})bufA[{}]",
                llvm_c_type(&ir_params[i]),
                ptr_slot
            ));
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
        out.push_str(&format!(
            "        {}__ref({});\n",
            name,
            ref_args.join(", ")
        ));
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

/// Timing driver: same argument setup as the differential driver, but times a
/// fixed repetition count of `name__ref` and `name` in one process so both see
/// identical inputs, buffers, and clock conditions.
fn build_time_driver(
    name: &str,
    left: &str,
    label: &str,
    left_is_sir: bool,
    func: &Function,
    ir_ret: &str,
    ir_params: &[String],
    reps: usize,
    fill: Option<u8>,
) -> String {
    let mut out = String::new();
    out.push_str(
        "#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n\
         #include <string.h>\n#include <time.h>\n\n",
    );

    let ir_p: Vec<String> = ir_params
        .iter()
        .enumerate()
        .map(|(i, t)| format!("{} a{}", llvm_c_type(t), i))
        .collect();
    let new_p: Vec<String> = func.params.iter().map(|p| emit::c_type(&p.ty)).collect();
    let new_params = if new_p.is_empty() {
        "void".to_string()
    } else {
        new_p.join(", ")
    };
    let (left_ret, left_p) = if left_is_sir {
        (emit::c_type(&func.return_ty), new_p.join(", "))
    } else {
        (llvm_c_type(ir_ret), ir_p.join(", "))
    };
    out.push_str(&format!("extern {} {}({});\n\n", left_ret, left, left_p));
    out.push_str(&format!(
        "extern {} {}({});\n\n",
        emit::c_type(&func.return_ty),
        name,
        new_params
    ));

    out.push_str(
        "static uint64_t now_ns(void) {\n\
         \x20   // Invariant TSC ticks: unlike clock_gettime wall time, this is\n\
         \x20   // not distorted by CPU frequency scaling between phases. Only\n\
         \x20   // ratios are consumed, so the historical ref_ns/new_ns labels\n\
         \x20   // are kept even though the unit is ticks.\n\
         \x20   unsigned lo, hi;\n\
         \x20   __asm__ __volatile__(\"rdtsc\" : \"=a\"(lo), \"=d\"(hi));\n\
         \x20   return ((uint64_t)hi << 32) | (uint64_t)lo;\n\
         }\n\n",
    );

    out.push_str("int main(void) {\n");
    out.push_str(
        "    static _Alignas(64) uint8_t bufA[8][2048];\n\
         \x20   static _Alignas(64) uint8_t bufB[8][2048];\n\
         \x20   static const uint64_t lens[] = {0,1,2,3,4,5,7,8,16,31,32};\n\
         \x20   static const uint64_t keys[] = {1,2,3,5,7,11,13,16,31};\n\
         \x20   uint64_t seed = 0x9E3779B97F4A7C15ull;\n\
         \x20   for (int i = 0; i < 8 * 2048; i++) {\n",
    );
    match fill {
        Some(value) => out.push_str(&format!(
            "        ((uint8_t *)bufA)[i] = (uint8_t){value}u;\n\
             \x20       ((uint8_t *)bufB)[i] = (uint8_t){value}u;\n"
        )),
        None => out.push_str(
            "        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;\n\
             \x20       ((uint8_t *)bufA)[i] = (uint8_t)seed;\n\
             \x20       ((uint8_t *)bufB)[i] = (uint8_t)seed;\n",
        ),
    }
    out.push_str("    }\n");

    let mut ref_args: Vec<String> = Vec::new();
    let mut new_args: Vec<String> = Vec::new();
    let mut ptr_slot = 0usize;
    let mut scalar_index = 0usize;
    for (i, p) in func.params.iter().enumerate() {
        if i >= ir_params.len() {
            new_args.push(format!("({})bufB[{}]", pointer_cast(&p.ty), ptr_slot));
            ptr_slot += 1;
            continue;
        }
        if is_pointer_ty(&p.ty) {
            let cast = pointer_cast(&p.ty);
            ref_args.push(format!(
                "({})bufA[{}]",
                llvm_c_type(&ir_params[i]),
                ptr_slot
            ));
            new_args.push(format!("({})bufB[{}]", cast, ptr_slot));
            ptr_slot += 1;
        } else {
            let cty = emit::c_type(&p.ty);
            out.push_str(&format!(
                "    {cty} s{scalar_index} = ({cty})(lens[4] + keys[{scalar_index} % 9]);\n"
            ));
            ref_args.push(format!("({})s{}", llvm_c_type(&ir_params[i]), scalar_index));
            new_args.push(format!("s{scalar_index}"));
            scalar_index += 1;
        }
    }

    let left_args: Vec<String> = if left_is_sir {
        new_args.clone()
    } else {
        ref_args.clone()
    };
    let ret_ty = emit::c_type(&func.return_ty);
    if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        out.push_str(&format!("    {}({});\n", left, left_args.join(", ")));
        out.push_str(&format!("    {}({});\n", name, new_args.join(", ")));
    } else {
        out.push_str(&format!(
            "    if ((uint64_t){}({}) != (uint64_t){}({})) {{\n\
             \x20       printf(\"{label}_MISMATCH {name}\\n\");\n\
             \x20       return 2;\n\
             \x20   }}\n",
            left,
            left_args.join(", "),
            name,
            new_args.join(", ")
        ));
    }
    out.push_str("    volatile uint64_t sink = 0;\n");
    out.push_str("    uint64_t t0, t1, ref_ns, new_ns, b0, b1, b2, b3, b4, o0, o1, o2, o3, o4;\n");
    let ref_loop_body = if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        format!("        {}({});\n", left, left_args.join(", "))
    } else {
        format!(
            "        sink += (uint64_t){}({});\n",
            left,
            left_args.join(", ")
        )
    };
    let new_loop_body = if ret_ty == "void" || llvm_c_type(ir_ret) == "void" {
        format!("        {}({});\n", name, new_args.join(", "))
    } else {
        format!(
            "        sink += (uint64_t){}({});\n",
            name,
            new_args.join(", ")
        )
    };
    out.push_str(&format!(
        "    t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b0 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o0 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o1 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b1 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b2 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o2 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b3 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o3 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{ref_loop_body}    }}\n\
         \x20   t1 = now_ns(); b4 = t1 - t0;\n\
         \x20   t0 = now_ns();\n\
         \x20   for (uint64_t r = 0; r < {reps}ull; r++) {{\n{new_loop_body}    }}\n\
         \x20   t1 = now_ns(); o4 = t1 - t0;\n\
         \x20   ref_ns = b0 < b1 ? b0 : b1; ref_ns = ref_ns < b2 ? ref_ns : b2;\n\
         \x20   ref_ns = ref_ns < b3 ? ref_ns : b3; ref_ns = ref_ns < b4 ? ref_ns : b4;\n\
         \x20   new_ns = o0 < o1 ? o0 : o1; new_ns = new_ns < o2 ? new_ns : o2;\n\
         \x20   new_ns = new_ns < o3 ? new_ns : o3; new_ns = new_ns < o4 ? new_ns : o4;\n\
         \x20   printf(\"{label} {name} reps={reps} ref_ns=%llu new_ns=%llu sink=%llu\\n\",\n\
         \x20       (unsigned long long)ref_ns, (unsigned long long)new_ns, (unsigned long long)sink);\n\
         \x20   return 0;\n}}\n"
    ));
    out
}

/// Define external globals referenced by the reference IR, as the
/// differential driver does.
fn inject_external_globals(driver: &mut String, ll: &str, ref_buffer_base: Option<usize>) {
    let globals = external_globals(ll);
    if globals.is_empty() {
        return;
    }
    let mut defs = String::new();
    for g in &globals {
        defs.push_str(&format!("_Alignas(64) uint8_t {g}[4096];\n"));
    }
    let pos = driver.find("int main").unwrap_or(driver.len());
    driver.insert_str(pos, &defs);
    // In ref mode the original IR reads the real globals, while the emitted C
    // receives those globals as extra pointer parameters. Copy the same
    // buffer contents into the injected globals so both sides see identical
    // data; without this, global-reading functions fail the timing driver's
    // equivalence check (and silently report no downstream measurement).
    if let Some(base) = ref_buffer_base {
        let mut init = String::new();
        for (i, g) in globals.iter().enumerate() {
            init.push_str(&format!(
                "    memcpy({g}, (const void *)bufA[{}], 2048);\n",
                base + i
            ));
        }
        if let Some(fill_end) = driver.find("\n    }\n") {
            driver.insert_str(fill_end + "\n    }\n".len(), &init);
        }
    }
}

/// Build and run the timing harness; returns the `EMITTIME` line on success.
fn time_function(
    ll: &str,
    name: &str,
    workdir: &Path,
    reps: usize,
    baseline: bool,
    fill: Option<u8>,
    cc: &str,
) -> Result<(String, usize), String> {
    let func =
        lower_function(ll, name).map_err(|e| e.lines().last().unwrap_or(&e).trim().to_string())?;
    let Some((ir_ret, ir_params)) = ir_signature(ll, name) else {
        return Err("could not parse IR signature".to_string());
    };
    let optimized = optimize(&func);
    let c_source = emit::emit_c(&optimized.function);
    let dir = workdir.join(name);
    fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let (mut driver, extra_source) = if baseline {
        let base_c = emit::emit_c(&func);
        let renamed = base_c.replace(&format!("{name}("), &format!("{name}__base("));
        let base_path = dir.join("baseline.c");
        fs::write(&base_path, &renamed).map_err(|e| format!("write baseline.c: {e}"))?;
        (
            build_time_driver(
                name,
                &format!("{name}__base"),
                "EMITTIME_BASE",
                true,
                &optimized.function,
                &ir_ret,
                &ir_params,
                reps,
                fill,
            ),
            base_path,
        )
    } else {
        let ref_ll = rename_symbol(ll, name, "__ref");
        let ref_path = dir.join("ref.ll");
        fs::write(&ref_path, &ref_ll).map_err(|e| format!("write ref.ll: {e}"))?;
        (
            build_time_driver(
                name,
                &format!("{name}__ref"),
                "EMITTIME",
                false,
                &optimized.function,
                &ir_ret,
                &ir_params,
                reps,
                fill,
            ),
            ref_path,
        )
    };
    let ref_buffer_base = if baseline {
        None
    } else {
        Some(
            func.params
                .iter()
                .take(ir_params.len())
                .filter(|p| is_pointer_ty(&p.ty))
                .count(),
        )
    };
    inject_external_globals(&mut driver, ll, ref_buffer_base);

    let emitted_path = dir.join("emitted.c");
    let driver_path = dir.join("driver_time.c");
    let exe_path = dir.join("time_exe");
    fs::write(&emitted_path, &c_source).map_err(|e| format!("write emitted.c: {e}"))?;
    fs::write(&driver_path, &driver).map_err(|e| format!("write driver_time.c: {e}"))?;

    compile_harness(
        cc,
        false,
        &exe_path,
        &driver_path,
        &emitted_path,
        &extra_source,
        !baseline,
    )?;
    let run = Command::new(&exe_path)
        .output()
        .map_err(|e| format!("run: {e}"))?;
    let stdout = String::from_utf8_lossy(&run.stdout);
    if !run.status.success() {
        return Err(format!(
            "time harness exit {:?}: {}",
            run.status.code(),
            stdout.lines().next().unwrap_or("")
        ));
    }
    let line = stdout
        .lines()
        .find(|l| l.starts_with("EMITTIME"))
        .map(|s| s.to_string())
        .ok_or_else(|| "no EMITTIME line".to_string())?;
    Ok((line, optimized.rewrites_applied))
}
/// Compile the differential/timing harness with the selected C compiler.
///
/// `extra` is either the original IR (`extra_is_ir`) or an additional C
/// translation unit. GCC cannot read LLVM IR, so the IR is compiled to an
/// object with clang first and linked with the GCC-built driver + emitted C.
fn compile_harness(
    cc: &str,
    sanitize: bool,
    exe: &Path,
    driver: &Path,
    emitted: &Path,
    extra: &Path,
    extra_is_ir: bool,
) -> Result<(), String> {
    let opt_level = std::env::var("SIR_OPT_LEVEL").unwrap_or_else(|_| "1".to_string());
    let opt_flag = format!("-O{opt_level}");
    let sanitizer_args: &[&str] = if sanitize {
        &[
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-g",
        ]
    } else {
        &[]
    };
    let output = if cc == "gcc" {
        let mut objects = Vec::new();
        if extra_is_ir {
            let object = extra.with_extension("o");
            let mut clang = Command::new("clang");
            clang.args([opt_flag.as_str(), "-w", "-falign-functions=64", "-c"]);
            clang.args(sanitizer_args);
            clang.args([extra.to_str().unwrap(), "-o", object.to_str().unwrap()]);
            let compiled = clang
                .output()
                .map_err(|e| format!("clang IR object: {e}"))?;
            if !compiled.status.success() {
                return Err(format!(
                    "clang IR object: {}",
                    String::from_utf8_lossy(&compiled.stderr)
                        .lines()
                        .next()
                        .unwrap_or("")
                ));
            }
            objects.push(object);
        } else {
            objects.push(extra.to_path_buf());
        }
        let mut gcc = Command::new("gcc");
        gcc.args([
            opt_flag.as_str(),
            "-w",
            "-falign-functions=64",
            "-o",
            exe.to_str().unwrap(),
        ]);
        gcc.args(sanitizer_args);
        gcc.arg(driver.to_str().unwrap());
        gcc.arg(emitted.to_str().unwrap());
        for object in &objects {
            gcc.arg(object.to_str().unwrap());
        }
        gcc.output().map_err(|e| format!("gcc: {e}"))?
    } else {
        let mut clang = Command::new("clang");
        clang.args([
            opt_flag.as_str(),
            "-w",
            "-falign-functions=64",
            "-o",
            exe.to_str().unwrap(),
        ]);
        clang.args(sanitizer_args);
        clang.args([
            driver.to_str().unwrap(),
            emitted.to_str().unwrap(),
            extra.to_str().unwrap(),
        ]);
        clang.output().map_err(|e| format!("clang: {e}"))?
    };
    if !output.status.success() {
        return Err(format!(
            "compile: {}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .next()
                .unwrap_or("")
        ));
    }
    Ok(())
}

/// Run the optimizer with stdout suppressed (it prints debug traces).
struct RunResult {
    function: Function,
    rewrites_applied: usize,
}

/// Run the single-pass optimizer.
fn optimize(func: &Function) -> RunResult {
    if let Some((function, rewrites, definitions)) =
        sir_opt::optimize(func, &sir_opt::family::allow_all)
    {
        if std::env::var_os("SIR_OPT_TRACE").is_some() {
            for definition in definitions {
                eprintln!("SIR_OPT {} def={definition}", func.name);
            }
        }
        RunResult {
            function,
            rewrites_applied: rewrites,
        }
    } else {
        RunResult {
            function: func.clone(),
            rewrites_applied: 0,
        }
    }
}

enum Outcome {
    Clean {
        cases: usize,
        rewrites: usize,
    },
    Mismatch {
        cases: usize,
        mismatches: usize,
        rewrites: usize,
    },
    Sanitizer {
        reason: String,
    },
    Refused {
        reason: String,
    },
    HarnessError {
        reason: String,
    },
}

fn run_function(
    ll: &str,
    name: &str,
    cases: usize,
    workdir: &Path,
    sanitize: bool,
    cc: &str,
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
    let ref_ll = rename_symbol(ll, name, "__ref");
    let driver = build_driver(name, &optimized.function, &ir_ret, &ir_params, cases);

    // Define any external globals the module references.
    let mut driver = driver;
    inject_external_globals(&mut driver, ll, None);

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

    // Native assurance hardening: under `sanitize` the ref/emitted pair runs
    // with ASan+UBSan and no recovery, so undefined behaviour aborts instead
    // of silently matching a wrong result.
    if let Err(reason) = compile_harness(
        cc,
        sanitize,
        &exe_path,
        &driver_path,
        &emitted_path,
        &ref_path,
        true,
    ) {
        return Outcome::HarnessError { reason };
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
    if sanitize {
        let stderr = String::from_utf8_lossy(&run.stderr).to_string();
        if stderr.contains("runtime error:")
            || stderr.contains("AddressSanitizer")
            || stderr.contains("UndefinedBehaviorSanitizer")
        {
            let reason = stderr
                .lines()
                .find(|l| {
                    l.contains("runtime error:")
                        || l.contains("AddressSanitizer")
                        || l.contains("UndefinedBehaviorSanitizer")
                })
                .unwrap_or("sanitizer report")
                .to_string();
            return Outcome::Sanitizer { reason };
        }
    }
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

fn ns_value(line: &str, key: &str) -> Option<u64> {
    let prefix = format!("{key}=");
    line.split_whitespace()
        .find_map(|token| token.strip_prefix(&prefix)?.parse().ok())
}

/// Rename every exact `@name` symbol reference, including calls,
/// function-pointer arrays, and metadata such as `llvm.compiler.used`.
/// Longer identifiers sharing the prefix are left untouched.
fn rename_symbol(text: &str, name: &str, suffix: &str) -> String {
    let needle = format!("@{name}");
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(position) = rest.find(&needle) {
        out.push_str(&rest[..position]);
        out.push_str(&needle);
        let after = &rest[position + needle.len()..];
        let continues_identifier = after
            .chars()
            .next()
            .map(|c| c.is_ascii_alphanumeric() || c == '_')
            .unwrap_or(false);
        if !continues_identifier {
            out.push_str(suffix);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: emit_c_diff <corpus.ll> [--cases N] [--workdir DIR] [--function NAME] [--sanitize] [--cc clang|gcc] [--time] [--time-reps N] [--time-baseline] [--measured-retain] [--time-fill V]"
        );
        std::process::exit(2);
    }
    let corpus = &args[1];
    let mut cases = 24usize;
    let mut workdir = std::env::temp_dir().join("emit_c_diff");
    let mut only: Option<String> = None;
    let mut sanitize = false;
    let mut time_mode = false;
    let mut time_baseline = false;
    let mut measured_retain = false;
    let mut time_fill: Option<u8> = None;
    let mut time_reps = 200_000usize;
    let mut cc = "clang".to_string();
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
            "--sanitize" => {
                sanitize = true;
                i += 1;
            }
            "--time" => {
                time_mode = true;
                i += 1;
            }
            "--time-baseline" => {
                time_mode = true;
                time_baseline = true;
                i += 1;
            }
            "--measured-retain" => {
                time_mode = true;
                measured_retain = true;
                i += 1;
            }
            "--time-reps" if i + 1 < args.len() => {
                time_reps = args[i + 1].parse().unwrap_or(200_000);
                i += 2;
            }
            "--time-fill" if i + 1 < args.len() => {
                let raw = args[i + 1].trim();
                time_fill = if let Some(hex) = raw.strip_prefix("0x") {
                    u8::from_str_radix(hex, 16).ok()
                } else {
                    raw.parse::<u8>().ok()
                };
                i += 2;
            }
            "--cc" if i + 1 < args.len() => {
                match args[i + 1].as_str() {
                    "clang" | "gcc" => cc = args[i + 1].clone(),
                    other => {
                        eprintln!("unsupported C compiler: {other} (expected clang or gcc)");
                        std::process::exit(2);
                    }
                }
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
    let mut violations = 0usize;
    let mut retained_base = 0usize;
    for name in function_names(&ll) {
        if let Some(only) = &only {
            if &name != only {
                continue;
            }
        }
        match run_function(
            &ll,
            &name,
            cases,
            &workdir,
            sanitize,
            &cc,
        ) {
            Outcome::Clean { cases, rewrites } => {
                println!("NATIVE {name}: clean cases={cases} rewrites={rewrites}");
                clean += 1;
                if time_mode {
                    match time_function(
                        &ll,
                        &name,
                        &workdir,
                        time_reps,
                        time_baseline,
                        time_fill,
                        &cc,
                    ) {
                        Ok((line, _)) => {
                            println!("{line}");
                            if measured_retain {
                                let ref_ns = ns_value(&line, "ref_ns");
                                let new_ns = ns_value(&line, "new_ns");
                                let retain = match (ref_ns, new_ns) {
                                    (Some(base), Some(new)) if new > base => {
                                        retained_base += 1;
                                        "base"
                                    }
                                    _ => "new",
                                };
                                println!(
                                    "EMITRETAIN {name} retain={retain} ref_ns={} new_ns={}",
                                    ref_ns.unwrap_or(0),
                                    new_ns.unwrap_or(0)
                                );
                            }
                        }
                        Err(reason) => {
                            println!("EMITTIME {name} error={reason}");
                            errors += 1;
                        }
                    }
                }
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
            Outcome::Sanitizer { reason } => {
                println!("NATIVE {name}: SANITIZER ({reason})");
                violations += 1;
            }
            Outcome::HarnessError { reason } => {
                println!("NATIVE {name}: HARNESS_ERROR ({reason})");
                errors += 1;
            }
        }
    }
    if sanitize {
        println!("# sanitize=address,undefined");
        println!("EMITC_SANITIZE violations={violations}");
    }
    let retention = if measured_retain {
        format!(" retained_base={retained_base}")
    } else {
        String::new()
    };
    println!(
        "EMITC_SUMMARY clean={clean} mismatched={mismatched} lower_refused={refused} harness_errors={errors}{retention}"
    );
    std::process::exit(if mismatched == 0 && errors == 0 && violations == 0 {
        0
    } else {
        1
    });
}
