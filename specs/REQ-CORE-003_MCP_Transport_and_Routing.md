# REQ-CORE-003: MCP Transport & Routing

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-CORE-003` |
| **Title** | MCP Transport & Routing |
| **Type** | Core Mechanic |
| **Status** | Draft |
| **Priority** | **Critical** |
| **Tags** | `#mcp` `#transport` `#json-rpc` `#routing` `#upstream` |

## 1. Context & Decision Rationale

This requirement defines the **entry point** for ThoughtGate—how MCP messages are received, parsed, routed, and forwarded. ThoughtGate sits between MCP hosts (agents) and MCP servers (tools), acting as an **Application Layer Gateway** with policy enforcement.

**Protocol Foundation:**
- MCP uses JSON-RPC 2.0 over Streamable HTTP (POST with SSE response) or stdio
- SEP-1686 extends MCP with task-based async execution
- ThoughtGate must be protocol-transparent for non-intercepted methods

**Architectural Position:**
```
┌─────────────┐     ┌─────────────────────────────────────┐     ┌─────────────┐
│  MCP Host   │────▶│           ThoughtGate               │────▶│  MCP Server │
│  (Agent)    │◀────│  [REQ-CORE-003: This Requirement]   │◀────│  (Tools)    │
└─────────────┘     └─────────────────────────────────────┘     └─────────────┘
```

**v0.1 Simplified Model:**
ThoughtGate is an **MCP-specific gateway**, not a transparent HTTP proxy. All inbound MCP requests are parsed for JSON-RPC routing.

| Traffic Type | Handling | Notes |
|--------------|----------|-------|
| Inbound MCP requests | Parse JSON-RPC, route by method | This REQ |
| Upstream responses | Pass through | No inspection in v0.1 |

**v0.1 simplification:** Green Path (REQ-CORE-001) and Amber Path (REQ-CORE-002) are **deferred**. All responses are passed through without streaming or inspection distinction. These will be reintroduced when response inspection or LLM streaming is needed.

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-CORE-001 | **Deferred** | Green Path deferred to v0.2+ |
| REQ-CORE-002 | **Deferred** | Amber Path deferred to v0.2+ |
| REQ-CORE-004 | **Provides to** | Error responses formatted per this spec |
| REQ-CORE-005 | **Coordinates with** | Lifecycle events (startup, shutdown) |
| REQ-POL-001 | **Receives from** | Routing decisions (Forward/Approve/Reject) |
| REQ-GOV-001 | **Provides to** | Task method handling (`tasks/*`) |

## 3. Intent

The system must:
1. Accept MCP connections from hosts via HTTP+SSE
2. Parse JSON-RPC 2.0 messages correctly (requests, responses, notifications, batches)
3. Route MCP methods to appropriate handlers
4. Forward requests to upstream MCP servers
5. Correlate responses back to originating requests
6. Support SEP-1686 task-augmented requests

## 4. Scope

### 4.1 In Scope
- JSON-RPC 2.0 parsing and validation
- MCP method routing
- HTTP+SSE transport (Streamable HTTP)
- Upstream client (connection, forwarding, response handling)
- Request/response correlation
- SEP-1686 `task` parameter detection
- SSE event streaming for notifications

### 4.2 Out of Scope
- Policy evaluation (REQ-POL-001)
- Error formatting details (REQ-CORE-004)
- Health endpoints (REQ-CORE-005)
- Task state management (REQ-GOV-001)
- stdio transport (deferred to future version)
- HTTP/2 and WebSocket upgrades (deferred, see REQ-CORE-001 notes)

## 5. Constraints

### 5.1 Runtime & Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `tokio` | Async runtime | 1.x (rt-multi-thread) |
| `axum` | HTTP server | 0.7.x |
| `reqwest` | Upstream HTTP client | 0.11.x |
| `serde_json` | JSON parsing | 1.x |
| `uuid` | Request ID generation | 1.x |

### 5.2 Protocol Compliance

**JSON-RPC 2.0 Requirements:**
- MUST accept `jsonrpc: "2.0"` field
- MUST handle requests (with `id`), notifications (without `id`), and batches (arrays)
- MUST preserve `id` type (string or integer) in responses
- MUST return proper error codes per JSON-RPC spec

**MCP Streamable HTTP Transport:**
MCP uses "Streamable HTTP" which is POST-based with optional SSE responses:
- Client sends: `POST /mcp/v1` with `Content-Type: application/json`
- Server responds with either:
  - `Content-Type: application/json` for simple responses
  - `Content-Type: text/event-stream` for streaming responses (notifications, progress)
