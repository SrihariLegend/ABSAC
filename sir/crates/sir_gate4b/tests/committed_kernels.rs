//! End-to-end Gate 4B chunk proofs against the committed artifacts.
//!
//! Each test reads `gate4/<kernel>.c` from disk, matches it against the
//! strict template, builds the kernel-term model from the recognized
//! instruction sequence, proves the per-chunk identities by
//! bit-blasting the SDM intrinsic models against the scalar element
//! fold, and replays every derivation.

use std::path::PathBuf;

use sir_gate4b::artifact::Artifact;
use sir_gate4b::proof::Gate4bProof;

fn repo_path(rel: &str) -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir.pop();
    dir.join(rel)
}

/// Requirement 8: the artifact is reproducible and matches the copy in
/// the tree byte for byte.
#[test]
fn artifacts_are_reproducible() {
    let cases = [
        ("gate4/k18_absac.c", "k18_all_equal"),
        ("gate4/k43_absac.c", "k43_sum_ascii"),
        ("gate4/k50_absac.c", "k50_count_masked"),
    ];
    let ll = repo_path("corpus/kernels.ll");
    for (c_path, name) in cases {
        let mut proof =
            Gate4bProof::check_file_with_ll(repo_path(c_path), &ll).expect("proof with binding");
        proof.replay_all().expect("replay");
        let artifact = Artifact::from_proof(&proof);
        assert!(artifact.all_replayed(), "{name}: lemmas must replay");
        assert!(
            artifact
                .binding
                .as_ref()
                .map(|b| b.all_ok())
                .unwrap_or(false),
            "{name}: SIR binding must hold"
        );
        let committed = repo_path(&format!("gate4/proof/{name}.proof.txt"));
        let generated = artifact.to_text();
        if committed.exists() {
            let on_disk = std::fs::read_to_string(&committed).expect("read artifact");
            assert_eq!(
                on_disk, generated,
                "{name}: committed artifact differs from a fresh run"
            );
        }
    }
}

#[test]
fn k50_chunk_lemmas_proved_from_committed_file() {
    let proof = Gate4bProof::check_file(repo_path("gate4/k50_absac.c")).expect("k50 proof");
    for note in &proof.notes {
        eprintln!("note: {note}");
    }
    assert!(
        proof.lemmas.len() >= 3,
        "expected 32/16/1 chunk lemmas, got {}",
        proof.lemmas.len()
    );
    let mut proof = proof;
    proof.replay_all().expect("replay all chunk lemmas");
    let artifact = Artifact::from_proof(&proof);
    assert!(artifact.all_replayed());
    eprintln!("{}", artifact.to_text());
}

#[test]
fn k18_chunk_lemmas_proved_from_committed_file() {
    let proof = Gate4bProof::check_file(repo_path("gate4/k18_absac.c")).expect("k18 proof");
    for note in &proof.notes {
        eprintln!("note: {note}");
    }
    let mut proof = proof;
    proof.replay_all().expect("replay all chunk lemmas");
    let artifact = Artifact::from_proof(&proof);
    assert!(artifact.all_replayed());
    eprintln!("{}", artifact.to_text());
}

#[test]
fn k43_chunk_lemmas_proved_from_committed_file() {
    let proof = Gate4bProof::check_file(repo_path("gate4/k43_absac.c")).expect("k43 proof");
    for note in &proof.notes {
        eprintln!("note: {note}");
    }
    let mut proof = proof;
    proof.replay_all().expect("replay all chunk lemmas");
    let artifact = Artifact::from_proof(&proof);
    assert!(artifact.all_replayed());
    eprintln!("{}", artifact.to_text());
}
