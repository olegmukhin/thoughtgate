# REQ-POL-001: Cedar Policy Engine

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-POL-001` |
| **Title** | Cedar Policy Engine |
| **Type** | Policy Component |
| **Status** | Draft |
| **Priority** | **Critical** |
| **Tags** | `#policy` `#cedar` `#security` `#classification` `#kubernetes` |

## 1. Context & Decision Rationale

This requirement defines the **policy decision layer** for ThoughtGate—how requests are classified into Green, Amber, Approval, or Red paths based on Cedar policies.

**Why Cedar?**
- Millisecond-latency evaluation (critical for proxy performance)
- Schema-validated policies (catch errors before deployment)
- Expressive policy language (supports complex rules)
- Battle-tested (AWS production workloads)
- Rust-native crate available

**Decision Flow:**
```
┌─────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Request   │────▶│  Cedar Engine   │────▶│    Decision     │
│  (ToolCall) │     │  (This REQ)     │     │ Green/Amber/    │
└─────────────┘     └─────────────────┘     │ Approval/Red    │
                            ▲               └─────────────────┘
                            │
                    ┌───────┴───────┐
                    │   Policies    │
                    │ (ConfigMap/   │
                    │  Env/Default) │
                    └───────────────┘
```

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-CORE-003 | **Receives from** | Parsed MCP requests |
| REQ-CORE-001 | **Routes to** | Green Path decisions |
| REQ-CORE-002 | **Routes to** | Amber Path decisions |
| REQ-CORE-004 | **Routes to** | Red Path (policy denied) |
| REQ-GOV-001 | **Routes to** | Approval decisions |
| REQ-GOV-002 | **Provides to** | Post-approval re-evaluation |

## 3. Intent

The system must:
1. Define a Cedar schema for MCP request classification
2. Evaluate policies to produce 3-way routing decisions (Forward/Approve/Reject)
3. Load policies from ConfigMap, environment, or embedded defaults
4. Hot-reload policies without restart
5. Infer principal identity from Kubernetes environment

> **v0.1 Simplification:** The 4-way classification (Green/Amber/Approval/Red) is reduced to
> 3 actions (Forward/Approve/Reject). Green and Amber paths are deferred until response
> inspection or streaming is needed. Post-approval re-evaluation is simplified since there's
> no Amber Path inspection to perform.

## 4. Scope

### 4.1 In Scope
- Cedar schema definition (entities, actions)
- Policy evaluation logic
- 3-way action output (Forward, Approve, Reject) - v0.1
- Policy loading (ConfigMap → Env → Embedded)
- Policy hot-reload
- Schema validation
- Principal identity inference (K8s)
- Local development mode
- Configuration management

### 4.2 Out of Scope
- Policy authoring UI (users write Cedar directly)
- Policy testing framework (deferred to future version)
- Policy versioning/history (deferred to future version)
- CRD-based policy management (architecture supports, not implemented)
- 4-way classification (Green/Amber/Approval/Red) - deferred to v0.2+
- Post-approval re-evaluation with ApprovalGrant context - deferred to v0.2+

## 5. Constraints

### 5.1 Runtime & Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `cedar-policy` | Policy engine | Latest stable |
| `arc-swap` | Atomic policy swap | 1.x |
| `notify` or polling | File watching | - |

### 5.2 Cedar Schema

```cedar
namespace ThoughtGate;

// ═══════════════════════════════════════════════════════════
// PRINCIPALS
// ═══════════════════════════════════════════════════════════

/// The application pod making requests through ThoughtGate
entity App in [Role] = {
    name: String,               // From HOSTNAME
    namespace: String,          // From K8s ServiceAccount
    service_account: String,    // From K8s ServiceAccount token
};

/// Role for RBAC grouping
entity Role = {
    name: String,
};

// ═══════════════════════════════════════════════════════════
// RESOURCES
// ═══════════════════════════════════════════════════════════

/// An MCP tool call request
entity ToolCall = {
    name: String,               // Tool name, e.g., "delete_user"
    server: String,             // Upstream MCP server identifier
};

/// Generic MCP method for non-tool requests
entity McpMethod = {
    method: String,             // e.g., "resources/read"
    server: String,
};

// ═══════════════════════════════════════════════════════════
// CONTEXT
// ═══════════════════════════════════════════════════════════

/// Approval grant for post-approval re-evaluation
entity ApprovalGrant = {
    task_id: String,
    approved_by: String,
    approved_at: Long,          // Unix timestamp
};

// ═══════════════════════════════════════════════════════════
// ACTIONS (v0.1 Simplified)
// ═══════════════════════════════════════════════════════════

/// Forward: Send request to upstream immediately
action Forward appliesTo {
    principal: [App, Role],
    resource: [ToolCall, McpMethod],
};

/// Approve: Require human/agent approval before forwarding
action Approve appliesTo {
    principal: [App, Role],
    resource: [ToolCall, McpMethod],
};

// Reject is implicit: no action permitted = reject
```

