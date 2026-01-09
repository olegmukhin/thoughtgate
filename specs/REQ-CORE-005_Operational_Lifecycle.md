# REQ-CORE-005: Operational Lifecycle

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-CORE-005` |
| **Title** | Operational Lifecycle |
| **Type** | Core Mechanic |
| **Status** | Draft |
| **Priority** | **High** |
| **Tags** | `#operations` `#health` `#shutdown` `#kubernetes` `#reliability` |

## 1. Context & Decision Rationale

This requirement defines how ThoughtGate manages its operational lifecycle—startup, health monitoring, and graceful shutdown. Proper lifecycle management is essential for:

1. **Kubernetes integration:** Health probes determine pod scheduling and traffic routing
2. **Zero-downtime deployments:** Graceful shutdown prevents request loss during rollouts
3. **Reliability:** Clear startup sequencing prevents serving before ready
4. **Debuggability:** Health endpoints expose internal state for troubleshooting

**Deployment Context:**
ThoughtGate runs as a sidecar proxy in Kubernetes. It must:
- Start quickly and signal readiness
- Accept traffic only when fully initialized
- Drain connections gracefully on shutdown
- Handle pending Approval tasks appropriately during shutdown

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-CORE-003 | **Coordinates with** | Connection draining on shutdown |
| REQ-POL-001 | **Waits for** | Policy loading before ready |
| REQ-GOV-001 | **Coordinates with** | Task state on shutdown |

## 3. Intent

The system must:
1. Perform orderly startup with clear sequencing
2. Expose health endpoints for orchestration
3. Handle shutdown signals gracefully
4. Drain in-flight requests without data loss
5. Preserve or fail pending Approval tasks appropriately

## 4. Scope

### 4.1 In Scope
- Startup sequencing and initialization
- Health probe endpoints (`/health`, `/ready`)
- SIGTERM/SIGINT handling
- Connection draining
- Request completion during shutdown
- Task handling during shutdown
- Startup/shutdown logging and metrics

### 4.2 Out of Scope
- Task persistence to external storage (deferred to future version)
- Cluster-aware shutdown coordination (deferred to future version)
- Automatic restart/recovery (Kubernetes handles this)

## 5. Constraints

### 5.1 Kubernetes Integration

**Probe Configuration:**
```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 2
  periodSeconds: 5
  failureThreshold: 2
```

**Timing Requirements:**
| Phase | Maximum Duration |
|-------|------------------|
| Startup to healthy | 10 seconds |
| Startup to ready | 15 seconds |
| Graceful shutdown | 30 seconds (configurable) |

### 5.2 Signal Handling

| Signal | Action |
|--------|--------|
| SIGTERM | Begin graceful shutdown |
| SIGINT | Begin graceful shutdown |
| SIGQUIT | Immediate shutdown (dump state) |

### 5.3 Configuration

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Health port | Same as main | `THOUGHTGATE_HEALTH_PORT` |
| Shutdown timeout | 30s | `THOUGHTGATE_SHUTDOWN_TIMEOUT_SECS` |
| Drain timeout | 25s | `THOUGHTGATE_DRAIN_TIMEOUT_SECS` |
| Startup timeout | 15s | `THOUGHTGATE_STARTUP_TIMEOUT_SECS` |
| Require upstream at startup | false | `THOUGHTGATE_REQUIRE_UPSTREAM_AT_STARTUP` |
| Upstream health interval | 30s | `THOUGHTGATE_UPSTREAM_HEALTH_INTERVAL_SECS` |
| Log level | info | `THOUGHTGATE_LOG_LEVEL` |
| Log format | json | `THOUGHTGATE_LOG_FORMAT` |

**Log Levels:**
- `error`: Unrecoverable failures, panics
- `warn`: Recoverable failures, rejections, timeouts
- `info`: Request lifecycle, state transitions
- `debug`: Detailed flow (disabled in production)
- `trace`: Byte-level details (disabled in production)

## 6. Interfaces

### 6.1 Health Endpoint

**Request:**
```
GET /health HTTP/1.1
```

**Response (Healthy):**
```json
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

**Response (Unhealthy):**
```json
HTTP/1.1 503 Service Unavailable
Content-Type: application/json

