//! Reproducible Gate 4B proof artifact.
//!
//! The artifact is a deterministic text file: source digests, the
//! recognized roles (bound to the actual C identifiers), every proved
//! lemma with its statement, and the replay status. Re-running the
//! checker re-parses the sources, re-derives every lemma and replays
//! every derivation; the artifact must come out byte-identical.

use crate::proof::Gate4bProof;

/// One lemma line in the artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LemmaRecord {
    pub name: String,
    pub statement: String,
    pub replayed: bool,
}

/// The Gate 4B artifact for one kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub source_path: String,
    pub source_digest: u64,
    pub function: String,
    pub kind: String,
    pub roles: Vec<(String, String)>,
    /// Every recognized source line, so the artifact records exactly
    /// which bytes the proof was built from.
    pub spans: Vec<(usize, usize, String)>,
    pub lemmas: Vec<LemmaRecord>,
    pub notes: Vec<String>,
    pub binding: Option<crate::binding::SirBinding>,
}

impl Artifact {
    pub fn from_proof(proof: &Gate4bProof) -> Artifact {
        let mut lemmas = Vec::new();
        let mut notes = proof.notes.clone();
        let mut kernel_state = proof.kernel_state.clone();
        for lemma in &proof.lemmas {
            let replayed = match kernel_state.replay(&lemma.theorem) {
                Ok(()) => true,
                Err(e) => {
                    notes.push(format!("lemma {} replay failed: {e}", lemma.name));
                    false
                }
            };
            lemmas.push(LemmaRecord {
                name: lemma.name.clone(),
                statement: lemma.statement.clone(),
                replayed,
            });
        }
        Artifact {
            source_path: proof.kernel.path.clone(),
            source_digest: proof.kernel.digest,
            function: proof.kernel.function.clone(),
            kind: proof.kernel.kind.name().to_string(),
            roles: proof
                .kernel
                .kind
                .roles()
                .into_iter()
                .map(|(r, i)| (r.to_string(), i))
                .collect(),
            spans: proof
                .kernel
                .spans
                .iter()
                .map(|s| (s.first_line, s.last_line, s.text.clone()))
                .collect(),
            lemmas,
            notes,
            binding: proof.binding.clone(),
        }
    }

    /// Deterministic rendering.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("gate4b-artifact v1\n");
        out.push_str(&format!("source: {}\n", self.source_path));
        out.push_str(&format!("source-digest-fnv1a64: {:016x}\n", self.source_digest));
        out.push_str(&format!("function: {}\n", self.function));
        out.push_str(&format!("kernel: {}\n", self.kind));
        for (role, ident) in &self.roles {
            out.push_str(&format!("role {role}: {ident}\n"));
        }
        if let Some(b) = &self.binding {
            out.push_str(&format!("sir-ir: {} (fnv1a64 {:016x})\n", b.ll_path, b.ll_digest));
            out.push_str(&format!(
                "sir-plan: {} {}\n",
                b.plan_operation, b.plan_predicate
            ));
            for (role, ident) in &b.plan_roles {
                out.push_str(&format!("sir-role {role}: {ident}\n"));
            }
            for (idx, emitter_name, node_name) in &b.sir_params {
                out.push_str(&format!(
                    "sir-param {idx}: emitter-name {emitter_name}, node-name {node_name}\n"
                ));
            }
            for (check, ok) in &b.checks {
                out.push_str(&format!(
                    "sir-check {}: {}\n",
                    if *ok { "ok" } else { "FAILED" },
                    check
                ));
            }
        }
        for (first, last, text) in &self.spans {
            if first == last {
                out.push_str(&format!("span {first}: {text}\n"));
            } else {
                out.push_str(&format!("span {first}..{last}: {text}\n"));
            }
        }
        for lemma in &self.lemmas {
            out.push_str(&format!(
                "lemma {} [{}]: {}\n",
                lemma.name,
                if lemma.replayed { "replayed" } else { "REPLAY-FAILED" },
                lemma.statement
            ));
        }
        for note in &self.notes {
            out.push_str(&format!("note: {note}\n"));
        }
        out
    }

    pub fn write(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_text())
    }

    pub fn all_replayed(&self) -> bool {
        !self.lemmas.is_empty() && self.lemmas.iter().all(|l| l.replayed)
    }
}
