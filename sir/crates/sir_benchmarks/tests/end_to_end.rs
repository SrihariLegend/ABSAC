//! End-to-end harness check: one corpus function must emit cleanly and
//! report zero mismatches through the real `emit_c_diff` binary.

use std::process::Command;

#[test]
fn corpus_harness_reports_no_mismatches() {
    let binary = env!("CARGO_BIN_EXE_emit_c_diff");
    let corpus = format!(
        "{}/../../../gate6a/v6_corpus.ll",
        env!("CARGO_MANIFEST_DIR")
    );
    let workdir = std::env::temp_dir().join(format!("sir_e2e_{}", std::process::id()));
    let profile = std::env::temp_dir().join(format!("sir_e2e_{}.profile", std::process::id()));
    let output = Command::new(binary)
        .arg(&corpus)
        .args([
            "--cases",
            "1",
            "--function",
            "p03_count_lt_const_u32",
            "--allow-defs",
            "0",
        ])
        .arg("--measured-retain")
        .args(["--time-reps", "1000"])
        .arg("--profile-out")
        .arg(&profile)
        .arg("--workdir")
        .arg(&workdir)
        .output()
        .expect("run emit_c_diff");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "emit_c_diff failed: {stdout}{stderr}"
    );
    assert!(
        stdout.contains("mismatched=0"),
        "corpus must stay differential-clean:\n{stdout}"
    );
    assert!(
        stdout.contains("EMITRETAIN"),
        "measured-retention decision must be reported:\n{stdout}"
    );
    assert!(
        stdout.contains("EMITPROFILE") && profile.exists(),
        "retention profile must be written:\n{stdout}"
    );
    let _ = std::fs::remove_dir_all(&workdir);
    let _ = std::fs::remove_file(&profile);
}