- This is distinct from legacy "GET /sse" handshake patterns
- ThoughtGate MUST support both response types

**MCP Method Requirements:**
- MUST forward unknown methods transparently to upstream
- MUST handle `Content-Type: application/json` for requests
- MUST forward SSE streams from upstream to client without buffering (Green Path)

**SEP-1686 Requirements:**
- MUST detect `task` field in request params
- MUST route task-augmented requests through governance layer
- MUST implement `tasks/*` method family

### 5.3 Connection Management

**Architecture: Single-Port MCP Gateway**
ThoughtGate operates as a dedicated MCP gateway on a single port, not a transparent HTTP proxy:
- All traffic on this port is assumed to be MCP JSON-RPC
- No protocol sniffing or multi-protocol handling required
- For environments requiring both MCP governance and general HTTP proxying, deploy separate instances

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Listen address | `0.0.0.0:8080` | `THOUGHTGATE_LISTEN` |
| Upstream URL | (required) | `THOUGHTGATE_UPSTREAM` |
| Request timeout | 30s | `THOUGHTGATE_REQUEST_TIMEOUT_SECS` |
| Upstream connect timeout | 5s | `THOUGHTGATE_UPSTREAM_CONNECT_TIMEOUT_SECS` |
| Max concurrent requests | 10000 | `THOUGHTGATE_MAX_CONCURRENT_REQUESTS` |
| Keep-alive | 60s | `THOUGHTGATE_KEEPALIVE_SECS` |
| Max request body size | 1MB | `THOUGHTGATE_MAX_REQUEST_BODY_BYTES` |
| **Approval timeout (v0.1)** | 300s (5min) | `THOUGHTGATE_APPROVAL_TIMEOUT_SECS` |

### 5.4 Blocking Approval Mode (v0.1)

**⚠️ Critical: Zombie Execution Prevention**

In v0.1 blocking mode, ThoughtGate holds the HTTP connection open while waiting for approval. This creates a risk of "zombie execution" where:
1. Client times out and closes connection
2. Human approves the request
3. ThoughtGate executes the tool (side effect happens!)
4. ThoughtGate tries to return response → socket closed
5. Client thinks request failed, may retry → **double execution**

**Mitigation: Connection Liveness Check**

Before executing an approved tool, ThoughtGate MUST verify the client connection is still alive:

```rust
// Pseudocode for blocking approval flow
async fn handle_blocking_approval(request: ToolCall, response_tx: Sender) {
    // 1. Send approval request to Slack
    let task_id = create_approval_task(&request).await;
    
    // 2. Wait for approval with timeout
    let approval = tokio::select! {
        result = wait_for_approval(task_id) => result,
        _ = tokio::time::sleep(approval_timeout) => {
            return Err(ApprovalTimeout { tool: request.name });
        }
    };
    
    // 3. CRITICAL: Check connection liveness before execution
    if response_tx.is_closed() {
        tracing::warn!(
            task_id = %task_id,
            tool = %request.name,
            "Approval received but client disconnected - aborting execution"
        );
        metrics::counter!("thoughtgate_zombie_execution_prevented_total").increment(1);
        return; // Do NOT execute - client cannot receive result
    }
    
    // 4. Connection alive - safe to execute
    match approval {
        ApprovalDecision::Approved => {
            let result = execute_tool(&request).await;
            let _ = response_tx.send(result); // May still fail if client disconnects now
        }
        ApprovalDecision::Rejected { by } => {
            return Err(ApprovalRejected { tool: request.name, rejected_by: by });
        }
    }
}
```

**Configuration:**

| Setting | Default | Description |
|---------|---------|-------------|
| `THOUGHTGATE_APPROVAL_TIMEOUT_SECS` | 300 (5 min) | Max time to wait for approval in blocking mode |
| `THOUGHTGATE_APPROVAL_LIVENESS_CHECK` | true | Check connection before execution |

**Behavior Matrix:**

| Scenario | Behavior |
|----------|----------|
| Approval received, client connected | Execute tool, return result |
| Approval received, client disconnected | Log warning, do NOT execute |
| Timeout reached, client connected | Return `-32008 ApprovalTimeout` error |
| Timeout reached, client disconnected | Log, clean up silently |
| Rejection received | Return `-32007 ApprovalRejected` error |

### 5.5 Upstream Client