> **v0.1 Note:** `StreamRaw` and `Inspect` actions are removed. When response inspection
> or streaming is needed in future versions, these can be reintroduced.

### 5.2.1 Action Semantics (Routing Reference) - v0.1

**Action-to-Behavior Mapping:**
| Cedar Action | String Literal | Behavior |
|--------------|----------------|----------|
| `Forward` | `"ThoughtGate::Action::Forward"` | Send request to upstream immediately |
| `Approve` | `"ThoughtGate::Action::Approve"` | Block until human approval, then forward |
| (none permitted) | N/A | Reject with policy denial error (-32003) |

**Evaluation Order:**
The policy engine checks actions in this order and returns the first permitted:
1. `Forward` → Send to upstream
2. `Approve` → Block for approval
3. (default) → Reject

**Cross-Module Reference:**
Other modules MUST use these exact action strings when calling Cedar:
```rust
// In REQ-CORE-003 (MCP Transport) routing:
let action = policy_engine.evaluate(
    &principal,
    &resource,
    "ThoughtGate::Action::Forward",  // Check Forward first
);

match action {
    PolicyAction::Forward => upstream.forward(request).await,
    PolicyAction::Approve { timeout } => {
        // Block until approval (v0.1 mode)
        let approval = wait_for_approval(request, timeout).await?;
        upstream.forward(request).await
    }
    PolicyAction::Reject { reason } => {
        Err(ThoughtGateError::PolicyDenied { reason })
    }
}
```

### 5.3 Policy Loading Priority

| Priority | Source | Path / Variable | Use Case |
|----------|--------|-----------------|----------|
| 1 | ConfigMap | `/etc/thoughtgate/policies.cedar` | Production (Hot-Reload) |
| 2 | Env Var | `$THOUGHTGATE_POLICIES` | Simple / CI (< 10KB) |
| 3 | Embedded | Compiled into binary | Local Dev / Fallback |

### 5.4 Identity Inference

**Kubernetes Sources:**
| Attribute | Source | Path |
|-----------|--------|------|
| `name` | Hostname | `$HOSTNAME` |
| `namespace` | SA mount | `/var/run/secrets/kubernetes.io/serviceaccount/namespace` |
| `service_account` | SA token | `/var/run/secrets/kubernetes.io/serviceaccount/token` (parse) |

**Local Development Override:**
| Variable | Purpose |
|----------|---------|
| `THOUGHTGATE_DEV_MODE=true` | Enable dev mode |
| `THOUGHTGATE_DEV_PRINCIPAL` | Override principal (default: `dev-app`) |
| `THOUGHTGATE_DEV_NAMESPACE` | Override namespace (default: `development`) |

### 5.5 Configuration

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Policy file path | `/etc/thoughtgate/policies.cedar` | `THOUGHTGATE_POLICY_FILE` |
| Schema file path | `/etc/thoughtgate/schema.cedarschema` | `THOUGHTGATE_SCHEMA_FILE` |
| Hot-reload interval | 10s | `THOUGHTGATE_POLICY_RELOAD_INTERVAL_SECS` |
| Dev mode | false | `THOUGHTGATE_DEV_MODE` |

## 6. Interfaces

### 6.1 Input: Policy Evaluation Request

```rust
pub struct PolicyRequest {
    pub principal: Principal,
    pub resource: Resource,
    pub context: Option<PolicyContext>,
}

pub struct Principal {
    pub app_name: String,
    pub namespace: String,
    pub service_account: String,
    pub roles: Vec<String>,
}

pub enum Resource {
    ToolCall {
        name: String,
        server: String,
    },
    McpMethod {
        method: String,
        server: String,
    },
}

pub struct PolicyContext {
    pub approval_grant: Option<ApprovalGrant>,
}

pub struct ApprovalGrant {
    pub task_id: String,
    pub approved_by: String,
    pub approved_at: i64,
}
```

### 6.2 Output: Policy Action (v0.1)

```rust
/// v0.1 Simplified Policy Actions
pub enum PolicyAction {
    /// Forward request to upstream immediately
    Forward,

    /// Require approval before forwarding (block until decision)
    Approve {
        /// Timeout for approval workflow
        timeout: Duration,
    },

    /// Reject the request
    Reject {
        /// Reason for denial (safe for logging, not user-facing)
        reason: String,
    },
}
```

