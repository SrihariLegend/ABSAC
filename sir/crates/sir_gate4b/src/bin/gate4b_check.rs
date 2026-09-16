//! gate4b_check — run the Gate 4B mechanical proof over the committed
//! generated kernels and write the proof artifacts.
//!
//! Exits non-zero if any kernel cannot be recognized, any chunk lemma
//! fails to prove, or any derivation fails to replay.

use std::path::PathBuf;
use std::process::ExitCode;

use sir_gate4b::artifact::Artifact;
use sir_gate4b::proof::Gate4bProof;

const KERNELS: [(&str, &str); 3] = [
    ("k18_all_equal", "gate4/k18_absac.c"),
    ("k43_sum_ascii", "gate4/k43_absac.c"),
    ("k50_count_masked", "gate4/k50_absac.c"),
];

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/sir_gate4b -> repository root
    dir.pop();
    dir.pop();
    dir.pop();
    dir
}

fn main() -> ExitCode {
    let root = repo_root();
    let out_dir = root.join("gate4/proof");
    let mut ok = true;
    for (name, rel) in KERNELS {
        let path = root.join(rel);
        println!("== {name} ({rel})");
        let ll_path = root.join("corpus/kernels.ll");
        match Gate4bProof::check_file_with_ll(&path, &ll_path) {
            Ok(mut proof) => {
                for note in &proof.notes {
                    println!("   note: {note}");
                }
                match proof.replay_all() {
                    Ok(()) => {}
                    Err(e) => {
                        println!("   REPLAY FAILED: {e}");
                        ok = false;
                        continue;
                    }
                }
                let artifact = Artifact::from_proof(&proof);
                if !artifact.all_replayed() {
                    println!("   artifact has non-replayed lemmas");
                    ok = false;
                }
                let out_path = out_dir.join(format!("{name}.proof.txt"));
                match artifact.write(&out_path) {
                    Ok(()) => println!(
                        "   wrote {} ({} lemmas)",
                        out_path.display(),
                        artifact.lemmas.len()
                    ),
                    Err(e) => {
                        println!("   failed to write artifact: {e}");
                        ok = false;
                    }
                }
            }
            Err(e) => {
                println!("   FAILED: {e}");
                ok = false;
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
