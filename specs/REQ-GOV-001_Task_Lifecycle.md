# REQ-GOV-001: Task Lifecycle & SEP-1686 Compliance

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-GOV-001` |
| **Title** | Task Lifecycle & SEP-1686 Compliance |
| **Type** | Governance Component |
| **Status** | Draft |
| **Priority** | **Critical** |
| **Tags** | `#governance` `#tasks` `#sep-1686` `#state-machine` `#async` |

## 1. Context & Decision Rationale

This requirement defines the **task lifecycle management** for ThoughtGate's approval workflows. It implements SEP-1686, the MCP specification for async task execution.

**What is SEP-1686?**
SEP-1686 introduces the "task primitive" to MCP, enabling:
- Deferred result retrieval via polling
- Long-running operations that outlive request/response cycles
- Status tracking for async workflows

**Why Tasks?**
Approval (human or agent-based) can take minutes, hours, or days. Without tasks:
- Agent would block waiting for approval
- Connection timeouts would fail the request
- No way to track approval status

With tasks:
- Agent receives task ID immediately
- Agent polls for status
- Approval happens out-of-band
- Agent retrieves result when ready

**⚠️ v0.1 Limitation: Blocking Mode Only**

SEP-1686 requires that:
1. Server declares `capabilities.tasks.requests.tools.call: true` during initialize
2. Server adds annotations to tools during `tools/list` to indicate async support
3. Client includes `task` field in request to opt-in to async mode

**v0.1 does NOT implement full SEP-1686.** Instead:
- ThoughtGate uses **blocking mode**: holds HTTP connection until approval
- No task capability advertisement
- No tool annotation rewriting
- Works with any client (no SEP-1686 support required)

**v0.2 will implement full SEP-1686** with:
- Task capability declaration during initialize
- Tool annotation rewriting during tools/list
- Task-augmented call handling
- Blocking fallback for legacy clients

See RFC-001 §10.1 for details.

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-CORE-003 | **Receives from** | MCP routing for `tasks/*` methods (v0.2+) |
| REQ-CORE-004 | **Provides to** | Error formatting for task/approval errors |
| REQ-CORE-005 | **Coordinates with** | Shutdown handling for pending approvals |
| REQ-POL-001 | **Receives from** | Approval decisions that trigger blocking/tasks |
| REQ-GOV-002 | **Provides to** | Task state for execution pipeline (v0.2+) |
| REQ-GOV-003 | **Coordinates with** | Approval decisions update task state |

## 3. Intent

**v0.1 (Blocking Mode):**
The system must:
1. Block HTTP connection until approval decision is received
2. Execute tool and return result on approval
3. Return error on rejection or timeout
4. Support configurable approval timeout

**v0.2+ (SEP-1686 Mode):**
The system must:
1. Implement SEP-1686 task state machine
2. Store tasks with request data for later execution
3. Handle `tasks/get`, `tasks/result`, `tasks/list`, `tasks/cancel` methods
4. Manage task TTL and expiration
5. Support concurrent access with proper synchronization
6. Rate limit task creation to prevent abuse
7. Advertise task capability during initialize
8. Rewrite tool annotations during tools/list

## 4. Scope

### 4.1 In Scope
- Task data structure
- Task state machine (agent-visible and internal states)
- In-memory task storage with TTL
- SEP-1686 method implementations
- Task creation from Approval decisions
- Concurrency control (optimistic locking)
- Rate limiting
- Task metrics and logging

### 4.2 Out of Scope
- Approval logic (REQ-GOV-002, REQ-GOV-003)
- Execution pipeline (REQ-GOV-002)
- Persistent storage (deferred to future version)
- Task migration/recovery (deferred to future version)

## 5. Constraints

### 5.1 SEP-1686 Compliance

**Task States (per SEP-1686):**
| State | Meaning | Terminal? |
|-------|---------|-----------|
| `working` | Request is being processed | No |
| `input_required` | Awaiting external input (approval) | No |
| `completed` | Success, result available | Yes |
| `failed` | Error occurred | Yes |
| `cancelled` | Cancelled by client | Yes |