> **Note:** The 4-way `PolicyDecision` enum (Green/Amber/Approval/Red) from the original
> design is simplified to 3 actions for v0.1. Green and Amber paths are deferred.

### 6.3 Cedar Engine Interface

```rust
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    /// Evaluate a request and return action (v0.1 simplified)
    fn evaluate(&self, request: &PolicyRequest) -> PolicyAction;

    /// Reload policies from configured source
    async fn reload(&self) -> Result<(), PolicyError>;

    /// Get current policy source
    fn policy_source(&self) -> PolicySource;

    /// Get policy statistics
    fn stats(&self) -> PolicyStats;
}

pub enum PolicySource {
    ConfigMap { path: PathBuf, loaded_at: DateTime<Utc> },
    Environment { loaded_at: DateTime<Utc> },
    Embedded,
}

pub struct PolicyStats {
    pub policy_count: usize,
    pub last_reload: Option<DateTime<Utc>>,
    pub reload_count: u64,
    pub evaluation_count: u64,
}
```

### 6.4 Errors

```rust
pub enum PolicyError {
    /// Policy file not found
    FileNotFound { path: PathBuf },
    
    /// Policy syntax error
    ParseError { details: String, line: Option<usize> },
    
    /// Schema validation failed
    SchemaValidation { details: String },
    
    /// Identity inference failed
    IdentityError { details: String },
}
```

## 7. Functional Requirements

### F-001: Policy Evaluation (v0.1 Simplified)

```
┌─────────────────────────────────────────────────────────┐
│           POLICY EVALUATION (v0.1)                       │
│                                                          │
│   1. Check: Is Forward permitted?                        │
│      └─► YES → Return Forward                            │
│                                                          │
│   2. Check: Is Approve permitted?                        │
│      └─► YES → Return Approve                            │
│                                                          │
│   3. Default: Return Reject (denied)                     │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

- **F-001.1:** Evaluate actions in order: Forward → Approve
- **F-001.2:** First permitted action determines the result
- **F-001.3:** No permitted action results in Reject
- **F-001.4:** Evaluation must complete in < 1ms (P99)

### F-002: Post-Approval Handling - DEFERRED

> **Deferred to v0.2+:** Post-approval re-evaluation with ApprovalGrant context is deferred.
> In v0.1 blocking mode, once approval is received, the request is forwarded immediately
> without re-evaluation. Policy drift detection will be added in a future version.

### F-003: Policy Loading

```rust
fn load_policies() -> Result<PolicySet, PolicyError> {
    // 1. Try ConfigMap
    let config_path = env::var("THOUGHTGATE_POLICY_FILE")
        .unwrap_or_else(|_| "/etc/thoughtgate/policies.cedar".into());
    
    if Path::new(&config_path).exists() {
        info!(path = %config_path, "Loading policies from ConfigMap");
        return load_from_file(&config_path);
    }
    
    // 2. Try Environment Variable
    if let Ok(policy_str) = env::var("THOUGHTGATE_POLICIES") {
        info!("Loading policies from environment variable");
        return parse_policies(&policy_str);
    }
    
    // 3. Fallback to Embedded
    warn!("Using embedded default policies");
    Ok(embedded_default_policies())
}
```

- **F-003.1:** Check ConfigMap path first
- **F-003.2:** Fall back to environment variable
- **F-003.3:** Fall back to embedded default
- **F-003.4:** Log which source was used
- **F-003.5:** Validate against schema before accepting

### F-004: Schema Validation

- **F-004.1:** Load schema from file or embedded
- **F-004.2:** Validate all policies against schema
- **F-004.3:** Reject policies that don't conform
- **F-004.4:** Provide clear error messages for schema violations

### F-005: Hot-Reload

```rust
async fn policy_reload_loop(
    engine: Arc<CedarEngine>,
    path: PathBuf,
    interval: Duration,
) {
    let mut last_mtime = None;
    
    loop {
        tokio::time::sleep(interval).await;
        
        let current_mtime = fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());
        
        if current_mtime != last_mtime {
            match engine.reload().await {
                Ok(()) => {
                    last_mtime = current_mtime;
                    info!("Policies reloaded successfully");
                    metrics::increment!("policy_reloads_total", "status" => "success");
                }
                Err(e) => {
                    error!(error = %e, "Policy reload failed, keeping old policies");
                    metrics::increment!("policy_reloads_total", "status" => "failure");
                }
            }
        }
    }
}
```

- **F-005.1:** Poll file mtime every N seconds (configurable)
- **F-005.2:** Use polling (not inotify) for K8s ConfigMap compatibility
- **F-005.3:** On change: parse → validate → atomic swap
- **F-005.4:** On validation failure: keep old policies, log error
- **F-005.5:** Atomic swap using `arc_swap` for lock-free reads

### F-006: Identity Inference

```rust
fn infer_principal() -> Result<Principal, PolicyError> {
    // Check for dev mode
    if env::var("THOUGHTGATE_DEV_MODE").is_ok() {
        return Ok(dev_mode_principal());
    }
    
    // Kubernetes identity
    let name = env::var("HOSTNAME")
        .map_err(|_| PolicyError::IdentityError {
            details: "HOSTNAME not set".into()
        })?;
    
    let namespace = fs::read_to_string(
        "/var/run/secrets/kubernetes.io/serviceaccount/namespace"
    ).map_err(|_| PolicyError::IdentityError {
        details: "Cannot read namespace from ServiceAccount".into()
    })?;
    
    let sa_token = fs::read_to_string(
        "/var/run/secrets/kubernetes.io/serviceaccount/token"
    ).ok();
    
    let service_account = sa_token
        .and_then(|t| parse_sa_from_token(&t))
        .unwrap_or_else(|| "default".into());
    
    Ok(Principal {
        app_name: name,
        namespace: namespace.trim().into(),
        service_account,
        roles: vec![],  // Roles loaded from policy or external source
    })
}
```

- **F-006.1:** Read identity from K8s ServiceAccount mount
- **F-006.2:** Support dev mode override via environment variables
- **F-006.3:** Log warning when using dev mode
- **F-006.4:** Fail startup if K8s identity required but not available

### F-007: Embedded Default Policy (v0.1)

```cedar
// Default permissive policy for development
// WARNING: Do not use in production