- **Connection Pooling:** Maintain persistent connections to upstream
- **Retry Policy:** No automatic retry (let caller handle)
- **Timeout:** Per-request timeout, configurable
- **TLS:** Support HTTPS upstreams, verify certificates by default

## 6. Interfaces

### 6.1 Input: Inbound MCP Request

```
POST /mcp/v1 HTTP/1.1
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "delete_user",
    "arguments": { "user_id": "123" },
    "task": { "ttl": 600000 }           // Optional: SEP-1686
  }
}
```

### 6.2 Output: MCP Response

**Success (direct):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { ... }
}
```

**Success (task-augmented, per SEP-1686):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "taskId": "abc-123",
    "status": "working",
    "createdAt": "2025-01-08T10:30:00Z",
    "ttl": 600000,
    "pollInterval": 5000
  }
}
```

**Error:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": { ... }
  }
}
```

### 6.3 Internal: Parsed Request Structure

```rust
pub struct McpRequest {
    pub id: Option<JsonRpcId>,           // None for notifications
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub task_metadata: Option<TaskMetadata>,  // SEP-1686
    
    // Internal tracking
    pub received_at: Instant,
    pub correlation_id: Uuid,
}

pub struct TaskMetadata {
    pub ttl: Option<Duration>,
}

pub enum JsonRpcId {
    String(String),
    Number(i64),
}
```

### 6.4 Internal: Routing Decision

```rust
pub enum RouteTarget {
    /// Forward to policy engine for classification
    PolicyEvaluation { request: McpRequest },

    /// Handle internally (tasks/* methods)
    TaskHandler { method: TaskMethod, request: McpRequest },

    /// Forward directly to upstream (unknown methods)
    PassThrough { request: McpRequest },
}

pub enum TaskMethod {
    Get,      // tasks/get
    Result,   // tasks/result
    List,     // tasks/list
    Cancel,   // tasks/cancel
}

