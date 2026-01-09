# REQ-GOV-002: Approval Execution Pipeline

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-GOV-002` |
| **Title** | Approval Execution Pipeline |
| **Type** | Governance Component |
| **Status** | Draft |
| **Priority** | **High** |
| **Tags** | `#governance` `#pipeline` `#amber` `#execution` `#approval` |

## 1. Context & Decision Rationale

This requirement defines the **execution pipeline** for approval-required requests. When a tool call requires human approval, it goes through a multi-phase pipeline:

1. **Pre-Approval Amber:** Transform/validate before showing to human
2. **Approval Wait:** Human reviews and decides
3. **Post-Approval Amber:** Re-validate after approval (policy may have changed)
4. **Execution:** Forward to upstream MCP server

**Why Two Amber Phases?**

| Phase | Purpose |
|-------|---------|
| Pre-Approval | Don't waste human time on requests that would fail anyway |
| Post-Approval | Catch policy drift, re-validate with current rules |

**Key Design Decision:** Human approves the *transformed* request (Option B), not the original. This ensures the human sees exactly what will be executed.

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-CORE-002 | **Uses** | Amber Path infrastructure (inspectors) |
| REQ-CORE-003 | **Uses** | Upstream forwarding |
| REQ-CORE-004 | **Uses** | Error responses for pipeline failures |
| REQ-POL-001 | **Uses** | Policy re-evaluation with approval context |
| REQ-GOV-001 | **Uses** | Task state transitions |
| REQ-GOV-003 | **Coordinates with** | Receives approval decisions |

## 3. Intent

The system must:
1. Run Pre-Approval Amber inspection before task creation
2. Store both original and transformed request in task
3. Trigger execution pipeline when approval is received
4. Validate approval and re-evaluate policy
5. Run Post-Approval Amber inspection
6. Detect and handle transform drift
7. Forward to upstream and store result

**⚠️ Result Storage for Bridged Tools (IMPORTANT)**

When the upstream is a **Bridged Tool** (HTTP→MCP via Tool Bridge), the execution pipeline must:
1. Execute the HTTP request against the backend service
2. Apply `output_mapping` to transform HTTP response → MCP ToolResult
3. Store the **final, mapped MCP ToolResult** in the task (not the raw HTTP response)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    BRIDGED TOOL RESULT MAPPING                                  │
│                                                                                 │
│   Execution:                                                                    │
│   1. HTTP POST /api/users/123 → HTTP 200 {"status": "deleted", "id": 123}      │
│                                                                                 │
│   2. Apply output_mapping (from bridge config):                                │
│      result.text = "User 123 deleted successfully"                             │
│      result.metadata.raw_status = response.status                              │
│                                                                                 │
│   3. Store in task.result:                                                     │
│      {                                                                          │
│        "content": [{ "type": "text", "text": "User 123 deleted successfully" }],│
│        "isError": false                                                         │
│      }                                                                          │
│                                                                                 │
│   ✅ Client receives standard MCP ToolResult                                    │
│   ✅ Client never sees raw HTTP response                                        │
│   ✅ Consistent interface regardless of upstream type                          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Rationale:**
- Client should not need to know if tool is native MCP or bridged HTTP
- `tasks/result` returns the same format regardless of execution path
- Output mapping happens at execution time, not result retrieval time

## 4. Scope

### 4.1 In Scope
- Pre-Approval Amber phase (before task creation)
- Execution pipeline (after approval)
- Approval validation
- Policy re-evaluation with approval context
- Post-Approval Amber phase
- Transform drift detection
- Upstream forwarding
- Result/failure handling
- Pipeline metrics and logging

### 4.2 Out of Scope
- Inspector implementation (REQ-CORE-002)
- Approval adapter integration (REQ-GOV-003)
- Task storage (REQ-GOV-001)
- Policy evaluation logic (REQ-POL-001)

## 5. Constraints

### 5.1 Approval Validity

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Approval validity window | 300s (5 min) | `THOUGHTGATE_APPROVAL_VALIDITY_SECS` |
| Transform drift mode | strict | `THOUGHTGATE_TRANSFORM_DRIFT_MODE` |

**Transform Drift Modes:**
| Mode | Behavior |
|------|----------|
| `strict` | Fail if Post-Approval transform differs from Pre-Approval |
| `permissive` | Log warning, continue with new transform |

### 5.2 Pipeline Timeout

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Execution timeout | 30s | `THOUGHTGATE_EXECUTION_TIMEOUT_SECS` |

## 6. Interfaces

### 6.1 Pipeline Input

```rust
pub struct PipelineInput {
    pub task: Task,
    pub approval: ApprovalRecord,
}
```

### 6.2 Pipeline Output

```rust
pub enum PipelineResult {
    Success {
        result: ToolCallResult,
    },
    Failure {
        stage: FailureStage,
        reason: String,
        retriable: bool,
    },
}
```

### 6.3 Pipeline Interface