{
  "status": "unhealthy",
  "reason": "upstream_unreachable",
  "details": "Cannot connect to MCP server"
}
```

### 6.2 Readiness Endpoint

**Request:**
```
GET /ready HTTP/1.1
```

**Response (Ready):**
```json
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status": "ready",
  "checks": {
    "policies_loaded": true,
    "upstream_reachable": true,
    "task_store_initialized": true
  }
}
```

**Response (Not Ready):**
```json
HTTP/1.1 503 Service Unavailable
Content-Type: application/json

{
  "status": "not_ready",
  "checks": {
    "policies_loaded": true,
    "upstream_reachable": false,
    "task_store_initialized": true
  },
  "reason": "upstream_unreachable"
}
```

### 6.3 Internal State

```rust
pub enum LifecycleState {
    Starting,       // Initialization in progress
    Ready,          // Accepting traffic
    ShuttingDown,   // Draining, rejecting new requests
    Stopped,        // Shutdown complete
}

pub struct LifecycleManager {
    state: AtomicCell<LifecycleState>,
    started_at: Instant,
    shutdown_signal: Option<broadcast::Sender<()>>,
    drain_complete: Option<broadcast::Sender<()>>,
    active_requests: AtomicUsize,
    pending_tasks: AtomicUsize,
}
```

## 7. Functional Requirements

### F-001: Startup Sequencing

The system MUST initialize in this order:

```
┌─────────────────────────────────────────────────────────┐
│                    STARTUP SEQUENCE                      │
│                                                          │
│  1. Load configuration                                   │
│     • Parse env vars and config files                    │
│     • Validate required settings                         │
│     • Set state: Starting                                │
│                                                          │
│  2. Initialize observability                             │
│     • Setup logging                                      │
│     • Setup metrics                                      │
│     • Setup tracing                                      │
│                                                          │
│  3. Initialize task store                                │
│     • Create in-memory store                             │
│     • Start TTL cleanup task                             │
│                                                          │
│  4. Load Cedar policies                                  │
│     • Load from ConfigMap/Env/Embedded                   │
│     • Validate against schema                            │
│     • Start hot-reload watcher                           │
│                                                          │
│  5. Connect to upstream                                  │
│     • Verify upstream is reachable                       │
│     • Initialize connection pool                         │
│                                                          │
│  6. Start HTTP server                                    │
│     • Bind to listen address                             │
│     • Health endpoint available immediately              │
│     • Set state: Ready                                   │
│     • Main endpoints accept traffic                      │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

- **F-001.1:** Fail fast if required configuration is missing
- **F-001.2:** Log each startup phase with timing
- **F-001.3:** Health endpoint available as soon as HTTP server starts
- **F-001.4:** Readiness endpoint returns not-ready until all checks pass
- **F-001.5:** Timeout startup if any phase exceeds limit
- **F-001.6:** **Upstream Unavailable Behavior:** By default, start even if upstream is unreachable

**Upstream Unavailable at Startup:**
| Configuration | Behavior | Use Case |
|---------------|----------|----------|
| `REQUIRE_UPSTREAM_AT_STARTUP=false` (default) | Start, mark NOT Ready | Normal: allows pod to start during upstream maintenance |
| `REQUIRE_UPSTREAM_AT_STARTUP=true` | Fail startup | Strict: ensures system only starts if fully functional |

When upstream is unavailable:
- Health probe returns 200 (process is alive)
- Readiness probe returns 503 (cannot serve traffic)
- Kubernetes will not route traffic until upstream recovers
- Background health check retries upstream at `UPSTREAM_HEALTH_INTERVAL_SECS`

### F-002: Health Probe

- **F-002.1:** Return 200 if process is alive and responsive
- **F-002.2:** Return 503 if critical subsystem has failed
- **F-002.3:** Include version and uptime in response
- **F-002.4:** Health check must complete in < 100ms
- **F-002.5:** Health check must not have side effects

**Health Criteria:**
| Check | Failure Action |
|-------|----------------|
| Process responsive | 503 (automatic via no response) |
| Memory pressure | 503 if > 90% memory used |
| Event loop blocked | 503 if health check takes > 1s |

### F-003: Readiness Probe

- **F-003.1:** Return 200 only when ALL checks pass
- **F-003.2:** Return 503 with failed checks in response
- **F-003.3:** Check policy loading status
- **F-003.4:** Check upstream connectivity (cached, not per-request)
- **F-003.5:** Check task store initialization
- **F-003.6:** During shutdown, return 503 (stop receiving traffic)

**Readiness Criteria:**
| Check | How Verified |
|-------|--------------|
| `policies_loaded` | Cedar engine has valid policy set |
| `upstream_reachable` | Last upstream check succeeded (within 30s) |
| `task_store_initialized` | Task store accepting operations |

