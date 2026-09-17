//! gate6b_run — Gate 6B multi-reduction fusion evaluation harness.
//!
//! Per kernel: outline regions → derive per-region primitive plans →
//! composition (shared buffer/length, no dependency edges) → emit the
//! fused pass → compile → differential + benchmark against the
//! deterministic per-region baseline (3vec).
//!
//! Usage:
//!   gate6b_run --corpus <corpus.ll> --expectations <expected.csv> [--kernel <name>]

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use sir_benchmarks::gate6b::{
    analyze_kernel, build_driver, describe_predicate, eligible_members, emit_fused,
    fusion_candidates, search_plans, RegionPlan,
};

struct Expectation {
    class: String,
    outcome: String,
    min_speedup: Option<f64>,
    rationale: String,
}

fn parse_expectations(path: &str) -> HashMap<String, Expectation> {
    let text = std::fs::read_to_string(path).expect("read expectations");
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("kernel\t") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 4 {
            continue;
        }
        map.insert(
            f[0].to_string(),
            Expectation {
                class: f[1].to_string(),
                outcome: f[2].to_string(),
                min_speedup: f[4].parse::<f64>().ok(),
                rationale: f.get(5).unwrap_or(&"").to_string(),
            },
        );
    }
    map
}

fn print_regions(regions: &[RegionPlan]) {
    for (i, r) in regions.iter().enumerate() {
        let plan = match &r.plan {
            Some(p) => format!(
                "{:?} buffer={} length={} pred={} width={}",
                p.operation,
                p.buffer_name,
                p.length_name,
                describe_predicate(&p.predicate),
                p.vector_width
            ),
            None => "none".to_string(),
        };
        println!(
            "  region[{i}] {} deps={:?} concepts={:?} plan={}",
            r.region.name, r.region.dep_sources, r.concepts, plan
        );
    }
}

