//! Policy types for Cedar policy engine integration.
//!
//! # v0.1 Simplified Model
//!
//! The policy engine evaluates requests and returns one of three actions:
//!
//! - **Forward**: Send request directly to upstream
//! - **Approve**: Require human approval before forwarding
//! - **Reject**: Deny the request with an error
//!
//! This replaces the original 4-way classification (Green/Amber/Approval/Red).
//! Green and Amber paths are deferred until response inspection or LLM streaming is needed.
//!
//! # Traceability
//! - Implements: REQ-POL-001 (Cedar Policy Engine)
//! - Implements: REQ-POL-001/F-001 (Policy Evaluation)

use std::time::Duration;

/// v0.1 Simplified Policy Actions.
///
/// The result of evaluating Cedar policies against an MCP request.
/// This enum determines how the request is handled.
///
/// # Evaluation Order
///
/// Policies are evaluated in this order:
/// 1. `Forward` - If permitted, send immediately
/// 2. `Approve` - If permitted, require human approval
/// 3. (default) - If nothing permitted, reject
///
/// # Traceability
/// - Implements: REQ-POL-001/§6.2 (Policy Action output)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyAction {
    /// Forward request to upstream immediately.
    ///
    /// The request is sent to the MCP server without human approval.
    /// Response is passed through directly to the agent.
    Forward,

    /// Require human approval before forwarding.
    ///
    /// In v0.1 blocking mode, the HTTP connection is held open until
    /// approval is received via Slack. On approval, the request is
    /// forwarded; on rejection or timeout, an error is returned.
    Approve {
        /// Timeout for the approval workflow.
        /// After this duration, the request fails with ApprovalTimeout.
        timeout: Duration,
    },

    /// Reject the request with a policy denial error.
    ///
    /// Returns JSON-RPC error code -32003 (PolicyDenied) to the agent.
    Reject {
        /// Reason for denial (safe for logging, not user-facing).
        /// Does not expose policy internals.
        reason: String,
    },
}

impl PolicyAction {
    /// Returns `true` if this action forwards the request immediately.
    pub fn is_forward(&self) -> bool {
        matches!(self, PolicyAction::Forward)
    }

    /// Returns `true` if this action requires approval.
    pub fn is_approve(&self) -> bool {
        matches!(self, PolicyAction::Approve { .. })
    }

    /// Returns `true` if this action rejects the request.
    pub fn is_reject(&self) -> bool {
        matches!(self, PolicyAction::Reject { .. })
    }

    /// Returns the approval timeout if this is an Approve action.
    pub fn timeout(&self) -> Option<Duration> {
        match self {
            PolicyAction::Approve { timeout } => Some(*timeout),
            _ => None,
        }
    }
}

impl Default for PolicyAction {
    /// Default action is to reject (fail-closed).
    ///
    /// If no policy explicitly permits the action, it is denied.
    fn default() -> Self {
        PolicyAction::Reject {
            reason: "No policy permits this action".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_action() {
        let action = PolicyAction::Forward;
        assert!(action.is_forward());
        assert!(!action.is_approve());
        assert!(!action.is_reject());
        assert_eq!(action.timeout(), None);
    }

    #[test]
    fn test_approve_action() {
        let action = PolicyAction::Approve {
            timeout: Duration::from_secs(300),
        };
        assert!(!action.is_forward());
        assert!(action.is_approve());
        assert!(!action.is_reject());
        assert_eq!(action.timeout(), Some(Duration::from_secs(300)));
    }

    #[test]
    fn test_reject_action() {
        let action = PolicyAction::Reject {
            reason: "Not permitted".to_string(),
        };
        assert!(!action.is_forward());
        assert!(!action.is_approve());
        assert!(action.is_reject());
        assert_eq!(action.timeout(), None);
    }

    #[test]
    fn test_default_is_reject() {
        let action = PolicyAction::default();
        assert!(action.is_reject());
    }
}