**Additional States (ThoughtGate-specific):**
| State | Meaning | Terminal? | Visible to Agent? |
|-------|---------|-----------|-------------------|
| `rejected` | Approver rejected request | Yes | Yes |
| `expired` | TTL exceeded | Yes | Yes |

### 5.2 Configuration

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Default TTL | 600s (10 min) | `THOUGHTGATE_TASK_DEFAULT_TTL_SECS` |
| Maximum TTL | 86400s (24 hr) | `THOUGHTGATE_TASK_MAX_TTL_SECS` |
| Cleanup interval | 60s | `THOUGHTGATE_TASK_CLEANUP_INTERVAL_SECS` |
| Max pending per principal | 10 | `THOUGHTGATE_TASK_MAX_PENDING_PER_PRINCIPAL` |
| Max pending global | 1000 | `THOUGHTGATE_TASK_MAX_PENDING_GLOBAL` |

### 5.3 Storage Constraints (v0.1)

- In-memory storage only (no persistence)
- Tasks lost on restart (acceptable for v0.1)
- Memory limit: ~2KB per task average

### 5.4 State Persistence Risk (CRITICAL)

**⚠️ v0.1 Limitation: Volatile Task State**

ThoughtGate sidecars in Kubernetes are ephemeral. If the Application Pod restarts (crash, deployment, scaling), the sidecar restarts and **all in-memory task state is lost**.

**Impact:**
- All pending Approval approvals are lost
- Active tasks in `working` or `input_required` state disappear
- Agents polling for task status will receive `404 Not Found`

**Client Expectations:**
Clients (agents) MUST handle task lookup failures gracefully:
1. On `404 Not Found` for `tasks/get` or `tasks/result`, assume task was lost
2. Retry the original `tools/call` request to create a new task
3. Implement idempotency keys if tool operations must not be duplicated

**Mitigation Strategies (Future):**
| Version | Strategy | Trade-off |
|---------|----------|-----------|
| v0.1 | Accept data loss | Simple, fast |
| future version | Redis-backed store | Adds dependency, survives restarts |
| future version | PostgreSQL + WAL | Full durability, complex |

**Operational Guidance:**
- Monitor `tasks_pending` metric before deployments
- Consider "drain" period before rolling updates
- Alert on high pending task counts during restarts

## 6. Interfaces

### 6.1 Task Structure

```rust
pub struct Task {
    // Identity
    pub id: TaskId,                              // UUID v4
    
    // Request Data
    pub original_request: ToolCallRequest,       // What agent sent
    pub pre_approval_transformed: ToolCallRequest,   // After Pre-Approval Amber
    pub request_hash: String,                    // SHA256 for integrity
    
    // Principal
    pub principal: Principal,                    // Who made the request
    
    // Timing
    pub created_at: DateTime<Utc>,
    pub ttl: Duration,
    pub expires_at: DateTime<Utc>,
    pub poll_interval: Duration,                 // Suggested poll frequency
    
    // State
    pub status: TaskStatus,
    pub status_message: Option<String>,
    pub transitions: Vec<TaskTransition>,        // Audit trail
    
    // Approval (populated when approved/rejected)
    pub approval: Option<ApprovalRecord>,
    
    // Result (populated when terminal)
    pub result: Option<ToolCallResult>,
    pub failure: Option<FailureInfo>,
}

pub struct TaskId(pub Uuid);

pub struct ToolCallRequest {
    pub name: String,
    pub arguments: serde_json::Value,
    pub mcp_request_id: JsonRpcId,
}

pub struct ApprovalRecord {
    pub decision: ApprovalDecision,
    pub decided_by: String,
    pub decided_at: DateTime<Utc>,
    pub approval_valid_until: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

pub enum ApprovalDecision {
    Approved,
    Rejected { reason: Option<String> },
}

pub struct FailureInfo {
    pub stage: FailureStage,
    pub reason: String,
    pub retriable: bool,
}

pub enum FailureStage {
    PreHitlInspection,
    ApprovalTimeout,
    ApprovalRejected,
    PolicyDrift,
    PostHitlInspection,
    TransformDrift,
    UpstreamError,
    ServiceShutdown,
}

pub struct TaskTransition {
    pub from: TaskStatus,
    pub to: TaskStatus,
    pub at: DateTime<Utc>,
    pub reason: Option<String>,
}
```

