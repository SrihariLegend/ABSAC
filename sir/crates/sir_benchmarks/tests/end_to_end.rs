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

    // A loop body's emission claims its statements' whole dependency
    // closure, so a node shared between the body and post-loop code used to
    // be declared *inside* the loop and read *after* it. `n04_stride2` is
    // the case: the bound `p1 << 1` sits in the body as a dependency of a
    // dead comparison and is also read by the post-loop `bound == 0` test,
    // and the emitted C did not compile (`use of undeclared identifier`).
    // The fix hoists such nodes in front of the loops, but only when their
    // value cannot change between iterations — the tempting version, which
    // hoists whatever the body would otherwise swallow, was measured at 7
    // mismatches in 8 cases on this same function before the filter was
    // added. Both halves are pinned here: it must compile, and it must
    // still agree with the original on every case.
    let stride_workdir = std::env::temp_dir().join(format!("sir_e2e_stride_{}", std::process::id()));
    let stride_corpus = format!(
        "{}/../../../gate6a/heldout_corpus.ll",
        env!("CARGO_MANIFEST_DIR")
    );
    let stride = Command::new(binary)
        .arg(&stride_corpus)
        .args(["--cases", "16", "--function", "n04_stride2"])
        .arg("--workdir")
        .arg(&stride_workdir)
        .output()
        .expect("run emit_c_diff on n04_stride2");
    let stride_out = String::from_utf8_lossy(&stride.stdout);
    assert!(
        stride.status.success(),
        "emit_c_diff failed on n04_stride2: {stride_out}{}",
        String::from_utf8_lossy(&stride.stderr)
    );
    assert!(
        stride_out.contains("mismatched=0") && stride_out.contains("harness_errors=0"),
        "a loop-invariant node shared by a body and post-loop code must emit \
         compilable, agreeing C:\n{stride_out}"
    );
    let _ = std::fs::remove_dir_all(&stride_workdir);
}
