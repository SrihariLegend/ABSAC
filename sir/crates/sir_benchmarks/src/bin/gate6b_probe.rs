//! Development probe: outline a multi-loop kernel and print the derived
//! per-region primitive plans. Used while building the Gate 6B fusion
//! automation on development kernels; the sealed corpus is only touched
//! by the one-shot `gate6b_run` evaluation.

use sir_benchmarks::gate6b::{analyze_kernel, describe_predicate, fusion_candidates};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: gate6b_probe <file.ll> <function>");
        std::process::exit(2);
    }
    let ll = std::fs::read_to_string(&args[1]).expect("read .ll");
    let regions = match analyze_kernel(&ll, &args[2]) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("analyze refused: {e}");
            std::process::exit(1);
        }
    };
    for (i, r) in regions.iter().enumerate() {
        println!(
            "region {}: {} params={:?} deps={:?} concepts={:?}",
            i, r.region.name, r.region.params, r.region.dep_sources, r.concepts
        );
        match &r.plan {
            Some(p) => println!(
                "  plan: {:?} buffer={} length={} predicate={} width={} buf_param={:?} len_param={:?}",
                p.operation,
                p.buffer_name,
                p.length_name,
                describe_predicate(&p.predicate),
                p.vector_width,
                r.buffer_param(),
                r.length_param()
            ),
            None => println!("  plan: none"),
        }
    }
    for c in fusion_candidates(&regions) {
        println!("fusion candidate: members={:?}", c.members);
    }
}
