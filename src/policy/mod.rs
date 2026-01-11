//! Cedar policy engine for ThoughtGate request classification.
//!
//! Implements: REQ-POL-001 (Cedar Policy Engine)
//!
//! This module provides policy-based routing decisions for MCP requests,
//! classifying them into Green (stream), Amber (inspect), Approval (HITL),
//! or Red (deny) paths based on Cedar policies.

pub mod engine;
pub mod loader;
pub mod principal;

use std::time::Duration;
use thiserror::Error;

/// Policy decision for request routing.
///
/// Implements: REQ-POL-001/§6.2 (Policy Decision Output)
///
/// Each variant corresponds to one of the four traffic paths in ThoughtGate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Green Path: Stream through without buffering (zero-copy).
    ///
    /// Implements: REQ-POL-001/F-001.1
    /// Routes to: REQ-CORE-001 (Zero-Copy Streaming)
    Green,

    /// Amber Path: Buffer and inspect before forwarding.
    ///
    /// Implements: REQ-POL-001/F-001.2
    /// Routes to: REQ-CORE-002 (Buffered Inspection)
    Amber,

    /// Approval Path: Require human/agent approval before proceeding.
    ///
    /// Implements: REQ-POL-001/F-001.3
    /// Routes to: REQ-GOV-001/002/003 (Governance)
    Approval {
        /// Suggested timeout for approval decision
        timeout: Duration,
    },

    /// Red Path: Deny the request (policy violation).
    ///
    /// Implements: REQ-POL-001/F-001.4
    /// Routes to: REQ-CORE-004 (Error Handling)
    Red {
        /// Reason for denial (safe for logging, not user-facing)
        reason: String,
    },
}

/// Request for policy evaluation.
///
/// Implements: REQ-POL-001/§6.1 (Policy Evaluation Request)
#[derive(Debug, Clone)]
pub struct PolicyRequest {
    /// The principal making the request
    pub principal: Principal,

    /// The resource being accessed
    pub resource: Resource,

    /// Optional context for post-approval re-evaluation
    pub context: Option<PolicyContext>,
}

/// Principal identity (app/service making the request).
///
/// Implements: REQ-POL-001/§6.1 (Principal)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// Application name (from HOSTNAME)
    pub app_name: String,

    /// Kubernetes namespace
    pub namespace: String,

    /// Kubernetes ServiceAccount name
    pub service_account: String,

    /// Assigned roles for RBAC
    pub roles: Vec<String>,
}

/// Resource being accessed (MCP tool or method).
///
/// Implements: REQ-POL-001/§6.1 (Resource)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    /// MCP tool call (e.g., "delete_user")
    ToolCall {
        /// Tool name
        name: String,
        /// Upstream server identifier
        server: String,
    },

    /// Generic MCP method (e.g., "resources/read")
    McpMethod {
        /// Method name
        method: String,
        /// Upstream server identifier
        server: String,
    },
}

/// Context for policy evaluation (approval grants, etc.).
///
/// Implements: REQ-POL-001/§6.1 (PolicyContext)
#[derive(Debug, Clone)]
pub struct PolicyContext {
    /// Approval grant for post-approval re-evaluation
    pub approval_grant: Option<ApprovalGrant>,
}

/// Approval grant from human/agent approver.
///
/// Implements: REQ-POL-001/§6.1 (ApprovalGrant)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalGrant {
    /// Task ID that was approved
    pub task_id: String,

    /// Who approved it (user ID or agent ID)
    pub approved_by: String,

    /// Unix timestamp when approved
    pub approved_at: i64,
}

/// Policy engine errors.
///
/// Implements: REQ-POL-001/§6.4 (Errors)
#[derive(Debug, Error, Clone)]
pub enum PolicyError {
    /// Policy file not found
    #[error("Policy file not found: {path}")]
    FileNotFound {
        /// Path that was not found
        path: String,
    },

    /// Policy syntax error
    #[error("Policy parse error at line {line:?}: {details}")]
    ParseError {
        /// Error details
        details: String,
        /// Line number (if available)
        line: Option<usize>,
    },

    /// Schema validation failed
    #[error("Schema validation failed: {details}")]
    SchemaValidation {
        /// Validation error details
        details: String,
    },

    /// Identity inference failed
    #[error("Identity error: {details}")]
    IdentityError {
        /// Error details
        details: String,
    },

    /// Cedar engine error
    #[error("Cedar engine error: {details}")]
    CedarError {
        /// Error details
        details: String,
    },
}

/// Policy loading source.
///
/// Implements: REQ-POL-001/§6.3 (PolicySource)
#[derive(Debug, Clone)]
pub enum PolicySource {
    /// Loaded from ConfigMap file
    ConfigMap {
        /// File path
        path: String,
        /// When it was loaded
        loaded_at: std::time::SystemTime,
    },

    /// Loaded from environment variable
    Environment {
        /// When it was loaded
        loaded_at: std::time::SystemTime,
    },

    /// Embedded default policies
    Embedded,
}

/// Policy engine statistics.
///
/// Implements: REQ-POL-001/§6.3 (PolicyStats)
#[derive(Debug, Clone, Default)]
pub struct PolicyStats {
    /// Number of policies loaded
    pub policy_count: usize,

    /// Last successful reload time
    pub last_reload: Option<std::time::SystemTime>,

    /// Total number of reloads
    pub reload_count: u64,

    /// Total number of evaluations
    pub evaluation_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_decision_variants() {
        let green = PolicyDecision::Green;
        assert!(matches!(green, PolicyDecision::Green));

        let amber = PolicyDecision::Amber;
        assert!(matches!(amber, PolicyDecision::Amber));

        let approval = PolicyDecision::Approval {
            timeout: Duration::from_secs(300),
        };
        assert!(matches!(approval, PolicyDecision::Approval { .. }));

        let red = PolicyDecision::Red {
            reason: "Test denial".to_string(),
        };
        assert!(matches!(red, PolicyDecision::Red { .. }));
    }

    #[test]
    fn test_principal_creation() {
        let principal = Principal {
            app_name: "test-app".to_string(),
            namespace: "production".to_string(),
            service_account: "default".to_string(),
            roles: vec!["user".to_string()],
        };

        assert_eq!(principal.app_name, "test-app");
        assert_eq!(principal.namespace, "production");
        assert_eq!(principal.roles.len(), 1);
    }

    #[test]
    fn test_resource_variants() {
        let tool = Resource::ToolCall {
            name: "delete_user".to_string(),
            server: "mcp-server".to_string(),
        };
        assert!(matches!(tool, Resource::ToolCall { .. }));

        let method = Resource::McpMethod {
            method: "resources/read".to_string(),
            server: "mcp-server".to_string(),
        };
        assert!(matches!(method, Resource::McpMethod { .. }));
    }
}