```rust
#[async_trait]
pub trait ExecutionPipeline: Send + Sync {
    /// Run Pre-Approval Amber phase before task creation
    async fn pre_approval_amber(
        &self,
        request: &ToolCallRequest,
        principal: &Principal,
    ) -> Result<PreHitlResult, PipelineError>;
    
    /// Execute approved task through full pipeline
    async fn execute_approved(
        &self,
        task: &Task,
        approval: &ApprovalRecord,
    ) -> PipelineResult;
}

pub struct PreHitlResult {
    pub transformed_request: ToolCallRequest,
    pub request_hash: String,
}

pub enum PipelineError {
    InspectionRejected { inspector: String, reason: String },
    InternalError { details: String },
}
```

### 6.4 Inspector Interface (from REQ-CORE-002)

```rust
/// Inspector trait - MUST match REQ-CORE-002 definition
#[async_trait]
pub trait Inspector: Send + Sync {
    async fn inspect(
        &self,
        body: &[u8],
        context: &InspectionContext,
    ) -> Result<InspectorDecision, InspectorError>;
    
    fn behavior(&self) -> InspectorBehavior;
    fn name(&self) -> &str;
}

pub enum InspectorBehavior {
    Observe,    // Can only observe, not modify
    Validate,   // Can reject but not modify
    Transform,  // Can modify the request
}

/// Decision enum - canonical definition in REQ-CORE-002
pub enum InspectorDecision {
    Approve,
    Modify(Bytes),
    Reject { status: StatusCode, reason: String },
}
```

## 7. Functional Requirements

### F-001: Pre-Approval Amber Phase

```
┌─────────────────────────────────────────────────────────────────────┐
│                     PRE-APPROVAL AMBER PHASE                            │
│                                                                     │
│   Input: Original ToolCallRequest                                   │
│                                                                     │
│   For each Inspector in chain:                                      │
│     • Observe  → Continue                                           │
│     • Validate → Continue or Reject                                 │
│     • Transform → Continue with modified request                    │
│                                                                     │
│   On Reject: Return error, do NOT create task                       │
│                                                                     │
│   Output: TransformedRequest + Hash                                 │
│                                                                     │
│   This transformed request is what the human will see and approve   │
└─────────────────────────────────────────────────────────────────────┘
```

- **F-001.1:** Run all inspectors in registration order
- **F-001.2:** Pass modified request to next inspector in chain
- **F-001.3:** On any rejection, fail immediately (no task created)
- **F-001.4:** Compute hash of final transformed request
- **F-001.5:** Return transformed request for task storage