### 6.2 Task Status

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    // Non-Terminal (agent keeps polling)
    Working,
    InputRequired,
    
    // Internal Transitional (not exposed to agent)
    Executing,
    
    // Terminal (agent retrieves result or error)
    Completed,
    Failed,
    Rejected,
    Cancelled,
    Expired,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Rejected | 
            Self::Cancelled | Self::Expired
        )
    }
    
    pub fn is_agent_visible(&self) -> bool {
        !matches!(self, Self::Executing)
    }
    
    /// Convert to SEP-1686 status string
    pub fn to_sep1686(&self) -> &'static str {
        match self {
            Self::Working | Self::Executing => "working",
            Self::InputRequired => "input_required",
            Self::Completed => "completed",
            Self::Failed | Self::Expired => "failed",
            Self::Rejected => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}
```

### 6.3 SEP-1686 Method Interfaces

**tasks/get:**
```rust
pub struct TasksGetRequest {
    pub task_id: TaskId,
}

pub struct TasksGetResponse {
    pub task_id: TaskId,
    pub status: String,                      // SEP-1686 status
    pub status_message: Option<String>,
    pub created_at: String,                  // ISO 8601
    pub ttl: u64,                            // milliseconds
    pub poll_interval: Option<u64>,          // milliseconds
}
```

**tasks/result:**
```rust
pub struct TasksResultRequest {
    pub task_id: TaskId,
}

pub type TasksResultResponse = ToolCallResult;
```

**tasks/list:**
```rust
pub struct TasksListRequest {
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

pub struct TasksListResponse {
    pub tasks: Vec<TaskSummary>,
    pub next_cursor: Option<String>,
}

pub struct TaskSummary {
    pub task_id: TaskId,
    pub status: String,
    pub created_at: String,
    pub tool_name: String,
}
```

**tasks/cancel:**
```rust
pub struct TasksCancelRequest {
    pub task_id: TaskId,
}

pub struct TasksCancelResponse {
    pub task_id: TaskId,
    pub status: String,
}
```

### 6.4 Task Manager Interface

```rust
#[async_trait]
pub trait TaskManager: Send + Sync {
    /// Create a new task for approval workflow
    async fn create(
        &self,
        original_request: ToolCallRequest,
        transformed_request: ToolCallRequest,
        principal: Principal,
        ttl: Duration,
    ) -> Result<Task, TaskError>;
    
    /// Get task by ID (SEP-1686: tasks/get)
    async fn get(&self, task_id: &TaskId) -> Result<Task, TaskError>;
    
    /// Get task result, blocks if not terminal (SEP-1686: tasks/result)
    async fn get_result(
        &self,
        task_id: &TaskId,
        timeout: Duration,
    ) -> Result<ToolCallResult, TaskError>;
    
    /// List tasks with pagination (SEP-1686: tasks/list)
    async fn list(
        &self,
        principal: &Principal,
        cursor: Option<String>,
        limit: usize,
    ) -> Result<TasksListResponse, TaskError>;
    
    /// Cancel a task (SEP-1686: tasks/cancel)
    async fn cancel(&self, task_id: &TaskId) -> Result<Task, TaskError>;
    
    /// Transition task state (internal, with optimistic locking)
    async fn transition(
        &self,
        task_id: &TaskId,
        expected_status: TaskStatus,
        new_status: TaskStatus,
        reason: Option<String>,
    ) -> Result<Task, TaskError>;
    
    /// Record approval decision (called by REQ-GOV-003)
    async fn record_approval(
        &self,
        task_id: &TaskId,
        decision: ApprovalDecision,
        decided_by: String,
    ) -> Result<Task, TaskError>;
    
    /// Store execution result (called by REQ-GOV-002)
    async fn complete(
        &self,
        task_id: &TaskId,
        result: ToolCallResult,
    ) -> Result<Task, TaskError>;
    