/// v0.1 Simplified Policy Actions
pub enum PolicyAction {
    /// Forward request to upstream immediately
    Forward,
    /// Require approval before forwarding
    Approve { timeout: Duration },
    /// Reject request with error
    Reject { reason: String },
}
```

### 6.5 Errors (Delegated to REQ-CORE-004)

| Scenario | Error Code | Handled By |
|----------|------------|------------|
| Malformed JSON | -32700 | This REQ (parse phase) |
| Invalid JSON-RPC | -32600 | This REQ (validation phase) |
| Method not found | -32601 | REQ-CORE-004 |
| Invalid params | -32602 | REQ-CORE-004 |
| Upstream errors | -32000 to -32099 | REQ-CORE-004 |

## 7. Functional Requirements

### F-001: JSON-RPC Parsing

The transport layer MUST parse incoming JSON according to JSON-RPC 2.0:

- **F-001.1:** Accept single request objects
- **F-001.2:** Accept batch requests (JSON arrays)
- **F-001.3:** Detect notifications (requests without `id`)
- **F-001.4:** Preserve `id` type (string or integer) for response correlation
- **F-001.5:** Reject malformed JSON with error code -32700
- **F-001.6:** Reject invalid JSON-RPC structure with error code -32600
- **F-001.7:** Generate `correlation_id` (UUID v4) for each request

**Correlation ID:**
Every request receives a unique correlation ID that propagates through all components:
- Generated at parse time in MCP Transport layer
- Stored in `McpRequest.correlation_id`
- Included in all log entries for this request
- Passed to Policy Engine, Task Manager, etc.
- Returned in error responses for debugging

### F-002: Method Routing

The transport layer MUST route methods to appropriate handlers:

| Method Pattern | Route To | Notes |
|----------------|----------|-------|
| `tools/call` | Policy Engine | May become task-augmented |
| `tools/list` | Policy Engine | May filter based on policy |
| `tasks/get` | Task Handler | SEP-1686 |
| `tasks/result` | Task Handler | SEP-1686 |
| `tasks/list` | Task Handler | SEP-1686 |
| `tasks/cancel` | Task Handler | SEP-1686 |
| `resources/*` | Policy Engine | Subject to classification |
| `prompts/*` | Policy Engine | Subject to classification |
| `*` (unknown) | Pass Through | Forward to upstream |

### F-003: SEP-1686 Detection

- **F-003.1:** Detect `task` field in request `params`
- **F-003.2:** Extract `ttl` from task metadata
- **F-003.3:** Flag request as task-augmented for downstream processing
- **F-003.4:** Validate task metadata structure

### F-004: Upstream Forwarding

- **F-004.1:** Maintain connection pool to upstream MCP server
- **F-004.2:** Forward requests with original headers (minus hop-by-hop)
- **F-004.3:** Apply configurable timeout per request
- **F-004.4:** Return upstream response to caller
- **F-004.5:** Handle upstream connection failures (delegate to REQ-CORE-004)

### F-005: Response Correlation

- **F-005.1:** Track in-flight requests by correlation ID
- **F-005.2:** Match responses to original requests
- **F-005.3:** Handle response timeout (delegate to REQ-CORE-004)
- **F-005.4:** Support concurrent requests to same upstream

### F-006: SSE Streaming

- **F-006.1:** Support SSE for server-to-client notifications
- **F-006.2:** Forward upstream SSE events to client
- **F-006.3:** Inject ThoughtGate notifications (e.g., `notifications/tasks/status`)
- **F-006.4:** Handle client disconnect (close upstream connection)

### F-007: Batch Request Handling

- **F-007.1:** Parse batch requests (JSON arrays)
- **F-007.2:** Process each request independently through policy evaluation
- **F-007.3:** Aggregate responses into batch response array
- **F-007.4:** Notifications in batch do not produce response entries
- **F-007.5:** **Approval Batch Policy:** If ANY request in batch requires Approval, entire batch is task-augmented

**Approval Batch Behavior:**
```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                     BATCH REQUEST WITH MIXED PATHS                              │
│                                                                                 │
│   Batch: [                                                                      │
│     {id:1, method:"tools/call", params:{name:"read_file"}}     → Green         │
│     {id:2, method:"tools/call", params:{name:"delete_user"}}   → Approval          │
│     {id:3, method:"resources/list"}                            → Amber         │
│   ]                                                                             │
│                                                                                 │
│   Result: Entire batch → Approval (highest restriction wins)                        │
│                                                                                 │
│   Response: {                                                                   │
│     "jsonrpc": "2.0",                                                          │
│     "id": null,                                                                 │
│     "result": {                                                                 │
│       "taskId": "batch-abc-123",                                               │
│       "status": "working",                                                      │
│       "itemCount": 3,                                                           │
│       "approvalRequired": [2]  // Indices requiring Approval                           │
│     }                                                                           │
│   }                                                                             │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

**Rationale:** 
- Prevents partial execution leaving system in inconsistent state
- Human approver sees complete context of agent's intended actions
- Simplifies client handling (all-or-nothing)

## 8. Non-Functional Requirements

### NFR-001: Observability

**Metrics:**
```
mcp_requests_total{method="tools/call|tasks/get|...", status="success|error"}
mcp_request_duration_seconds{method="...", quantile="0.5|0.9|0.99"}
mcp_upstream_requests_total{status="success|error|timeout"}
mcp_upstream_duration_seconds{quantile="..."}
mcp_connections_active
mcp_batch_size_histogram

# v0.1 Blocking Approval Metrics
thoughtgate_blocking_approval_total{result="approved|rejected|timeout|client_disconnected"}
thoughtgate_blocking_approval_duration_seconds{quantile="..."}
thoughtgate_zombie_execution_prevented_total  # Critical safety metric
```

**Logging:**
```json
{
  "level": "info",
  "message": "MCP request received",
  "correlation_id": "uuid",
  "method": "tools/call",
  "has_task_metadata": true,
  "client_ip": "..."
}
```

**Tracing:**
- Span: `mcp.request` with attributes: `method`, `correlation_id`, `task_augmented`
- Child span: `mcp.upstream` for forwarded requests

### NFR-002: Performance

| Metric | Target |
|--------|--------|
| Parse latency (P99) | < 1ms |
| Routing latency (P99) | < 0.5ms |
| Max concurrent requests | 10,000 |
| Memory per request | < 64KB average |

### NFR-003: Reliability

- **Connection pooling:** Reuse upstream connections
- **Backpressure:** Reject new requests when at max concurrency (503)
- **Timeout handling:** Fail fast on slow upstreams

## 9. Verification Plan

### 9.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Valid JSON-RPC request | Parse, route, respond | EC-MCP-001 |
| Malformed JSON | Return -32700 | EC-MCP-002 |
| Invalid JSON-RPC (missing jsonrpc field) | Return -32600 | EC-MCP-003 |
| Notification (no id) | Process, no response | EC-MCP-004 |
| Batch request | Process all, return array | EC-MCP-005 |
| Empty batch | Return -32600 | EC-MCP-006 |
| Unknown method | Forward to upstream | EC-MCP-007 |
| Task-augmented request | Detect metadata, flag request | EC-MCP-008 |
| Upstream timeout | Return timeout error | EC-MCP-009 |
| Upstream connection refused | Return connection error | EC-MCP-010 |
| Max concurrency reached | Return 503 | EC-MCP-011 |
| Client disconnects mid-request | Cancel upstream, cleanup | EC-MCP-012 |
| Integer request ID | Preserve type in response | EC-MCP-013 |
| String request ID | Preserve type in response | EC-MCP-014 |
| SSE stream from upstream | Forward events to client | EC-MCP-015 |

### 9.2 Assertions

**Unit Tests:**
- `test_parse_valid_jsonrpc` — Verify parsing of well-formed requests
- `test_parse_batch_request` — Verify batch handling
- `test_detect_task_metadata` — Verify SEP-1686 detection
- `test_route_tools_call` — Verify routing to policy engine
- `test_route_tasks_get` — Verify routing to task handler
- `test_preserve_id_type` — Verify ID type preservation

**Integration Tests:**
- `test_end_to_end_forward` — Request flows through to upstream
- `test_concurrent_requests` — Multiple requests handled correctly
- `test_upstream_timeout` — Timeout triggers error response
- `test_sse_forwarding` — SSE events reach client

**Load Tests:**
- `bench_parse_throughput` — Target: 100,000 req/s parse rate
- `bench_concurrent_connections` — 10,000 concurrent connections

## 10. Implementation Reference

### Request Handler Pattern

```rust
async fn handle_mcp_request(
    State(app): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // 1. Parse JSON-RPC
    let requests = match parse_jsonrpc(&payload) {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };
    
    // 2. Route each request
    let mut responses = Vec::new();
    for request in requests {
        let response = match route_request(&app, request).await {
            RouteTarget::PolicyEvaluation { request } => {
                app.policy_engine.evaluate_and_execute(request).await
            }
            RouteTarget::TaskHandler { method, request } => {
                app.task_manager.handle(method, request).await
            }
            RouteTarget::PassThrough { request } => {
                app.upstream.forward(request).await
            }
        };
        
        // Only add response if request had an ID (not a notification)
        if request.id.is_some() {
            responses.push(response);
        }
    }
    
    // 3. Return response(s)
    match responses.len() {
        0 => StatusCode::NO_CONTENT.into_response(),
        1 => Json(responses.remove(0)).into_response(),
        _ => Json(responses).into_response(),
    }
}
```

### Upstream Client Pattern

```rust
pub struct UpstreamClient {
    client: reqwest::Client,
    base_url: Url,
    timeout: Duration,
}

impl UpstreamClient {
    pub async fn forward(&self, request: McpRequest) -> Result<JsonRpcResponse, UpstreamError> {
        let response = self.client
            .post(self.base_url.clone())
            .json(&request.to_jsonrpc())
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| self.classify_error(e))?;
        
        let body = response.json().await?;
        Ok(body)
    }
    
    fn classify_error(&self, error: reqwest::Error) -> UpstreamError {
        if error.is_timeout() {
            UpstreamError::Timeout
        } else if error.is_connect() {
            UpstreamError::ConnectionFailed
        } else {
            UpstreamError::Unknown(error.to_string())
        }
    }
}
```

### Anti-Patterns to Avoid

- **❌ Blocking JSON parsing:** Use async-aware parsing, don't block runtime
- **❌ Unbounded request queues:** Enforce max concurrency with semaphore
- **❌ ID type coercion:** Don't convert integer IDs to strings or vice versa
- **❌ Swallowing notifications:** Notifications don't get responses, but still process them
- **❌ Single upstream connection:** Use connection pooling for throughput

## 11. Definition of Done

- [ ] JSON-RPC 2.0 parser implemented (requests, notifications, batches)
- [ ] Method router implemented with correct routing table
- [ ] SEP-1686 task metadata detection working
- [ ] Upstream client with connection pooling
- [ ] SSE streaming support for notifications
- [ ] Request/response correlation working
- [ ] Concurrency limit enforced
- [ ] All edge cases (EC-MCP-001 to EC-MCP-015) covered by tests
- [ ] Metrics and logging implemented
- [ ] Performance targets met (parse < 1ms P99)