### F-004: Graceful Shutdown

```
┌─────────────────────────────────────────────────────────┐
│                   SHUTDOWN SEQUENCE                      │
│                                                          │
│  1. Receive SIGTERM/SIGINT                               │
│     • Set state: ShuttingDown                            │
│     • Log shutdown initiated                             │
│                                                          │
│  2. Stop accepting new requests                          │
│     • Readiness probe returns 503                        │
│     • New requests get 503 Service Unavailable           │
│     • K8s stops routing traffic (after probe fails)      │
│                                                          │
│  3. Wait for in-flight requests (drain timeout)          │
│     • Track active request count                         │
│     • Wait until count reaches 0 or timeout              │
│                                                          │
│  4. Handle pending Approval tasks                            │
│     • Option A: Fail pending tasks                       │
│     • Option B: Wait briefly for approvals               │
│     • Log all pending tasks                              │
│                                                          │
│  5. Close connections                                    │
│     • Close upstream connection pool                     │
│     • Close listener                                     │
│                                                          │
│  6. Cleanup                                              │
│     • Flush metrics                                      │
│     • Flush logs                                         │
│     • Set state: Stopped                                 │
│     • Exit with code 0                                   │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

- **F-004.1:** Stop accepting new requests immediately on signal
- **F-004.2:** Allow in-flight requests to complete (up to drain timeout)
- **F-004.3:** Return 503 for requests arriving during shutdown
- **F-004.4:** Log pending request/task counts during drain
- **F-004.5:** Force shutdown if drain timeout exceeded
- **F-004.6:** Exit with code 0 on clean shutdown, non-zero on forced

### F-005: Request Draining

```rust
pub async fn drain_requests(
    manager: &LifecycleManager,
    timeout: Duration,
) -> DrainResult {
    let deadline = Instant::now() + timeout;
    
    loop {
        let active = manager.active_requests.load(Ordering::SeqCst);
        
        if active == 0 {
            return DrainResult::Complete;
        }
        
        if Instant::now() > deadline {
            tracing::warn!(
                active_requests = active,
                "Drain timeout exceeded, forcing shutdown"
            );
            return DrainResult::Timeout { remaining: active };
        }
        
        tracing::info!(
            active_requests = active,
            "Draining requests..."
        );
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
```

### F-006: Approval Task Handling on Shutdown

Pending Approval tasks (status: `input_required`) need special handling:

| Option | Behavior | When to Use |
|--------|----------|-------------|
| **Fail** | Transition to `failed:shutdown` | Default (v0.1) |
| **Wait** | Brief wait for pending approvals | If approvals expected soon |
| **Persist** | Save to external store | future version feature |

**v0.1 Behavior (Fail):**
- On shutdown, find all tasks in `working` or `input_required`
- Transition each to `failed` with reason `service_shutdown`
- Log each failed task with task_id and original tool
- Agent will see failure when polling and can resubmit

### F-007: Upstream Health Check

- **F-007.1:** Periodic health check to upstream (every 30s)
- **F-007.2:** Cache result for readiness probe
- **F-007.3:** Simple connectivity check (not full request)
- **F-007.4:** Update metric on health change

```rust
async fn check_upstream_health(client: &UpstreamClient) -> bool {
    // Simple TCP connect check, or HTTP HEAD if supported
    client.health_check().await.is_ok()
}
```

## 8. Non-Functional Requirements

### NFR-001: Observability

**Metrics:**
```
thoughtgate_lifecycle_state{state="starting|ready|shutting_down"}
thoughtgate_uptime_seconds
thoughtgate_startup_duration_seconds
thoughtgate_shutdown_duration_seconds
thoughtgate_active_requests
thoughtgate_pending_tasks
thoughtgate_upstream_health{status="healthy|unhealthy"}
thoughtgate_drain_timeout_total
```

**Logging:**
```json
{"level":"info","message":"Starting ThoughtGate","version":"0.1.0"}
{"level":"info","message":"Policies loaded","source":"configmap","policy_count":5}
{"level":"info","message":"Upstream connected","url":"http://mcp-server:3000"}
{"level":"info","message":"ThoughtGate ready","startup_duration_ms":1234}
{"level":"info","message":"Shutdown signal received","signal":"SIGTERM"}
{"level":"info","message":"Draining requests","active_requests":5}
{"level":"info","message":"Shutdown complete","duration_ms":2500}
```

### NFR-002: Performance

| Metric | Target |
|--------|--------|
| Startup time | < 10s to ready |
| Health check latency | < 100ms |
| Readiness check latency | < 200ms |
| Shutdown (no pending) | < 5s |

### NFR-003: Reliability

- Health endpoint must never panic
- Shutdown must always complete (forced if needed)
- No resource leaks on restart cycles

## 9. Verification Plan

### 9.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Clean startup | Ready in < 10s | EC-OPS-001 |
| Missing config | Fail fast with clear error | EC-OPS-002 |
| Upstream unreachable at start | Start, but not ready | EC-OPS-003 |
| Policy file missing | Use fallback, log warning | EC-OPS-004 |
| SIGTERM received | Begin graceful shutdown | EC-OPS-005 |
| Requests during shutdown | Return 503 | EC-OPS-006 |
| Drain completes | Exit 0 | EC-OPS-007 |
| Drain timeout | Force exit, log warning | EC-OPS-008 |
| Pending Approval tasks at shutdown | Fail tasks, log | EC-OPS-009 |
| Health check during startup | Return 503 until ready | EC-OPS-010 |
| Upstream becomes unreachable | Readiness fails, health OK | EC-OPS-011 |
| Rapid restart cycles | No resource leaks | EC-OPS-012 |
| SIGQUIT received | Immediate shutdown | EC-OPS-013 |

### 9.2 Assertions

**Unit Tests:**
- `test_startup_sequence_order` — Phases execute in correct order
- `test_readiness_checks` — All checks evaluated correctly
- `test_shutdown_state_transitions` — State machine correct

**Integration Tests:**
- `test_kubernetes_probes` — Probes work with K8s-style requests
- `test_graceful_shutdown` — Requests complete during drain
- `test_drain_timeout` — Forced shutdown after timeout
- `test_task_failure_on_shutdown` — Pending tasks properly failed

**Chaos Tests:**
- `test_rapid_restart_cycles` — 100 start/stop cycles, no leaks
- `test_shutdown_under_load` — Shutdown while handling 1000 req/s

## 10. Implementation Reference

### Lifecycle Manager

```rust
pub struct LifecycleManager {
    state: Arc<AtomicCell<LifecycleState>>,
    shutdown_tx: broadcast::Sender<()>,
    active_requests: Arc<AtomicUsize>,
}

impl LifecycleManager {
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            state: Arc::new(AtomicCell::new(LifecycleState::Starting)),
            shutdown_tx,
            active_requests: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn is_ready(&self) -> bool {
        matches!(self.state.load(), LifecycleState::Ready)
    }
    
    pub fn is_shutting_down(&self) -> bool {
        matches!(
            self.state.load(),
            LifecycleState::ShuttingDown | LifecycleState::Stopped
        )
    }
    
    pub fn begin_shutdown(&self) {
        self.state.store(LifecycleState::ShuttingDown);
        let _ = self.shutdown_tx.send(());
    }
    
    pub fn track_request(&self) -> RequestGuard {
        self.active_requests.fetch_add(1, Ordering::SeqCst);
        RequestGuard {
            counter: Arc::clone(&self.active_requests),
        }
    }
}

pub struct RequestGuard {
    counter: Arc<AtomicUsize>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}
```

### Signal Handler

```rust
async fn setup_signal_handlers(lifecycle: Arc<LifecycleManager>) {
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    
    tokio::select! {
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM");
        }
        _ = sigint.recv() => {
            tracing::info!("Received SIGINT");
        }
    }
    
    lifecycle.begin_shutdown();
}
```

### Anti-Patterns to Avoid

- **❌ Blocking health checks:** Use async, never block
- **❌ Side effects in probes:** Probes must be read-only
- **❌ Ignoring drain timeout:** Always force shutdown eventually
- **❌ Leaking connections:** Close all connections on shutdown
- **❌ Sync shutdown in async context:** Use proper async shutdown

## 11. Definition of Done

- [ ] Startup sequencing implemented with logging
- [ ] `/health` endpoint implemented and tested
- [ ] `/ready` endpoint with all checks
- [ ] SIGTERM/SIGINT handlers installed
- [ ] Request draining with timeout
- [ ] Pending Approval tasks failed on shutdown
- [ ] Metrics for lifecycle events
- [ ] All edge cases (EC-OPS-001 to EC-OPS-013) covered
- [ ] Tested with Kubernetes probe configuration
- [ ] No resource leaks after 100 restart cycles