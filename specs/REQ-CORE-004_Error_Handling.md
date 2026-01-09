# REQ-CORE-004: Error Handling

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-CORE-004` |
| **Title** | Error Handling |
| **Type** | Core Mechanic |
| **Status** | Draft |
| **Priority** | **High** |
| **Tags** | `#errors` `#json-rpc` `#red-path` `#reliability` |

## 1. Context & Decision Rationale

This requirement defines how ThoughtGate handles and communicates errors. Proper error handling is critical for:

1. **Debuggability:** Agents and operators need clear error messages
2. **Reliability:** Graceful degradation rather than crashes
3. **Security:** Don't leak internal details in error messages
4. **Protocol compliance:** JSON-RPC 2.0 error format

**Red Path Context:**
The "Red Path" is when the Cedar policy engine denies a request. This is one category of error, but this requirement covers ALL error scenarios—policy denial, upstream failures, internal errors, and protocol violations.

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-CORE-003 | **Receives from** | Parse errors, upstream errors |
| REQ-POL-001 | **Receives from** | Policy denial decisions |
| REQ-GOV-001 | **Receives from** | Task errors (not found, expired, etc.) |
| REQ-GOV-002 | **Receives from** | Execution pipeline failures |
| All | **Provides to** | Standardized error formatting |

## 3. Intent

The system must:
1. Classify all error conditions into well-defined categories
2. Map errors to appropriate JSON-RPC error codes
3. Provide actionable error messages (without leaking sensitive data)
4. Log errors with full context for debugging
5. Track error metrics for observability
6. Support error recovery where possible

## 4. Scope

### 4.1 In Scope
- JSON-RPC 2.0 error response formatting
- Error classification and categorization
- Error code assignment
- Error logging with context
- Error metrics
- Red Path (policy denial) handling
- Upstream error handling
- Internal error handling
- Panic recovery

### 4.2 Out of Scope
- Business logic for when to error (defined by other REQs)
- Retry logic (handled by callers or specific REQs)
- Circuit breakers (deferred to future version)

## 5. Constraints

### 5.1 JSON-RPC 2.0 Error Codes

**Standard Codes (MUST use):**
| Code | Message | Meaning |
|------|---------|---------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Not valid JSON-RPC |
| -32601 | Method not found | Method doesn't exist |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal server error |

**Server Error Range (MUST use for custom errors):**
| Range | Usage |
|-------|-------|
| -32000 to -32099 | Server errors (ThoughtGate-defined) |

### 5.2 ThoughtGate Custom Error Codes

| Code | Name | Meaning |
|------|------|---------|
| -32000 | Upstream Connection Failed | Cannot connect to MCP server |
| -32001 | Upstream Timeout | MCP server didn't respond in time |
| -32002 | Upstream Error | MCP server returned error |
| -32003 | Policy Denied | Cedar policy rejected request |
| -32004 | Task Not Found | Invalid task ID |
| -32005 | Task Expired | Task TTL exceeded |
| -32006 | Task Cancelled | Task was cancelled |
| -32007 | Approval Rejected | Human rejected the request |
| -32008 | Approval Timeout | Approval window expired |
| -32009 | Rate Limited | Too many requests |
| -32010 | Inspection Failed | Amber path inspector rejected |
| -32011 | Policy Drift | Policy changed, denying approved request |
| -32012 | Transform Drift | Request changed during approval |
| -32013 | Service Unavailable | ThoughtGate overloaded |

### 5.3 Error Message Guidelines

**DO:**
- Be specific about what failed
- Suggest remediation when possible
- Include request correlation ID
- Use consistent terminology

**DON'T:**
- Expose internal implementation details
- Include stack traces in responses
- Reveal policy rules or Cedar internals
- Include sensitive data (arguments, credentials)

## 6. Interfaces

### 6.1 Input: Error Conditions

