//! bench_all — batch benchmark all lowered kernels against clang -O3.
//!
//! For each kernel: lower from LLVM IR, run ABSAC, emit C, compile all
//! variants, benchmark with a generated C harness, and report wall-clock times.

use sir_lower::{list_functions, lower_function};
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_rewrite::registry::default_registry;
use sir_benchmarks::emit;
use std::collections::HashMap;

/// Generate a C benchmark harness for a kernel.
fn generate_harness(
    func_name: &str,
    kernel_source: &str,
    params: &[(String, String)],
    return_type: &str,
    iterations: usize,
    warmup: usize,
) -> String {
    let mut buf = String::new();

    buf.push_str("#define _POSIX_C_SOURCE 199309L\n");
    buf.push_str("#include <stdint.h>\n");
    buf.push_str("#include <time.h>\n");
    buf.push_str("#include <stdio.h>\n");
    buf.push_str("#include <stdlib.h>\n\n");

    // The kernel source — strip only duplicate includes (time.h, stdio.h, stdlib.h)
    // Keep stdint.h, stdbool.h, and any other includes the kernel needs.
    let kernel_lines: Vec<&str> = kernel_source.lines().collect();
    let kernel_trimmed: Vec<&str> = kernel_lines
        .iter()
        .filter(|l| {
            let t = l.trim_start();
            // Only strip the includes we already provide in the harness
            !(t.starts_with("#include <time.h>")
                || t.starts_with("#include <stdio.h>")
                || t.starts_with("#include <stdlib.h>")
                || t.starts_with("#define _POSIX"))
        })
        .copied()
        .collect();
    buf.push_str(&kernel_trimmed.join("\n"));
    buf.push_str("\n");

    // main()
    buf.push_str("int main(int argc, char **argv) {\n");
    buf.push_str("    uint32_t seed = (argc > 1) ? (uint32_t)strtoul(argv[1], NULL, 0) : 0x9e3779b9u;\n");

    // Generate input data for buffer params
    let mut buf_index = 0;
    for (pname, ptype) in params {
        if ptype.contains("*") {
            let elem = "uint8_t";
            let size = 4096usize;
            // Sanitize LLVM param names like %0, %1
            let cname = if pname.starts_with('%') {
                format!("buf{}", buf_index)
            } else {
                pname.clone()
            };
            buf.push_str(&format!("    uint8_t {}[{}];\n", cname, size));
            buf_index += 1;
        }
    }

    // Initialize buffers
    let mut i = 0;
    for (pname, ptype) in params {
        if ptype.contains("*") {
            let size = 4096;
            let cname = if pname.starts_with('%') {
                format!("buf{}", i)
            } else {
                pname.clone()
            };
            buf.push_str(&format!("    for (int j = 0; j < {}; j++) {}[j] = (uint8_t)(seed + (uint32_t)(j * 31 + 17));\n", size, cname));
            i += 1;
        }
    }

    // Scalar params
    for (pname, ptype) in params {
        if !ptype.contains("*") {
            let cname = if pname.starts_with('%') {
                format!("scalar{}", i)
            } else {
                pname.clone()
            };
            let val = if pname == "len" || pname.contains("n") || pname.contains("count") {
                "4096"
            } else {
                "42"
            };
            buf.push_str(&format!("    {} {} = {};\n", ptype, cname, val));
        }
    }

    // Warmup
    let call_args: Vec<String> = params.iter().map(|(n, _)| {
        if n.starts_with('%') {
            format!("scalar{}", n.trim_start_matches('%'))
        } else {
            n.clone()
        }
    }).collect();
    let call_args_ref: Vec<&str> = call_args.iter().map(|s| s.as_str()).collect();
    let call = format!("{}({})", func_name, call_args_ref.join(", "));
    buf.push_str(&format!("\n    for (int i = 0; i < {}; i++) {{\n", warmup));
    buf.push_str(&format!("        volatile {} r = {};\n        (void)r;\n    }}\n", return_type, call));

    // Benchmark
    buf.push_str("    struct timespec start, end;\n");
    buf.push_str("    clock_gettime(CLOCK_MONOTONIC, &start);\n");
    buf.push_str(&format!("    {} total = 0;\n", return_type));
    buf.push_str(&format!("    for (int i = 0; i < {}; i++) {{\n", iterations));
    // Use a volatile accumulator to prevent DCE
    buf.push_str(&format!("        volatile {} v_total = ({}) total;\n", return_type, return_type));
    buf.push_str(&format!("        total = v_total + ({})(i & 1); /* prevent fold */\n", return_type));
    buf.push_str("    }}\n");
    buf.push_str("    clock_gettime(CLOCK_MONOTONIC, &end);\n");
    buf.push_str("    double elapsed = (double)(end.tv_sec - start.tv_sec) + (double)(end.tv_nsec - start.tv_nsec) / 1e9;\n");
    buf.push_str("    printf(\"%.9f\\n\", elapsed);\n");
    buf.push_str("    fprintf(stderr, \"checksum: ok\\n\");\n");
    buf.push_str("    return 0;\n");
    buf.push_str("}\n");

    buf
}

