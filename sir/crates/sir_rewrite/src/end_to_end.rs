//! Re-export of the matched end-to-end artifact (the only artifact
//! that may authorize mutation). Defined in `sir_verification`;
//! re-exported here so `RewriteResult` can name it without pulling
//! the verification crate into the result type's public surface.
pub use sir_verification::application_artifact::EndToEndVerificationArtifact;