```rust
/// All error types that can occur in ThoughtGate
pub enum ThoughtGateError {
    // Protocol errors (from REQ-CORE-003)
    ParseError { details: String },
    InvalidRequest { details: String },
    MethodNotFound { method: String },
    InvalidParams { details: String },
    
    // Upstream errors (from REQ-CORE-003)
    UpstreamConnectionFailed { url: String, reason: String },
    UpstreamTimeout { url: String, timeout_secs: u64 },
    UpstreamError { code: i32, message: String },
    
    // Policy errors (from REQ-POL-001)
    PolicyDenied { tool: String, reason: Option<String> },
    // Note: ApprovalAutoUpgrade removed - not SEP-1686 compliant
    // v0.1 uses blocking mode; v0.2+ uses proper SEP-1686 task response
    
    // Task errors (from REQ-GOV-001) - v0.2+
    TaskNotFound { task_id: String },
    TaskExpired { task_id: String },
    TaskCancelled { task_id: String },
    
    // Approval errors (from REQ-GOV-002, REQ-GOV-003)
    ApprovalRejected { tool: String, rejected_by: Option<String> },  // v0.1: blocking mode rejection
    ApprovalTimeout { tool: String, timeout_secs: u64 },             // v0.1: blocking mode timeout
    
    // Pipeline errors (from REQ-GOV-002)
    InspectionFailed { inspector: String, reason: String },
    PolicyDrift { task_id: String },
    TransformDrift { task_id: String },
    
    // Operational errors
    RateLimited { retry_after_secs: Option<u64> },
    ServiceUnavailable { reason: String },
    InternalError { correlation_id: String },
}
```

### 6.2 Output: JSON-RPC Error Response

```rust
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<ErrorData>,
}

pub struct ErrorData {
    pub correlation_id: String,
    pub error_type: String,
    pub details: Option<serde_json::Value>,
    pub retry_after: Option<u64>,
}
```

**Example Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32003,
    "message": "Policy denied: Tool 'delete_user' is not permitted",
    "data": {
      "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
      "error_type": "policy_denied",
      "details": {
        "tool": "delete_user"
      }
    }
  }
}
```

### 6.3 Internal: Error Logging

```rust
pub struct ErrorLog {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub correlation_id: String,
    pub error_type: String,
    pub error_code: i32,
    pub message: String,
    pub context: ErrorContext,
}

