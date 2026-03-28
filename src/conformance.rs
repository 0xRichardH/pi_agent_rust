//! Conformance testing and reporting.
//!
//! This module provides conformance testing infrastructure
//! for validating extension behavior against expected criteria.

pub mod report;

use serde::{Deserialize, Serialize};

/// Conformance test result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConformanceResult {
    /// Test passed.
    Pass,
    /// Test failed with reason.
    Fail(String),
    /// Test was skipped.
    Skip(String),
}

/// Overall conformance verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConformanceVerdict {
    /// All tests passed.
    Pass,
    /// Some tests failed.
    Fail,
    /// Some tests skipped.
    Skip,
}

impl ConformanceVerdict {
    /// Check if verdict is acceptable (pass or skip).
    pub fn is_acceptable(&self) -> bool {
        matches!(self, ConformanceVerdict::Pass | ConformanceVerdict::Skip)
    }
}