    /// Mark task as failed (called by REQ-GOV-002)
    async fn fail(
        &self,
        task_id: &TaskId,
        failure: FailureInfo,
    ) -> Result<Task, TaskError>;
}
```

### 6.5 Errors

```rust
pub enum TaskError {
    NotFound { task_id: TaskId },
    Expired { task_id: TaskId },
    AlreadyTerminal { task_id: TaskId, status: TaskStatus },
    ConcurrentModification { task_id: TaskId, expected: TaskStatus, actual: TaskStatus },
    RateLimited { principal: String, retry_after: Duration },
    CapacityExceeded,
    ResultNotReady { task_id: TaskId },
    Internal { details: String },
}
```

## 7. Functional Requirements

### F-001: Task State Machine

```
┌─────────────────────────────────────────────────────────────────┐
│                      TASK STATE MACHINE                          │
│                                                                  │
│                        ┌─────────┐                               │
│            ┌──────────▶│ Working │──────────┐                    │
│            │           └────┬────┘          │                    │
│         (create)            │               │                    │
│                             ▼               │                    │
│                    ┌────────────────┐       │                    │
│                    │ InputRequired  │       │                    │
│                    │(await approval)│       │                    │
│                    └───────┬────────┘       │                    │
│                            │                │                    │
│           ┌────────────────┼────────────────┼──────────┐        │
│           ▼                ▼                ▼          ▼        │
│     ┌──────────┐    ┌──────────┐    ┌──────────┐ ┌─────────┐   │
│     │ Approved │    │ Rejected │    │ Cancelled│ │ Expired │   │
│     │(internal)│    │(terminal)│    │(terminal)│ │(terminal)│   │
│     └────┬─────┘    └──────────┘    └──────────┘ └─────────┘   │
│          │                                                       │
│          ▼                                                       │
│    ┌───────────┐                                                │
│    │ Executing │ (internal, not visible to agent)               │
│    └─────┬─────┘                                                │
│          │                                                       │
│    ┌─────┴─────┐                                                │
│    ▼           ▼                                                │
│ ┌───────────┐ ┌───────────┐                                     │
│ │ Completed │ │  Failed   │                                     │
│ │ (terminal)│ │ (terminal)│                                     │
│ └───────────┘ └───────────┘                                     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Valid Transitions:**
| From | To | Trigger |
|------|----|---------|
| `Working` | `InputRequired` | Pre-Approval complete, awaiting approval |
| `Working` | `Failed` | Pre-Approval inspection failed |
| `InputRequired` | `Executing` | Approval received |
| `InputRequired` | `Rejected` | Approver rejected |
| `InputRequired` | `Cancelled` | Agent cancelled |
| `InputRequired` | `Expired` | TTL exceeded |
| `Executing` | `Completed` | Execution succeeded |
| `Executing` | `Failed` | Execution failed |

- **F-001.1:** Enforce valid transitions only
- **F-001.2:** Reject invalid transitions with error
- **F-001.3:** Record all transitions in audit trail
- **F-001.4:** Terminal states are immutable

### F-002: Task Creation

- **F-002.1:** Generate unique task ID (UUID v4)
- **F-002.2:** Store original and transformed request
- **F-002.3:** Compute request hash (SHA256) for integrity verification
- **F-002.4:** Apply TTL bounds (min 60s, max configurable)
- **F-002.5:** Check rate limits before creation
- **F-002.6:** Check global capacity before creation
- **F-002.7:** Compute initial poll_interval based on TTL

**Poll Interval Calculation:**
The `poll_interval` returned to agents is computed dynamically based on remaining TTL:

```rust
fn compute_poll_interval(remaining_ttl: Duration) -> Duration {
    let secs = remaining_ttl.as_secs();
    match secs {
        0..=60 => Duration::from_secs(2),      // Last minute: poll every 2s
        61..=300 => Duration::from_secs(5),    // Last 5 min: poll every 5s
        301..=900 => Duration::from_secs(10),  // Last 15 min: poll every 10s
        _ => Duration::from_secs(30),          // Otherwise: poll every 30s
    }
}
```

**Configuration:**
| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Min poll interval | `2s` | `THOUGHTGATE_TASK_MIN_POLL_INTERVAL_SECS` |
| Max poll interval | `30s` | `THOUGHTGATE_TASK_MAX_POLL_INTERVAL_SECS` |

**Rationale:** More frequent polling as expiration approaches allows agents to react quickly to approval decisions while reducing load during long waits.

### F-003: SEP-1686 tasks/get

- **F-003.1:** Return current task status
- **F-003.2:** Map internal states to agent-visible states (`Executing` → `Working`)
- **F-003.3:** Include timing information (created_at, ttl, poll_interval)
- **F-003.4:** Return error -32004 for unknown task ID

### F-004: SEP-1686 tasks/result

- **F-004.1:** Return result immediately if task completed
- **F-004.2:** Return error with failure info if task failed/rejected/expired
- **F-004.3:** Block and poll if task not terminal (up to timeout)
- **F-004.4:** Return error -32006 (ResultNotReady) if timeout exceeded

### F-005: SEP-1686 tasks/list

- **F-005.1:** Return tasks for the requesting principal only
- **F-005.2:** Support cursor-based pagination
- **F-005.3:** Order by creation time (newest first)
- **F-005.4:** Include only summary information (not full request data)
- **F-005.5:** Default limit: 20, max limit: 100

### F-006: SEP-1686 tasks/cancel

- **F-006.1:** Only cancel tasks in `InputRequired` state
- **F-006.2:** Return error -32007 for terminal tasks
- **F-006.3:** Return error -32007 for `Executing` tasks (too late)
- **F-006.4:** Notify approval adapter to cleanup pending approval
- **F-006.5:** Record cancellation in audit trail

### F-007: Optimistic Locking

- **F-007.1:** Check expected status before transition
- **F-007.2:** Return `ConcurrentModification` error if status changed
- **F-007.3:** Validate transition is legal per state machine
- **F-007.4:** Atomic update (no partial transitions)

### F-008: TTL and Expiration

- **F-008.1:** Run cleanup task periodically (configurable interval, default 60s)
- **F-008.2:** Expire non-terminal tasks where `now > expires_at`
- **F-008.3:** Log each expiration with task_id, tool, and age
- **F-008.4:** Remove terminal tasks after grace period (1 hour after terminal)

### F-009: Rate Limiting

- **F-009.1:** Track pending (non-terminal) tasks per principal
- **F-009.2:** Reject creation if principal exceeds limit
- **F-009.3:** Track total pending tasks globally
- **F-009.4:** Reject creation if global limit exceeded
- **F-009.5:** Return `RateLimited` error with `retry_after` hint

## 8. Non-Functional Requirements

### NFR-001: Observability

**Metrics:**
```
tasks_created_total{tool="delete_user"}
tasks_completed_total{tool="delete_user", outcome="completed|failed|rejected|expired|cancelled"}
tasks_active{status="working|input_required"}
tasks_duration_seconds{stage="total|approval_wait|execution"}
task_transitions_total{from="working", to="input_required"}
task_rate_limited_total{principal="app-xyz"}
```

**Logging:**
```json
{"level":"info","message":"Task created","task_id":"abc-123","tool":"delete_user","principal":"app-xyz"}
{"level":"info","message":"Task status changed","task_id":"abc-123","from":"input_required","to":"executing"}
{"level":"warn","message":"Task expired","task_id":"abc-123","tool":"delete_user","age_seconds":600}
```

### NFR-002: Performance

| Metric | Target |
|--------|--------|
| Task creation | < 1ms |
| Task lookup | < 0.1ms |
| Transition | < 0.5ms |
| Memory per task | < 2KB |
| Max concurrent tasks | 10,000 |

### NFR-003: Reliability

- No data races on concurrent access
- Atomic state transitions
- Graceful handling of storage pressure

## 9. Verification Plan

### 9.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Create task | Returns task with Working status | EC-TASK-001 |
| Get existing task | Returns task status | EC-TASK-002 |
| Get non-existent task | Returns NotFound error | EC-TASK-003 |
| Get result, task completed | Returns result immediately | EC-TASK-004 |
| Get result, task pending | Blocks until complete or timeout | EC-TASK-005 |
| Get result, task failed | Returns failure info | EC-TASK-006 |
| Cancel pending task | Transitions to Cancelled | EC-TASK-007 |
| Cancel completed task | Returns AlreadyTerminal error | EC-TASK-008 |
| Cancel executing task | Returns AlreadyTerminal error | EC-TASK-009 |
| List tasks | Returns principal's tasks only | EC-TASK-010 |
| List with pagination | Returns correct pages | EC-TASK-011 |
| Task expires | Transitions to Expired | EC-TASK-012 |
| Concurrent transition | One succeeds, one gets ConcurrentMod | EC-TASK-013 |
| Rate limit exceeded | Returns RateLimited error | EC-TASK-014 |
| Capacity exceeded | Returns CapacityExceeded error | EC-TASK-015 |
| Shutdown with pending | Tasks marked as Failed (shutdown) | EC-TASK-016 |

### 9.2 Assertions

**Unit Tests:**
- `test_state_machine_valid_transitions` — Valid transitions succeed
- `test_state_machine_invalid_transitions` — Invalid transitions rejected
- `test_optimistic_locking` — Concurrent modification detected
- `test_ttl_expiration` — Tasks expire correctly
- `test_rate_limiting` — Limits enforced per principal
- `test_global_capacity` — Global limits enforced

**Integration Tests:**
- `test_full_task_lifecycle` — Create → InputRequired → Executing → Completed
- `test_task_cancellation` — Cancel while in InputRequired
- `test_concurrent_access` — Multiple clients accessing same task

**Load Tests:**
- `bench_task_creation` — Target: 10,000 tasks/second
- `bench_concurrent_transitions` — 1,000 concurrent transitions

## 10. Implementation Reference

### Task Store

```rust
pub struct InMemoryTaskStore {
    tasks: DashMap<TaskId, Task>,
    by_principal: DashMap<String, Vec<TaskId>>,
    config: TaskStoreConfig,
}

impl InMemoryTaskStore {
    pub fn new(config: TaskStoreConfig) -> Self {
        Self {
            tasks: DashMap::new(),
            by_principal: DashMap::new(),
            config,
        }
    }
    
    pub async fn insert(&self, task: Task) -> Result<(), TaskError> {
        // Check global capacity
        if self.tasks.len() >= self.config.max_global {
            return Err(TaskError::CapacityExceeded);
        }
        
        // Check per-principal limit
        let principal_key = task.principal.app_name.clone();
        let pending_count = self.count_pending_for_principal(&principal_key);
        
        if pending_count >= self.config.max_per_principal {
            return Err(TaskError::RateLimited {
                principal: principal_key,
                retry_after: Duration::from_secs(60),
            });
        }
        
        // Insert
        let task_id = task.id.clone();
        self.tasks.insert(task_id.clone(), task);
        self.by_principal
            .entry(principal_key)
            .or_default()
            .push(task_id);
        
        Ok(())
    }
}
```

### Request Hash

```rust
fn hash_request(request: &ToolCallRequest) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(request.name.as_bytes());
    hasher.update(request.arguments.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}
```

### Anti-Patterns to Avoid

- **❌ Blocking locks:** Use `DashMap` or similar for concurrent access
- **❌ Unbounded storage:** Always enforce limits
- **❌ Lost transitions:** Always record in audit trail
- **❌ Exposing internal states:** Map `Executing` to `Working` for agents
- **❌ Silent expiration:** Always log and metric on expiration

## 11. Definition of Done

- [ ] Task structure defined with all fields
- [ ] State machine implemented with valid transitions only
- [ ] In-memory storage with TTL cleanup
- [ ] `tasks/get` implemented per SEP-1686
- [ ] `tasks/result` implemented with blocking behavior
- [ ] `tasks/list` implemented with pagination
- [ ] `tasks/cancel` implemented with state validation
- [ ] Optimistic locking prevents race conditions
- [ ] Rate limiting enforced (per-principal and global)
- [ ] Metrics for all task operations
- [ ] All edge cases (EC-TASK-001 to EC-TASK-016) covered
- [ ] Performance targets met (< 1ms creation)