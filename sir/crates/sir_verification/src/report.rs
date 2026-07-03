//! VerificationReport — human-readable verification output.

use std::fmt;

use crate::VerificationBackend;

/// A human-readable verification report for a set of obligations.
#[derive(Clone, Debug)]
pub struct VerificationReport {
    pub entries: Vec<ReportEntry>,
}

impl VerificationReport {
    /// Create an empty report.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a report entry.
    pub fn add(&mut self, entry: ReportEntry) {
        self.entries.push(entry);
    }
}

impl fmt::Display for VerificationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, entry) in self.entries.iter().enumerate() {
            writeln!(f, "Obligation #{}", i)?;
            writeln!(f, "Transformation: {}", entry.transformation_name)?;
            writeln!(f, "Backend: {}", entry.backend)?;
            writeln!(f, "Status: {}", entry.status)?;
            if let Some(ref details) = entry.details {
                writeln!(f, "{}", details)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// A single entry in a verification report.
#[derive(Clone, Debug)]
pub struct ReportEntry {
    pub transformation_name: String,
    pub backend: String,
    pub status: ReportStatus,
    pub details: Option<String>,
}

/// The status of a single verification attempt.
#[derive(Clone, Debug)]
pub enum ReportStatus {
    Proven,
    Rejected,
    Unknown,
}

impl fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportStatus::Proven => write!(f, "PROVEN"),
            ReportStatus::Rejected => write!(f, "REJECTED"),
            ReportStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

impl fmt::Display for VerificationBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationBackend::Symbolic => write!(f, "Symbolic"),
            VerificationBackend::Exhaustive => write!(f, "Exhaustive"),
        }
    }
}