fn compile_and_run(driver: &str, work: &Path, sanitize: bool) -> Result<(String, i32), String> {
    std::fs::create_dir_all(work).map_err(|e| format!("mkdir: {e}"))?;
    let c_path = work.join("driver.c");
    let exe_path = work.join("driver");
    std::fs::write(&c_path, driver).map_err(|e| format!("write driver: {e}"))?;
    let mut clang = Command::new("clang");
    clang.args([
        "-O2",
        "-march=native",
        "-mavx2",
        "-msse4.2",
        "-mpopcnt",
        "-std=c11",
        "-D_GNU_SOURCE",
    ]);
    if sanitize {
        // Extend the native differential to the fusion corpus with
        // ASan+UBSan; any diagnostic fails the row instead of comparing
        // outputs from undefined behaviour.
        clang.args([
            "-fsanitize=address,undefined",
            "-fno-sanitize-recover=all",
            "-g",
        ]);
    }
    let out = clang
        .arg(&c_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("-lm")
        .output()
        .map_err(|e| format!("clang spawn: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "clang failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let run = Command::new(&exe_path)
        .output()
        .map_err(|e| format!("run: {e}"))?;
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
            return Err(format!("sanitizer: {reason}"));
        }
    }
    Ok((
        String::from_utf8_lossy(&run.stdout).to_string(),
        run.status.code().unwrap_or(-1),
    ))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut corpus = String::new();
    let mut expectations_path = String::new();
    let mut only_kernel: Option<String> = None;
    let mut sanitize = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--corpus" => {
                corpus = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--expectations" => {
                expectations_path = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--kernel" => {
                only_kernel = args.get(i + 1).cloned();
                i += 2;
            }
            "--sanitize" => {
                sanitize = true;
                i += 1;
            }
            other => {
                eprintln!("unknown argument '{other}'");
                std::process::exit(2);
            }
        }
    }
    if corpus.is_empty() || expectations_path.is_empty() {
        eprintln!(
            "usage: gate6b_run --corpus <corpus.ll> --expectations <expected.csv> [--kernel <name>] [--sanitize]"
        );
        std::process::exit(2);
    }

    let ll_text = std::fs::read_to_string(&corpus).expect("read corpus");
    let expectations = parse_expectations(&expectations_path);
    let kernels: Vec<String> = expectations.keys().cloned().collect();
    let work_root = std::env::temp_dir().join(format!("gate6b-{}", std::process::id()));

    println!("# Gate 6B fusion evaluation");
    println!("# corpus={corpus}");
    println!("# apparatus: gate6b_run + extract_loop_regions + plan derivation + composition + emitter");
    if sanitize {
        println!("# sanitize=address,undefined");
    }

    let mut pass = 0usize;
    let mut fail = 0usize;
    for kernel in kernels {
        if let Some(only) = &only_kernel {
            if &kernel != only {
                continue;
            }
        }
        let expect = expectations.get(&kernel).unwrap();
        println!("\nKERNEL {kernel} class={} expected={}", expect.class, expect.outcome);
        println!("  rationale: {}", expect.rationale);

        let regions = match analyze_kernel(&ll_text, &kernel) {
            Ok(r) => r,
            Err(e) => {
                println!("  ANALYSIS_FAIL {e}");
                if expect.outcome == "no_fuse" {
                    // An N row requires that no fused plan is produced; an
                    // analysis/lowering refusal is a refusal with a recorded
                    // reason, not a fusion outcome.
                    pass += 1;
                    println!("  DECISION no_fuse (analysis refused)");
                    println!("  EXPECTATION PASS");
                } else {
                    fail += 1;
                    println!("  EXPECTATION FAIL (analysis refused)");
                }
                continue;
            }
        };
        print_regions(&regions);
        let candidates = fusion_candidates(&regions);
        println!("  fusion_candidates={}", candidates.len());

        if expect.outcome == "no_fuse" {
            if candidates.is_empty() {
                pass += 1;
                println!("  DECISION no_fuse (composition refused)");
                println!("  EXPECTATION PASS");
            } else {
                fail += 1;
                println!("  DECISION fused (unexpected) candidates={:?}", candidates);
                println!("  EXPECTATION FAIL (fusion must be refused)");
            }
            continue;
        }

        if expect.outcome != "fuse" {
            fail += 1;
            println!("  EXPECTATION FAIL (unknown expected outcome '{}')", expect.outcome);
            continue;
        }

        let Some(candidate) = candidates.iter().max_by_key(|c| c.members.len()) else {
            fail += 1;
            println!("  DECISION no_fuse (no compatible members)");
            println!("  EXPECTATION FAIL (fusion required)");
            continue;
        };
        // Search over the composition action space. Engine 0 has only the
        // per-region action; the enriched space adds the fusion primitive.
        let all_members = eligible_members(&regions);
        // Acceptance workload: the documented pure-win distribution, where
        // an independent All loop cannot early-exit, so the cost model sees
        // the full traversal it would otherwise skip (docs/GATE5B_RESULTS.md,
        // "Early-Exit Interaction"). The early-exit-favouring search is
        // reported as a sensitivity line.
        let options = search_plans(&regions, &candidate.members, 1.0);
        let options_early_exit = search_plans(&regions, &candidate.members, 0.05);
        let engine0 = options
            .iter()
            .find(|o| o.name == "engine0_3vec")
            .map(|o| o.modelled_cost)
            .unwrap_or(f64::NAN);
        let winner = options[0].clone();
        let cost_list: Vec<String> = options
            .iter()
            .map(|o| format!("{}={:.3}", o.name, o.modelled_cost))
            .collect();
        println!(
            "  SEARCH engine0_cost={engine0:.3} search_choice={} members={:?} options=[{}]",
            winner.name,
            winner.fused_members,
            cost_list.join(", ")
        );
        if let Some(ee) = options_early_exit.first() {
            println!(
                "  SEARCH_EARLYEXIT_SENSITIVITY choice={} members={:?} cost={:.3}",
                ee.name, ee.fused_members, ee.modelled_cost
            );
        }
        if winner.fused_members.len() < 2 {
            fail += 1;
            println!("  SEARCH_FAIL (search did not select a fusion plan)");
            println!("  EXPECTATION FAIL");
            continue;
        }
        let fused_members = winner.fused_members.clone();
        let independent_members: Vec<usize> = all_members
            .iter()
            .copied()
            .filter(|m| !fused_members.contains(m))
            .collect();
        let fused = match emit_fused(&kernel, &regions, &fused_members, &all_members) {
            Ok(f) => Some(f),
            Err(e) => {
                fail += 1;
                println!("  EMIT_FAIL {e}");
                println!("  EXPECTATION FAIL");
                continue;
            }
        };
        let driver = match build_driver(
            &kernel,
            &regions,
            &fused_members,
            &independent_members,
            &all_members,
            fused.as_ref(),
        ) {
            Ok(d) => d,
            Err(e) => {
                fail += 1;
                println!("  DRIVER_FAIL {e}");
                println!("  EXPECTATION FAIL");
                continue;
            }
        };
        let work = work_root.join(&kernel);
        match compile_and_run(&driver, &work, sanitize) {
            Ok((stdout, code)) => {
                for line in stdout.lines() {
                    println!("  {line}");
                }
                let correctness_ok = stdout.contains("CORRECTNESS OK");
                let mut speed_ok = true;
                let mut speed_note = String::new();
                if let Some(min) = expect.min_speedup {
                    for line in stdout
                        .lines()
                        .filter(|l| l.starts_with("BENCH ") && l.contains("dist=equal"))
                    {
                        let ratio = line
                            .split("ratio=")
                            .nth(1)
                            .and_then(|r| r.trim().parse::<f64>().ok())
                            .unwrap_or(1.0);
                        let speedup = 1.0 / ratio;
                        if speedup + 1e-9 < min {
                            speed_ok = false;
                            speed_note = format!(
                                " ({} < required {min})",
                                format!("{speedup:.3}x")
                            );
                        }
                    }
                }
                if code == 0 && correctness_ok && speed_ok {
                    pass += 1;
                    println!("  EXPECTATION PASS");
                } else {
                    fail += 1;
                    println!(
                        "  EXPECTATION FAIL (exit={code} correctness_ok={correctness_ok} speed_ok={speed_ok}{speed_note})"
                    );
                }
            }
            Err(e) => {
                fail += 1;
                println!("  COMPILE_OR_RUN_FAIL {e}");
                println!("  EXPECTATION FAIL");
            }
        }
    }

    println!("\nGATE6B_SUMMARY pass={pass} fail={fail}");
    if fail > 0 {
        std::process::exit(1);
    }
}