### F-002: Execution Pipeline (Post-Approval)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    EXECUTION PIPELINE                               │
│                                                                     │
│   1. VALIDATION                                                     │
│      • Approval not expired?                                        │
│      • Request hash matches stored?                                 │
│      • Task in correct state?                                       │
│                                                                     │
│   2. POLICY RE-EVALUATION                                           │
│      • Evaluate with ApprovalGrant context                          │
│      • Any permit → continue (as Amber)                             │
│      • No permit → fail (policy drift)                              │
│                                                                     │
│   3. POST-APPROVAL AMBER                                                │
│      • Run inspector chain again                                    │
│      • Compare output hash to stored hash                           │
│      • If different: transform drift                                │
│                                                                     │
│   4. UPSTREAM FORWARD                                               │
│      • Send final request to MCP server                             │
│      • Apply execution timeout                                      │
│      • Handle response or error                                     │
│                                                                     │
│   Output: PipelineResult (Success or Failure)                       │
└─────────────────────────────────────────────────────────────────────┘
```

- **F-002.1:** Execute all phases in order
- **F-002.2:** Fail fast on any phase failure
- **F-002.3:** Record failure stage for debugging
- **F-002.4:** Mark retriable errors appropriately

### F-003: Approval Validation

- **F-003.1:** Check approval validity window
- **F-003.2:** Verify request hash matches stored
- **F-003.3:** Verify approval references correct task
- **F-003.4:** Return clear error on validation failure

### F-004: Policy Re-evaluation

- **F-004.1:** Include ApprovalGrant in policy context
- **F-004.2:** Any permit (Green/Amber/Approval) allows execution
- **F-004.3:** No permit means policy drift (fail)
- **F-004.4:** Log and metric policy drift events

### F-005: Post-Approval Amber Phase

- **F-005.1:** Run same inspector chain as Pre-Approval
- **F-005.2:** Compare output hash to stored hash
- **F-005.3:** In strict mode, fail on drift
- **F-005.4:** In permissive mode, log and continue
- **F-005.5:** Rejection in Post-Approval fails the task

### F-006: Upstream Forward

- **F-006.1:** Apply execution timeout
- **F-006.2:** Handle upstream errors
- **F-006.3:** Return tool result on success

### F-007: Pipeline Orchestration

Full flow when approval decision is made:

1. Pre-Approval Amber → Task Creation → Approval Request
2. (Wait for approval)
3. Approval → Validation → Re-eval → Post-Amber → Forward → Result

- **F-007.1:** Orchestrate Pre-Approval → Task Creation → Approval Request
- **F-007.2:** Orchestrate Approval → Execution → Result Storage
- **F-007.3:** Handle errors at each stage appropriately

## 8. Non-Functional Requirements

### NFR-001: Observability

**Metrics:**
```
pipeline_pre_approval_duration_seconds
pipeline_pre_approval_result{result="passed|rejected"}
pipeline_execution_duration_seconds
pipeline_execution_result{result="success|validation_failed|policy_drift|amber_rejected|upstream_error"}
pipeline_transform_drift_total{mode="strict|permissive"}
pipeline_policy_drift_total
```

**Logging:**
```json
{"level":"info","event":"pre_approval_start","task_id":"abc-123","tool":"delete_user"}
{"level":"info","event":"pre_approval_complete","task_id":"abc-123","inspectors_run":3}
{"level":"info","event":"execution_start","task_id":"abc-123"}
{"level":"info","event":"execution_complete","task_id":"abc-123","result":"success"}
{"level":"warn","event":"transform_drift","task_id":"abc-123","mode":"permissive"}
{"level":"warn","event":"policy_drift","task_id":"abc-123"}
```

### NFR-002: Performance

| Metric | Target |
|--------|--------|
| Pre-Approval Amber latency | < 50ms (P99) |
| Validation latency | < 5ms |
| Policy re-evaluation latency | < 5ms |
| Post-Approval Amber latency | < 50ms (P99) |
| Total execution overhead | < 100ms (excluding upstream) |

### NFR-003: Reliability

- Pipeline must not leave task in inconsistent state
- Failures must be clearly attributed to stage
- Transform drift must never silently change executed request in strict mode

## 9. Verification Plan

### 9.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Pre-Approval passes | Task created with transformed request | EC-PIP-001 |
| Pre-Approval rejects | No task created, error returned | EC-PIP-002 |
| Approval valid | Execution proceeds | EC-PIP-003 |
| Approval expired | Task failed with ApprovalTimeout | EC-PIP-004 |
| Hash mismatch | Task failed with IntegrityViolation | EC-PIP-005 |
| Policy still permits | Execution proceeds | EC-PIP-006 |
| Policy now denies | Task failed with PolicyDrift | EC-PIP-007 |
| Post-Approval same output | Execution proceeds | EC-PIP-008 |
| Post-Approval different (strict) | Task failed with TransformDrift | EC-PIP-009 |
| Post-Approval different (permissive) | Execution proceeds with warning | EC-PIP-010 |
| Post-Approval rejects | Task failed | EC-PIP-011 |
| Upstream success | Task completed with result | EC-PIP-012 |
| Upstream timeout | Task failed with UpstreamError | EC-PIP-013 |
| Upstream error | Task failed with UpstreamError | EC-PIP-014 |

### 9.2 Assertions

**Unit Tests:**
- `test_approval_validation_expired` — Expired approval fails
- `test_approval_validation_hash_mismatch` — Hash mismatch fails
- `test_policy_reevaluation_permitted` — Permitted continues
- `test_policy_reevaluation_denied` — Denied fails as drift
- `test_transform_drift_strict` — Strict mode fails on drift
- `test_transform_drift_permissive` — Permissive mode continues

**Integration Tests:**
- `test_full_pipeline_success` — All phases succeed
- `test_pipeline_inspector_rejection` — Inspector rejection handled
- `test_pipeline_upstream_timeout` — Timeout handled correctly

## 10. Implementation Reference

### Pipeline Implementation

```rust
pub struct ApprovalPipeline {
    inspectors: Vec<Arc<dyn Inspector>>,
    policy_engine: Arc<dyn PolicyEngine>,
    upstream_client: Arc<UpstreamClient>,
    config: PipelineConfig,
}

pub struct PipelineConfig {
    pub approval_validity: Duration,
    pub execution_timeout: Duration,
    pub transform_drift_mode: TransformDriftMode,
}

#[derive(Clone, Copy)]
pub enum TransformDriftMode {
    Strict,
    Permissive,
}
```

### Request Hashing

```rust
fn hash_request(request: &ToolCallRequest) -> String {
    use sha2::{Sha256, Digest};
    
    let canonical = serde_json::json!({
        "name": request.name,
        "arguments": request.arguments,
    });
    
    let bytes = serde_json::to_vec(&canonical).unwrap();
    let hash = Sha256::digest(&bytes);
    hex::encode(hash)
}
```

### Anti-Patterns to Avoid

- **❌ Skipping Post-Approval Amber:** Always re-validate, even if Pre-Approval passed
- **❌ Ignoring transform drift:** Always detect, even in permissive mode
- **❌ Silent policy changes:** Log and metric policy drift
- **❌ Partial execution state:** Use transactions or cleanup on failure
- **❌ Missing stage attribution:** Always record which stage failed

## 11. Definition of Done

- [ ] Pre-Approval Amber phase implemented
- [ ] Approval validation (expiry, hash, task ID)
- [ ] Policy re-evaluation with approval context
- [ ] Post-Approval Amber phase implemented
- [ ] Transform drift detection (strict and permissive modes)
- [ ] Upstream forwarding with timeout
- [ ] Result/failure handling and task state updates
- [ ] Pipeline orchestration for full flow
- [ ] Metrics for all phases
- [ ] All edge cases (EC-PIP-001 to EC-PIP-014) covered
- [ ] Performance targets met