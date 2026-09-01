//! Week-1 probe: OpenSSL constant-time memcmp — what does the pipeline see?
use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_generation::candidate::Candidate;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_nodes::Function;
use sir_optimizer::{Optimizer, OptimizerConfig};
use sir_printer::text::TextPrinter;
use sir_rewrite::registry::default_registry;
use sir_semantics::semantics::SemanticEngine;
use sir_semantics::truth::Provenance;
use sir_types::{ConstantData, Span, Type};

fn span() -> Span { Span::unknown() }

fn build_openssl_ct_memcmp() -> Function {
    let mut b = Builder::new(
        "openssl_ct_memcmp",
        &[
            ("a", Type::Array { element: Box::new(Type::u8()), length: 64 }),
            ("b", Type::Array { element: Box::new(Type::u8()), length: 64 }),
            ("len", Type::u64()),
        ],
        Type::u8(),
    );
    let a = b.parameter_index(0).unwrap();
    let bb = b.parameter_index(1).unwrap();
    let len = b.parameter_index(2).unwrap();
    let one = b.constant(ConstantData::u64(1), Type::u64(), span());
    let i_init = b.constant(ConstantData::u64(0), Type::u64(), span());
    let x_init = b.constant(ConstantData::u8(0), Type::u8(), span());
    let a_byte = b.array_access(a, i_init, Type::u8(), span()).unwrap();
    let b_byte = b.array_access(bb, i_init, Type::u8(), span()).unwrap();
    let xor_val = b.bit_xor(a_byte, b_byte, span()).unwrap();
    let next_x = b.bit_or(x_init, xor_val, span()).unwrap();
    let next_i = b.add(i_init, one, span()).unwrap();
    let cond = b.lt(next_i, len, span()).unwrap();
    let loop_node = b.r#loop(
        &[a_byte, b_byte, xor_val, next_x, next_i, cond],
        cond, &[next_i, next_x], &[i_init, x_init],
        Type::Tuple { elements: vec![Type::u64(), Type::u8()] }, span(),
    ).unwrap();
    let x_result = b.tuple_extract(loop_node, 1, Type::u8(), span()).unwrap();
    b.return_value(x_result, span()).unwrap();
    b.build()
}

fn main() {
    println!("ABSAC Probe: OpenSSL constant-time memcmp\n=========================================\n");
    let func = build_openssl_ct_memcmp();
    let printer = TextPrinter::new(false);
    println!("=== Lowered SIR ===");
    println!("{}", printer.function_to_string(&func));

    let mut verifier = sir_verify::Verifier::new(&func);
    print!("Graph invariants: ");
    println!("{}", if verifier.verify() { "PASS" } else { "FAIL" });

    println!("\n=== Layer 1 — Analysis ===");
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    println!("total facts: {}", analysis.database().total_facts());
    for (id, lf) in analysis.database().loops.iter() {
        println!("loop {:?}: reductions = {:?}", id,
            lf.reductions.iter().map(|r| (&r.reduction_kind, r.variable)).collect::<Vec<_>>());
    }

    println!("\n=== Layer 2 — Semantics ===");
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());
    let truths: Vec<_> = semantics.database().truths().cloned().collect();
    println!("regions: {}", semantics.database().region_count());
    println!("truths:  {}", truths.len());
    for (i, region) in semantics.database().regions() {
        println!("  region {:?}: concepts = {:?}", i, region.concepts());
    }
    for (i, t) in truths.iter().enumerate() {
        let prov = match &t.provenance {
            Provenance::Physical { .. } => "physical",
            Provenance::Derived { .. } => "derived",
        };
        println!("  truth[{}]: {:?} ({})", i, t.concept, prov);
    }
    if truths.is_empty() {
        println!("  >>> GAP: no truths — DisjunctiveReduction did not fire on OR-reduction loop");
    }

    println!("\n=== Layer 3 — Inference ===");
    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());
    let mut beliefs = 0;
    for (_, ctxs) in inference.context_database().contexts() {
        beliefs += ctxs.len();
        for ctx in ctxs { println!("  belief: {:?}", ctx.representation); }
    }
    println!("beliefs: {}", beliefs);

    println!("\n=== Layer 4 — Generation ===");
    let mut generator = CandidateGenerator::new();
    let authorizations = sir_semantics::authorization::derive_authorizations(
        &func,
        analysis.database(),
        semantics.database(),
    );
    generator.generate(inference.context_database(), semantics.database(), &authorizations);
    let candidates: Vec<Candidate> = generator.database().all_candidates().cloned().collect();
    println!("candidates: {}", candidates.len());
    for c in &candidates { println!("  candidate: {:?} {:?}", c.definition_id, c.strategy); }

    println!("\n=== Layer 5 — Optimizer ===");
    let config = OptimizerConfig::default();
    let registry = default_registry();
    let optimizer = Optimizer::new(config, registry);
    let result = optimizer.optimize(&func);
    println!("rewrites: {} ({} -> {})", result.rewrites_applied, result.initial_nodes, result.final_nodes);
    println!("termination: {:?}", result.termination);

    println!("\n=== Summary ===");
    println!("Recognizer fired: {}", if !truths.is_empty() { "YES" } else { "NO" });
    println!("Rewrite applied:  {}", if result.rewrites_applied > 0 { "YES" } else { "NO" });
    if truths.is_empty() {
        println!("\nGAP: DisjunctiveReduction recognizer needs to handle OR-reduction");
        println!("     over XOR of two array accesses (a[i] ^ b[i] | x pattern).");
    }
}