permit(
    principal,
    action == ThoughtGate::Action::"Forward",
    resource
);

permit(
    principal,
    action == ThoughtGate::Action::"Approve",
    resource
);
```

- **F-007.1:** Embedded policy permits Forward and Approve (for dev)
- **F-007.2:** Log WARNING when embedded policy is active
- **F-007.3:** Embedded policy should never be used in production

## 8. Non-Functional Requirements

### NFR-001: Observability

**Metrics:**
```
policy_evaluations_total{action="forward|approve|reject"}
policy_evaluation_duration_seconds{quantile="0.5|0.9|0.99"}
policy_reloads_total{status="success|failure"}
policy_source{source="configmap|env|embedded"}
```

**Logging:**
```json
{"level":"info","message":"Policy evaluation","principal":"app-xyz","resource":"delete_user","action":"approve"}
{"level":"info","message":"Policies reloaded","source":"configmap","policy_count":12}
```

**Audit Log (for compliance):**
```json
{
  "event": "policy_decision",
  "timestamp": "2025-01-08T10:30:00Z",
  "principal": {
    "app": "agent-service",
    "namespace": "production",
    "service_account": "agent-sa"
  },
  "resource": {
    "type": "tool_call",
    "name": "delete_user"
  },
  "action": "approve",
  "policy_source": "configmap"
}
```

### NFR-002: Performance

| Metric | Target |
|--------|--------|
| Evaluation latency (P50) | < 0.1ms |
| Evaluation latency (P99) | < 1ms |
| Hot-reload latency | < 100ms |
| Memory per policy | < 1KB average |

### NFR-003: Reliability

- Policy evaluation must never panic
- Invalid policies must not crash the system
- Hot-reload failures must not affect running policies

## 9. Verification Plan

### 9.1 Edge Case Matrix (v0.1)

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Forward permitted | Return Forward | EC-POL-001 |
| Only Approve permitted | Return Approve | EC-POL-002 |
| No action permitted | Return Reject | EC-POL-003 |
| ConfigMap exists | Load from ConfigMap | EC-POL-004 |
| ConfigMap missing, Env exists | Load from Env | EC-POL-005 |
| Both missing | Load embedded | EC-POL-006 |
| ConfigMap invalid syntax | Keep old, log error | EC-POL-007 |
| ConfigMap schema violation | Keep old, log error | EC-POL-008 |
| ConfigMap updated | Reload within interval | EC-POL-009 |
| K8s identity available | Infer principal | EC-POL-010 |
| K8s identity missing, dev mode | Use dev principal | EC-POL-011 |
| K8s identity missing, no dev | Fail startup | EC-POL-012 |
| Role-based policy | Match role hierarchy | EC-POL-013 |

### 9.2 Assertions

**Unit Tests:**
- `test_evaluate_forward` — Forward permits return Forward
- `test_evaluate_approve` — Approve permits return Approve
- `test_evaluate_reject` — No permits return Reject
- `test_policy_loading_priority` — ConfigMap > Env > Embedded
- `test_schema_validation` — Invalid policies rejected

**Integration Tests:**
- `test_hot_reload_atomic` — 1000 requests during reload, no errors
- `test_configmap_symlink_swap` — K8s-style ConfigMap update works
- `test_identity_inference_k8s` — Identity inferred in K8s environment

**Performance Tests:**
- `bench_evaluation_latency` — Target: P99 < 1ms
- `bench_concurrent_evaluation` — 10k concurrent evaluations

## 10. Implementation Reference

### Cedar Engine Implementation (v0.1)

```rust
pub struct CedarEngine {
    authorizer: Authorizer,
    policies: ArcSwap<PolicySet>,
    schema: Schema,
    principal: Principal,
}