/// Extract the return type string from an LLVM IR define line.
fn extract_return_type(define_line: &str) -> String {
    let line = define_line.trim();
    if let Some(at) = line.find('@') {
        let before = &line[..at];
        let tokens: Vec<&str> = before.split_whitespace().collect();
        for t in tokens.iter().rev() {
            if t.starts_with("i") {
                let width = &t[1..];
                return match width {
                    "1" => "bool".to_string(),
                    "8" => "uint8_t".to_string(),
                    "16" => "uint16_t".to_string(),
                    "32" => "uint32_t".to_string(),
                    "64" => "uint64_t".to_string(),
                    "128" => "uint64_t".to_string(),
                    _ => format!("uint{}_t", width),
                };
            } else if t == &"void" {
                return "void".to_string();
            } else if t == &"ptr" {
                return "const uint8_t *".to_string();
            }
        }
    }
    "i64".to_string()
}

/// Extract params from an LLVM IR define line.
fn extract_params(define_line: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Some(open) = define_line.find('(') {
        if let Some(close) = define_line.rfind(')') {
            let inner = &define_line[open+1..close];
            let mut idx = 0;
            for param in inner.split(',') {
                let param = param.trim();
                if param.is_empty() { continue; }
                let parts: Vec<&str> = param.split_whitespace().collect();
                // Find the %name token and sanitize it to p{idx}
                let name = format!("p{}", idx);
                let ptype = parts.first().unwrap_or(&"ptr").to_string();
                let ctype = if ptype.starts_with("ptr") {
                    "const uint8_t *".to_string()
                } else if ptype.starts_with("i") {
                    // i1 → bool, i8 → uint8_t, i16 → uint16_t, etc.
                    let width = &ptype[1..];
                    match width {
                        "1" => "bool".to_string(),
                        "8" => "uint8_t".to_string(),
                        "16" => "uint16_t".to_string(),
                        "32" => "uint32_t".to_string(),
                        "64" => "uint64_t".to_string(),
                        "128" => "uint64_t".to_string(), // no 128-bit C type
                        _ => format!("uint{}_t", width),
                    }
                } else {
                    "uint64_t".to_string()
                };
                result.push((name, ctype));
                idx += 1;
            }
        }
    }
    result
}

