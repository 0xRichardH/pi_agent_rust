//! Extension conformance matrix generation.
//!
//! This module provides the API matrix and test plan generation
//! for extension conformance testing.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// API capability matrix for extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMatrix {
    /// Extension ID to capability mapping.
    pub extensions: HashMap<String, ExtensionCapabilities>,
}

/// Capabilities required/provided by an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionCapabilities {
    /// Required host capabilities.
    pub requires: Vec<String>,
    /// Provided API surface.
    pub provides: Vec<String>,
}

/// Test plan for conformance testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestPlan {
    /// Extensions to test.
    pub extensions: Vec<TestCase>,
}

/// Individual conformance test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Extension identifier.
    pub extension_id: String,
    /// Required capabilities.
    pub required_caps: Vec<String>,
    /// Expected behaviors to validate.
    pub expected_behaviors: Vec<String>,
}

/// Load API matrix from JSON file.
pub fn load_api_matrix(path: &Path) -> Result<ApiMatrix> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read API matrix from {:?}", path))?;
    let matrix: ApiMatrix = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse API matrix from {:?}", path))?;
    Ok(matrix)
}

/// Build a conformance test plan from inclusion list and API matrix.
pub fn build_test_plan(
    _inclusion: &crate::extension_inclusion::InclusionList,
    _api_matrix: Option<&ApiMatrix>,
) -> TestPlan {
    // Stub implementation - returns empty test plan
    TestPlan { extensions: vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_test_plan_empty() {
        let inclusion = crate::extension_inclusion::InclusionList {
            extensions: vec![],
            version: "1.0.0".to_string(),
        };
        let plan = build_test_plan(&inclusion, None);
        assert!(plan.extensions.is_empty());
    }
}