pub struct ErrorContext {
    pub method: Option<String>,
    pub task_id: Option<String>,
    pub principal: Option<String>,
    pub upstream_url: Option<String>,
    pub duration_ms: Option<u64>,
    // Internal-only fields (not in response)
    pub stack_trace: Option<String>,
    pub internal_details: Option<String>,
}
```

## 7. Functional Requirements

### F-001: Error Classification

The system MUST classify errors into categories:

| Category | Errors | User Actionable? |
|----------|--------|------------------|
| **Protocol** | Parse, InvalidRequest, MethodNotFound, InvalidParams | Yes (fix request) |
| **Upstream** | ConnectionFailed, Timeout, UpstreamError | Maybe (retry/wait) |
| **Policy** | PolicyDenied | Yes (change request or request different tool) |
| **Task** | NotFound, Expired, Cancelled | Yes (create new task) - v0.2+ |
| **Approval** | Rejected, Timeout | Yes (resubmit for approval) |
| **Pipeline** | InspectionFailed, PolicyDrift, TransformDrift | Maybe (resubmit) |
| **Operational** | RateLimited, ServiceUnavailable | Yes (wait and retry) |
| **Internal** | InternalError | No (contact operator) |

### F-002: Error Code Mapping

```rust
impl ThoughtGateError {
    pub fn to_jsonrpc_code(&self) -> i32 {
        match self {
            // Standard JSON-RPC codes
            Self::ParseError { .. } => -32700,
            Self::InvalidRequest { .. } => -32600,
            Self::MethodNotFound { .. } => -32601,
            Self::InvalidParams { .. } => -32602,
            Self::InternalError { .. } => -32603,
            
            // ThoughtGate custom codes
            Self::UpstreamConnectionFailed { .. } => -32000,
            Self::UpstreamTimeout { .. } => -32001,
            Self::UpstreamError { .. } => -32002,
            Self::PolicyDenied { .. } => -32003,
            Self::TaskNotFound { .. } => -32004,      // v0.2+
            Self::TaskExpired { .. } => -32005,       // v0.2+
            Self::TaskCancelled { .. } => -32006,     // v0.2+
            Self::ApprovalRejected { .. } => -32007,
            Self::ApprovalTimeout { .. } => -32008,
            Self::RateLimited { .. } => -32009,
            Self::InspectionFailed { .. } => -32010,
            Self::PolicyDrift { .. } => -32011,
            Self::TransformDrift { .. } => -32012,
            Self::ServiceUnavailable { .. } => -32013,
            // -32014 reserved (was ApprovalAutoUpgrade, removed as non-SEP-1686-compliant)
        }
    }
}
```

### F-003: Error Message Generation

- **F-003.1:** Generate user-friendly message for each error type
- **F-003.2:** Include tool/method name where relevant
- **F-003.3:** Include task ID for task-related errors
- **F-003.4:** Include retry guidance for retriable errors
- **F-003.5:** Never include sensitive data (arguments, credentials, internal IPs)

**Message Templates:**
```
ParseError:           "Invalid JSON: {details}"
InvalidRequest:       "Invalid JSON-RPC request: {details}"
MethodNotFound:       "Method '{method}' not found"
InvalidParams:        "Invalid parameters: {details}"
UpstreamConnFailed:   "Cannot connect to MCP server"
UpstreamTimeout:      "MCP server did not respond in time"
UpstreamError:        "MCP server error: {message}"
PolicyDenied:         "Policy denied: Tool '{tool}' is not permitted"
TaskNotFound:         "Task '{task_id}' not found"
TaskExpired:          "Task '{task_id}' has expired"
TaskCancelled:        "Task '{task_id}' was cancelled"
ApprovalRejected:     "Request for '{tool}' was rejected during approval"
ApprovalTimeout:      "Approval window expired for '{tool}' after {timeout_secs}s"
RateLimited:          "Too many requests. Retry after {retry_after} seconds"
InspectionFailed:     "Request validation failed: {reason}"
PolicyDrift:          "Policy changed. Request no longer permitted"
TransformDrift:       "Request context changed during approval"
ServiceUnavailable:   "Service temporarily unavailable"
InternalError:        "Internal error. Reference: {correlation_id}"
```

### F-004: Error Response Formatting

```rust
impl ThoughtGateError {
    pub fn to_jsonrpc_error(&self, correlation_id: &str) -> JsonRpcError {
        JsonRpcError {
            code: self.to_jsonrpc_code(),
            message: self.to_message(),
            data: Some(ErrorData {
                correlation_id: correlation_id.to_string(),
                error_type: self.error_type_name(),
                details: self.safe_details(),
                retry_after: self.retry_after(),
            }),
        }
    }
}
```

### F-005: Error Logging

- **F-005.1:** Log ALL errors at appropriate level
- **F-005.2:** Include full context (correlation ID, method, timing)
- **F-005.3:** Include stack trace for Internal errors
- **F-005.4:** Redact sensitive data before logging
- **F-005.5:** Use structured JSON logging format

**Log Levels:**
| Error Category | Log Level |
|----------------|-----------|
| Protocol (client error) | WARN |
| Upstream | WARN |
| Policy denied | INFO (expected behavior) |
| Task lifecycle | INFO |
| Approval lifecycle | INFO |
| Rate limited | WARN |
| Internal | ERROR |

### F-006: Panic Recovery

- **F-006.1:** Catch panics at request handler boundary
- **F-006.2:** Convert panic to InternalError response
- **F-006.3:** Log panic with stack trace at ERROR level
- **F-006.4:** Ensure connection is properly closed
- **F-006.5:** Increment panic metric

```rust
async fn handle_with_panic_recovery(
    request: McpRequest,
) -> JsonRpcResponse {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        handle_request(request)
    }));
    
    match result {
        Ok(response) => response,
        Err(panic_info) => {
            let correlation_id = Uuid::new_v4().to_string();
            tracing::error!(
                correlation_id = %correlation_id,
                panic = ?panic_info,
                "Panic in request handler"
            );
            metrics::increment_counter!("thoughtgate_panics_total");
            
            ThoughtGateError::InternalError { correlation_id }
                .to_jsonrpc_error(&correlation_id)
        }
    }
}
```

### F-007: Error Metrics

All errors MUST be tracked:

```
thoughtgate_errors_total{type="parse_error|policy_denied|...", code="-32700"}
thoughtgate_upstream_errors_total{type="timeout|connection|error"}
thoughtgate_panics_total
```

## 8. Non-Functional Requirements

### NFR-001: Observability

**Metrics:**
```
thoughtgate_errors_total{type, code}
thoughtgate_error_latency_seconds{type}
thoughtgate_panics_total
```

**Logging:**
- Structured JSON format
- Correlation ID in every log entry
- Timing information for debugging

**Tracing:**
- Error span with error details
- Link to parent request span

### NFR-002: Performance

- Error path should not be significantly slower than success path
- Error formatting < 1ms
- No allocations in hot error paths where possible

### NFR-003: Reliability

- Error responses must always be valid JSON-RPC
- Panics must be caught and converted to errors
- Partial responses must never be sent

### NFR-004: Security

- Never expose internal IP addresses
- Never expose Cedar policy details
- Never expose request arguments in error messages
- Never expose stack traces in responses
- Sanitize all user-provided data before logging

## 9. Verification Plan

### 9.1 Edge Case Matrix

| Scenario | Expected Error | Test ID |
|----------|----------------|---------|
| Malformed JSON | -32700 Parse error | EC-ERR-001 |
| Missing `jsonrpc` field | -32600 Invalid Request | EC-ERR-002 |
| Unknown method | -32601 Method not found | EC-ERR-003 |
| Invalid tool params | -32602 Invalid params | EC-ERR-004 |
| Upstream unreachable | -32000 Connection failed | EC-ERR-005 |
| Upstream slow | -32001 Timeout | EC-ERR-006 |
| Upstream 500 | -32002 Upstream error | EC-ERR-007 |
| Cedar policy denies | -32003 Policy denied | EC-ERR-008 |
| Unknown task ID | -32004 Task not found | EC-ERR-009 |
| Expired task | -32005 Task expired | EC-ERR-010 |
| Cancelled task | -32006 Task cancelled | EC-ERR-011 |
| Human rejects | -32007 Approval rejected | EC-ERR-012 |
| Approval window closes | -32008 Approval timeout | EC-ERR-013 |
| Too many requests | -32009 Rate limited | EC-ERR-014 |
| Inspector rejects | -32010 Inspection failed | EC-ERR-015 |
| Policy changed post-approval | -32011 Policy drift | EC-ERR-016 |
| Request changed post-approval | -32012 Transform drift | EC-ERR-017 |
| Max concurrency | -32013 Service unavailable | EC-ERR-018 |
| Handler panic | -32603 Internal error | EC-ERR-019 |

### 9.2 Assertions

**Unit Tests:**
- `test_error_code_mapping` — All errors map to correct codes
- `test_error_message_generation` — Messages are user-friendly
- `test_error_data_sanitization` — No sensitive data leaks
- `test_panic_recovery` — Panics produce valid error response

**Integration Tests:**
- `test_upstream_timeout_error` — Timeout produces correct error
- `test_policy_denied_error` — Cedar denial produces correct error
- `test_error_logging` — Errors logged with full context

**Security Tests:**
- `test_no_internal_ip_leak` — Internal IPs never in response
- `test_no_policy_leak` — Cedar rules never in response
- `test_no_argument_leak` — Request arguments never in error message

## 10. Implementation Reference

### Error Conversion Pattern

```rust
impl From<ThoughtGateError> for JsonRpcResponse {
    fn from(error: ThoughtGateError) -> Self {
        let correlation_id = get_current_correlation_id();
        
        // Log the error
        error.log(&correlation_id);
        
        // Track metric
        metrics::increment_counter!(
            "thoughtgate_errors_total",
            "type" => error.error_type_name(),
            "code" => error.to_jsonrpc_code().to_string()
        );
        
        // Build response
        JsonRpcResponse::error(error.to_jsonrpc_error(&correlation_id))
    }
}
```

### Error Handler Middleware

```rust
pub async fn error_handling_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let correlation_id = request
        .headers()
        .get("x-correlation-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // Store correlation ID for logging
    let _guard = tracing::span!(
        tracing::Level::INFO,
        "request",
        correlation_id = %correlation_id
    ).entered();
    
    let response = next.run(request).await;
    
    response
}
```

### Anti-Patterns to Avoid

- **❌ Generic error messages:** "An error occurred" is useless
- **❌ Exposing internals:** "Cedar policy line 42 failed" reveals too much
- **❌ Silent failures:** Every error must be logged
- **❌ Inconsistent codes:** Same error type must always use same code
- **❌ Missing correlation ID:** Every error response needs one

## 11. Definition of Done

- [ ] All error types defined in `ThoughtGateError` enum
- [ ] Error code mapping implemented and tested
- [ ] Error message templates defined (no sensitive data)
- [ ] JSON-RPC error response formatting working
- [ ] Error logging with full context
- [ ] Panic recovery at handler boundary
- [ ] Metrics for all error types
- [ ] Security review passed (no data leaks)
- [ ] All edge cases (EC-ERR-001 to EC-ERR-019) covered
- [ ] Documentation of all error codes for API consumers