impl CedarEngine {
    /// Evaluate a policy request.
    ///
    /// # Decision Logic (v0.1)
    /// Check Forward → Approve → Reject
    ///
    /// # Returns
    /// - `PolicyAction::Forward` if Forward action is permitted
    /// - `PolicyAction::Approve` if only Approve action is permitted
    /// - `PolicyAction::Reject` if no action is permitted
    pub fn evaluate(&self, request: &PolicyRequest) -> PolicyAction {
        let policies = self.policies.load();

        // v0.1: Check actions in priority order: Forward → Approve
        let actions = ["Forward", "Approve"];

        for action_name in &actions {
            if self.is_action_permitted(request, action_name, &policies) {
                return match *action_name {
                    "Forward" => PolicyAction::Forward,
                    "Approve" => PolicyAction::Approve {
                        timeout: Duration::from_secs(300), // Default 5 minutes
                    },
                    _ => unreachable!(),
                };
            }
        }

        // No action permitted - Reject
        PolicyAction::Reject {
            reason: "No policy permits this request".to_string(),
        }
    }
}
```

### Example Policies (v0.1)

```cedar
// ══════════════════════════════════════════════════════════
// FORWARD: Safe operations go directly to upstream
// ══════════════════════════════════════════════════════════

permit(
    principal,
    action == ThoughtGate::Action::"Forward",
    resource
) when {
    resource.name.startsWith("get_") ||
    resource.name.startsWith("list_") ||
    resource.name.startsWith("describe_") ||
    resource.name.startsWith("read_")
};

// ══════════════════════════════════════════════════════════
// APPROVE: Dangerous operations need human approval
// ══════════════════════════════════════════════════════════

permit(
    principal,
    action == ThoughtGate::Action::"Approve",
    resource
) when {
    resource.name.startsWith("delete_") ||
    resource.name.startsWith("drop_") ||
    resource.name.startsWith("destroy_") ||
    resource.name == "execute_sql" ||
    resource.name == "send_email" ||
    resource.name == "transfer_funds"
};

// Production namespace: writes need approval
permit(
    principal,
    action == ThoughtGate::Action::"Approve",
    resource
) when {
    principal.namespace == "production" &&
    !resource.name.startsWith("get_") &&
    !resource.name.startsWith("list_")
};

// ══════════════════════════════════════════════════════════
// ROLE OVERRIDES: Admins can forward all operations
// ══════════════════════════════════════════════════════════

permit(
    principal in ThoughtGate::Role::"admin",
    action == ThoughtGate::Action::"Forward",
    resource
);
```

### Anti-Patterns to Avoid

- **❌ Blocking on policy load:** Use async loading, don't block startup
- **❌ Mutable policy set:** Use `ArcSwap` for lock-free reads
- **❌ Ignoring schema:** Always validate against schema
- **❌ Logging policy details:** Don't expose policy rules in logs/errors
- **❌ Hardcoded identity:** Always infer from environment

## 11. Definition of Done (v0.1)

- [ ] Cedar schema defined and documented (Forward/Approve actions)
- [ ] Policy evaluation (3-way: Forward/Approve/Reject) implemented
- [ ] Policy loading with priority (ConfigMap → Env → Embedded)
- [ ] Schema validation on load
- [ ] Hot-reload with atomic swap
- [ ] Identity inference (K8s + dev mode)
- [ ] Audit logging for decisions
- [ ] Metrics for evaluations and reloads
- [ ] All edge cases (EC-POL-001 to EC-POL-013) covered
- [ ] Performance target met (P99 < 1ms)
- [ ] Example policies documented