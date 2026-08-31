//! gate5a_bench — generates and benchmarks the full target-plan landscape.
//!
//! For each kernel, enumerates all valid candidate plans, generates C code,
//! compiles, benchmarks at multiple sizes, and records the landscape.
//!
//! Output: CSV to stdout, summary to stderr.

use sir_benchmarks::gate5a::*;
use std::io::Write;

fn main() {
    let kernels = ["k18_all_equal", "k43_sum_ascii", "k50_count_masked"];

    // Generate the full C benchmark harness
    let mut c_code = String::new();
    c_code.push_str("#define _GNU_SOURCE\n");
    c_code.push_str("#include <stdint.h>\n");
    c_code.push_str("#include <stdio.h>\n");
    c_code.push_str("#include <stdlib.h>\n");
    c_code.push_str("#include <string.h>\n");
    c_code.push_str("#include <time.h>\n");
    c_code.push_str("#include <sched.h>\n");
    c_code.push_str("#include <immintrin.h>\n");
    c_code.push_str("\n");

    // Original scalar implementations
    c_code.push_str("static uint64_t k18_all_equal_orig(const uint8_t *buf, uint64_t n, uint8_t val) {\n");
    c_code.push_str("    uint64_t all = 1;\n");
    c_code.push_str("    for (uint64_t i = 0; i < n; i++) all &= (buf[i] == val);\n");
    c_code.push_str("    return all;\n");
    c_code.push_str("}\n\n");
    c_code.push_str("static uint64_t k43_sum_ascii_orig(const uint8_t *buf, uint64_t n) {\n");
    c_code.push_str("    uint64_t sum = 0;\n");
    c_code.push_str("    for (uint64_t i = 0; i < n; i++) sum += buf[i];\n");
    c_code.push_str("    return sum;\n");
    c_code.push_str("}\n\n");
    c_code.push_str("static uint64_t k50_count_masked_orig(const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target) {\n");
    c_code.push_str("    uint64_t count = 0;\n");
    c_code.push_str("    for (uint64_t i = 0; i < n; i++) count += ((buf[i] & mask) == target);\n");
    c_code.push_str("    return count;\n");
    c_code.push_str("}\n\n");

    // Generate all candidate plans
    let mut all_plans: Vec<(String, CandidatePlan)> = Vec::new();
    for kernel in &kernels {
        let plans = enumerate_plans(kernel);
        for plan in plans {
            let params = match *kernel {
                "k18_all_equal" => "const uint8_t *buf, uint64_t n, uint8_t val",
                "k43_sum_ascii" => "const uint8_t *buf, uint64_t n",
                "k50_count_masked" => "const uint8_t *buf, uint64_t n, uint8_t mask, uint8_t target",
                _ => "",
            };
            let extra = match *kernel {
                "k50_count_masked" => "mask, target",
                _ => "",
            };
            let code = emit_plan(&plan, params, "buf", "n", extra);
            c_code.push_str(&code);
            c_code.push_str("\n");
            all_plans.push((kernel.to_string(), plan));
        }
    }

    // Benchmark harness
    c_code.push_str("static volatile uint64_t g_sink = 0;\n");
    c_code.push_str("static inline double now_sec(void) {\n");
    c_code.push_str("    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);\n");
    c_code.push_str("    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;\n");
    c_code.push_str("}\n\n");

    // Correctness check + benchmark for each plan
    c_code.push_str("int main(void) {\n");
    c_code.push_str("    cpu_set_t cpuset; CPU_ZERO(&cpuset); CPU_SET(0, &cpuset);\n");
    c_code.push_str("    sched_setaffinity(0, sizeof(cpuset), &cpuset);\n");
    c_code.push_str("    uint8_t *buf = aligned_alloc(64, 1048576);\n");
    c_code.push_str("    srand(42);\n");
    c_code.push_str("    for (int i = 0; i < 1048576; i++) buf[i] = (uint8_t)(rand() & 0xFF);\n");
    c_code.push_str("    uint8_t val = 0x42, mask = 0x0F, target = 0x05;\n");
    c_code.push_str("    uint64_t sizes[] = {64, 256, 4096, 65536};\n");
    c_code.push_str("    int nsizes = 4;\n");
    c_code.push_str("    uint64_t iters = 20000;\n");
    c_code.push_str("    volatile uint64_t offset = 0;\n");
    c_code.push_str("\n");
    // For k18, also test with all-equal buffer (no early exit)
    c_code.push_str("    // Also prepare all-equal buffer for k18\n");
    c_code.push_str("    uint8_t *eqbuf = aligned_alloc(64, 1048576);\n");
    c_code.push_str("    memset(eqbuf, val, 1048576);\n");
    c_code.push_str("\n");
    c_code.push_str("    printf(\"kernel,plan_id,isa,reduction,unroll,tail,control,size,distribution,correct,median_ns,speedup_vs_orig\\n\");\n");
    c_code.push_str("\n");

    // Generate benchmark calls for each plan
    for (kernel, plan) in &all_plans {
        let id = plan.id;
        let isa_str = format!("{:?}", plan.isa);
        let red_str = format!("{:?}", plan.reduction);
        let unroll_str = format!("{:?}", plan.unroll);
        let tail_str = format!("{:?}", plan.tail);
        let ctrl_str = format!("{:?}", plan.control);

        // Determine which distributions to test
        let distributions: &[&str] = if kernel == "k18_all_equal" {
            &["random", "all_equal"]
        } else {
            &["random"]
        };

        for &dist in distributions {
            for si in 0..4 {
                c_code.push_str("    {\n");
                c_code.push_str(&format!("        uint64_t n = sizes[{}];\n", si));
                if dist == "all_equal" {
                    c_code.push_str("        uint8_t *b = eqbuf;\n");
                } else {
                    c_code.push_str("        uint8_t *b = buf;\n");
                }

                // Correctness check (random data, small size)
                c_code.push_str("        // Correctness check\n");
                let orig_call = match kernel.as_str() {
                    "k18_all_equal" => format!("k18_all_equal_orig(b, n, val)"),
                    "k43_sum_ascii" => format!("k43_sum_ascii_orig(b, n)"),
                    "k50_count_masked" => format!("k50_count_masked_orig(b, n, mask, target)"),
                    _ => "0".to_string(),
                };
                let plan_call = match kernel.as_str() {
                    "k18_all_equal" => format!("k18_all_equal_c{}(b, n, val)", id),
                    "k43_sum_ascii" => format!("k43_sum_ascii_c{}(b, n)", id),
                    "k50_count_masked" => format!("k50_count_masked_c{}(b, n, mask, target)", id),
                    _ => "0".to_string(),
                };
                c_code.push_str(&format!("        uint64_t r_orig = {};\n", orig_call));
                c_code.push_str(&format!("        uint64_t r_plan = {};\n", plan_call));
                c_code.push_str("        int correct = (r_orig == r_plan);\n");

                // Benchmark
                c_code.push_str("        // Benchmark\n");
                c_code.push_str("        double times[15];\n");
                c_code.push_str("        for (int t = 0; t < 15; t++) {\n");
                c_code.push_str("            double t0 = now_sec();\n");
                c_code.push_str("            for (uint64_t i = 0; i < iters; i++) {\n");
                c_code.push_str("                offset = (offset + 1) & 0xFFF;\n");
                c_code.push_str(&format!("                g_sink ^= {}(b + offset, n{});\n",
                    plan_call.split('(').next().unwrap(),
                    if kernel == "k18_all_equal" { ", val" }
                    else if kernel == "k50_count_masked" { ", mask, target" }
                    else { "" }
                ));
                c_code.push_str("            }\n");
                c_code.push_str("            times[t] = (now_sec() - t0) / iters * 1e9;\n");
                c_code.push_str("        }\n");
                c_code.push_str("        // Sort for median\n");
                c_code.push_str("        for (int i = 0; i < 14; i++) for (int j = i+1; j < 15; j++)\n");
                c_code.push_str("            if (times[j] < times[i]) { double tmp = times[i]; times[i] = times[j]; times[j] = tmp; }\n");
                c_code.push_str("        double median = times[7];\n");

                // Also benchmark original
                c_code.push_str("        // Benchmark original\n");
                c_code.push_str("        double orig_times[15];\n");
                c_code.push_str("        for (int t = 0; t < 15; t++) {\n");
                c_code.push_str("            double t0 = now_sec();\n");
                c_code.push_str("            for (uint64_t i = 0; i < iters; i++) {\n");
                c_code.push_str("                offset = (offset + 1) & 0xFFF;\n");
                c_code.push_str(&format!("                g_sink ^= {}(b + offset, n{});\n",
                    orig_call.split('(').next().unwrap(),
                    if kernel == "k18_all_equal" { ", val" }
                    else if kernel == "k50_count_masked" { ", mask, target" }
                    else { "" }
                ));
                c_code.push_str("            }\n");
                c_code.push_str("            orig_times[t] = (now_sec() - t0) / iters * 1e9;\n");
                c_code.push_str("        }\n");
                c_code.push_str("        for (int i = 0; i < 14; i++) for (int j = i+1; j < 15; j++)\n");
                c_code.push_str("            if (orig_times[j] < orig_times[i]) { double tmp = orig_times[i]; orig_times[i] = orig_times[j]; orig_times[j] = tmp; }\n");
                c_code.push_str("        double orig_median = orig_times[7];\n");
                c_code.push_str("        double speedup = (orig_median > 0) ? median / orig_median : 0;\n");

                // Print result
                c_code.push_str(&format!(
                    "        printf(\"{},{},{},{},{},{},{},{},{},%d,%.1f,%.3f\\n\",\n",
                    kernel, id, isa_str, red_str, unroll_str, tail_str, ctrl_str,
                    "%lu", dist
                ));
                c_code.push_str(&format!("               (unsigned long)sizes[{}], correct, median, speedup);\n", si));
                c_code.push_str("    }\n");
            }
        }
    }

    c_code.push_str("    free(buf); free(eqbuf);\n");
    c_code.push_str("    return 0;\n");
    c_code.push_str("}\n");

    // Write to file
    let out_path = std::env::var("GATE5A_OUT").unwrap_or_else(|_| "/tmp/gate5a_bench.c".to_string());
    std::fs::write(&out_path, &c_code).expect("failed to write C file");

    eprintln!("Generated {} ({} bytes, {} plans)",
        out_path, c_code.len(), all_plans.len());
    eprintln!("Plans per kernel:");
    for kernel in &kernels {
        let count = all_plans.iter().filter(|(k, _)| k == kernel).count();
        eprintln!("  {}: {} plans", kernel, count);
    }
    eprintln!("");
    eprintln!("Compile and run with:");
    eprintln!("  clang -O3 -march=native -mavx2 -msse4.2 -mpopcnt -std=c11 -D_GNU_SOURCE {} -o /tmp/gate5a_bench -lm", out_path);
    eprintln!("  /tmp/gate5a_bench > /tmp/gate5a_results.csv");
}
