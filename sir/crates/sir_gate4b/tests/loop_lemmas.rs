//! Loop-level lemmas: the fold splitting law the loop invariants need.

use std::path::PathBuf;

use sir_gate4b::emitted::parse_file;
use sir_gate4b::loops::fold_split_law;
use sir_gate4b::model::Model;
use sir_mech::kernel::Kernel;

fn repo_path(rel: &str) -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir.pop();
    dir.join(rel)
}

fn split_law_for(rel: &str) {
    let kernel = parse_file(repo_path(rel)).expect("recognized");
    let mut k = Kernel::new();
    let model = Model::build(&kernel, &mut k).expect("model");
    let split = fold_split_law(&mut k, &model).expect("split law");
    eprintln!("{}", k.describe(&split.theorem.statement));
    assert_eq!(split.theorem.statement.hyps.len(), 2, "b <= c and c <= a");
    k.replay(&split.theorem).expect("replay split law");
}

#[test]
fn split_law_k50() {
    split_law_for("gate4/k50_absac.c");
}

#[test]
fn split_law_k43() {
    split_law_for("gate4/k43_absac.c");
}

fn bound_and_memory_for(rel: &str, expected_bound: i128, width: usize) {
    let kernel = parse_file(repo_path(rel)).expect("recognized");
    let mut k = Kernel::new();
    let model = Model::build(&kernel, &mut k).expect("model");
    let bound = sir_gate4b::bounds::accumulator_bound(&mut k, &model, "b").expect("bound");
    eprintln!("{}", k.describe(&bound.theorem.statement));
    assert_eq!(bound.bound, expected_bound);
    k.replay(&bound.theorem).expect("replay bound");
    let mem = sir_gate4b::bounds::memory_bounds_lemma(&mut k, width, "m").expect("memory");
    k.replay(&mem).expect("replay memory bounds");
    let scalar = sir_gate4b::bounds::scalar_memory_bound(&mut k, "s").expect("scalar memory");
    k.replay(&scalar).expect("replay scalar memory");
}

#[test]
fn bounds_k50() {
    bound_and_memory_for("gate4/k50_absac.c", 1, 32);
}

#[test]
fn bounds_k43() {
    bound_and_memory_for("gate4/k43_absac.c", 255, 32);
}