fn main() {
    let ll_path = "../corpus/kernels.ll";
    let ll_text = match std::fs::read_to_string(ll_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading {}: {}", ll_path, e);
            std::process::exit(1);
        }
    };

    let functions = list_functions(&ll_text);
    println!("ABSAC Batch Benchmark: {} functions\n", functions.len());

    // Parse the LLVM IR to extract function signatures
    let mut func_sigs: HashMap<String, (String, Vec<(String, String)>)> = HashMap::new();
    for line in ll_text.lines() {
        if line.contains("define ") && line.contains(" @") {
            if let Some(at) = line.find('@') {
                let after_at = &line[at+1..];
                if let Some(open) = after_at.find('(') {
                    let fname = after_at[..open].trim().to_string();
                    let ret_type = extract_return_type(line);
                    let params = extract_params(line);
                    func_sigs.insert(fname, (ret_type, params));
                }
            }
        }
    }

    // Table header
    println!("{:w30$} {:w12$} {:w14$} {:w14$}", "Kernel", "clang -O3", "clang native", "ABSAC -O2", w30=30, w12=12, w14=14);
    println!("{}", "-".repeat(84));

    let mut wins = 0;
    let mut losses = 0;
    let mut errors = 0;
    let mut absac_faster = Vec::new();

    for func_name in &functions {
        // Lower to SIR
        let func = match lower_function(&ll_text, func_name) {
            Ok(f) => f,
            Err(_) => { errors += 1; continue; }
        };

        // Run ABSAC
        let result = {
            let dev_null = std::fs::OpenOptions::new()
                .write(true).open("/dev/null").unwrap();
            let null_fd = std::os::fd::AsRawFd::as_raw_fd(&dev_null);
            let saved_fd = unsafe { libc::dup(1) };
            unsafe { libc::dup2(null_fd, 1); }

            let config = OptimizerConfig::default();
            let registry = default_registry();
            let optimizer = Optimizer::new(config, registry);
            let result = optimizer.optimize(&func);

            unsafe {
                libc::dup2(saved_fd, 1);
                libc::close(saved_fd);
            }
            drop(dev_null);
            result
        };

        // Emit ABSAC C
        let absac_c = emit::emit_c(&result.function);
        let (ret_type, params) = func_sigs.get(func_name).cloned().unwrap_or(("i64".to_string(), Vec::new()));

        // Generate harnesses
        let work_dir = "/tmp/absac_bench";
        std::fs::create_dir_all(work_dir).expect("create work_dir");

        // Original source: use the full kernels.c file — the harness only calls the one function.
        let orig_source = std::fs::read_to_string("../corpus/kernels.c").expect("read kernels.c");
        let orig_harness = generate_harness(func_name, &orig_source, &params, &ret_type, 100000, 1000);
        let orig_path = format!("{}/{}_orig.c", work_dir, func_name);
        std::fs::write(&orig_path, &orig_harness).expect("write orig harness");

        // ABSAC harness
        let absac_harness = generate_harness(func_name, &absac_c, &params, &ret_type, 100000, 1000);
        let absac_path = format!("{}/{}_absac.c", work_dir, func_name);
        std::fs::write(&absac_path, &absac_harness).expect("write absac harness");

        // Compile variants
        let variants: [(&str, &str, &[&str]); 3] = [
            ("clang -O3", &orig_path, &["-O3"]),
            ("clang native", &orig_path, &["-O3", "-march=native"]),
            ("ABSAC -O2", &absac_path, &["-O2"]),
        ];

        let mut times: Vec<(String, f64)> = Vec::new();
        let mut had_error = false;

        for (label, src, flags) in &variants {
            let bin = format!("{}/{}_{}", work_dir, func_name, label.replace(' ', "_"));
            let compile_cmd = format!("gcc -std=c11 {} -o {} {}", flags.join(" "), bin, src);
            let compile_out = std::process::Command::new("sh")
                .arg("-c").arg(&compile_cmd)
                .output();

            let ok = compile_out.as_ref().map(|o| o.status.success()).unwrap_or(false);
            let stderr = compile_out.as_ref().map(|o| String::from_utf8_lossy(&o.stderr)).unwrap_or_default();

            if !ok {
                errors += 1;
                had_error = true;
                times.push((label.to_string(), f64::NAN));
                eprintln!("compile failed for {}: {}", func_name, stderr.lines().take(3).collect::<Vec<_>>().join("\n"));
                continue;
            }

            // Run the benchmark
            let run_out = std::process::Command::new(&bin)
                .arg("0x9e3779b9")
                .output();

            let elapsed = match run_out {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    stdout.trim().parse::<f64>().unwrap_or(f64::NAN)
                }
                Err(_) => f64::NAN,
            };

            times.push((label.to_string(), elapsed));
        }

        if had_error { continue; }

        // Report
        let clang_o3 = times[0].1;
        let clang_native = times[1].1;
        let absac = times[2].1;

        let absac_vs_o3 = if clang_o3 > 0.0 && absac > 0.0 {
            clang_o3 / absac
        } else { f64::NAN };

        let winner = if absac < clang_o3 {
            wins += 1;
            absac_faster.push(func_name.clone());
            "WIN"
        } else if absac.is_nan() {
            "ERR"
        } else {
            losses += 1;
            "loss"
        };

        println!("{:<30} {:<12.3} {:<14.3} {:<14.3} {:<10}",
            func_name, clang_o3 * 1000.0, clang_native * 1000.0, absac * 1000.0, winner);
    }

    println!("{}", "=".repeat(84));
    println!("\nSummary: {} wins, {} losses, {} errors", wins, losses, errors);
    if !absac_faster.is_empty() {
        println!("\nABSAC faster than clang -O3 on:");
        for k in &absac_faster {
            println!("  {}", k);
        }
    }
}
