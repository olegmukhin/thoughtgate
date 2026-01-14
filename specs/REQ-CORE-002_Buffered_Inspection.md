# REQ-CORE-002: Buffered Inspection (Amber Path)

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-CORE-002` |
| **Title** | Buffered Inspection (Amber Path) |
| **Type** | Core Mechanic |
| **Status** | **DEFERRED (v0.3+)** |
| **Priority** | Low (deferred) |
| **Tags** | `#proxy` `#buffering` `#security` `#inspection` `#redaction` `#deferred` |

> ## ⚠️ DEFERRED TO FUTURE VERSION
>
> **This requirement is deferred from v0.1.** The Amber Path was designed for request/response
> inspection (PII detection, schema validation, transformations), but v0.1 does not implement
> inspection functionality.
>
> **v0.1 Simplification:**
> - All requests are forwarded without inspection
> - All responses are passed through without inspection
> - No buffering distinction between paths
>
> **When to Reintroduce:**
> - When PII detection/redaction is needed
> - When schema validation is required
> - When request/response transformation is needed
> - When pre-approval inspection of tool arguments is needed
>
> **See:** `architecture.md` Section 7.2 (Out of Scope)

## 1. Context & Decision Rationale

This requirement implements the **"Amber Path"** in ThoughtGate's traffic classification system. The Amber Path is used when traffic requires inspection, validation, or transformation before forwarding.

**Traffic Classification:**
| Path | Trigger | Behavior | Requirement |
|------|---------|----------|-------------|
| **Green** | `PolicyDecision::Green` | Zero-copy streaming | REQ-CORE-001 |
| **Amber (This REQ)** | `PolicyDecision::Amber` | Buffered inspection | REQ-CORE-002 |
| **Approval** | `PolicyDecision::Approval` | Human/agent approval | REQ-GOV-001/002/003 |
| **Red** | `PolicyDecision::Red` | Immediate rejection | REQ-CORE-004 |

**When is Amber Path Used?**
- PII detection and redaction
- Schema validation
- Request transformation
- Pre-Approval inspection (before approver sees request)
- Post-Approval inspection (after approval, before execution)

**Design Philosophy:**
Amber Path prioritizes **Safety** and **Visibility** over raw latency. It accumulates the entire body into memory to enable inspection, but uses zero-copy techniques where possible to minimize overhead.

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-POL-001 | **Receives from** | `PolicyDecision::Amber` triggers this path |
| REQ-CORE-003 | **Provides to** | Inspection capability for MCP requests |
| REQ-CORE-004 | **Uses** | Error responses for inspection failures |
| REQ-GOV-002 | **Provides to** | Pre-Approval and Post-Approval inspection phases |

## 3. Intent

The system must:
1. Safely accumulate request/response bodies into memory
2. Enforce size limits to prevent DoS attacks
3. Allow inspection, modification, or rejection by a chain of inspectors
4. Use zero-copy techniques when no modification occurs
5. Handle timeouts to prevent slow-drip attacks

## 4. Scope

### 4.1 In Scope
- Body buffering with size limits
- Inspector trait and chain execution
- Zero-copy forwarding when no modification
- Content-Length recalculation on modification
- Timeout enforcement for entire Amber Path lifecycle
- Concurrency limiting via semaphore
- Compression handling (strip Accept-Encoding)
- Trailer preservation
- Panic safety for inspectors

### 4.2 Out of Scope
- Zero-copy streaming (REQ-CORE-001)
- Policy evaluation (REQ-POL-001)
- Specific inspector implementations (PII detector, schema validator)
- Approval task creation (REQ-GOV-001)

## 5. Constraints

### 5.1 Runtime & Dependencies

| Crate | Purpose | Notes |
|-------|---------|-------|
| `tokio` | Async runtime | rt-multi-thread |
| `hyper` | HTTP implementation | v1.x |
| `bytes` | Zero-copy buffers | `Bytes` type |
| `http-body-util` | Body utilities | `collect()`, `Limited` |
| `thiserror` | Error handling | Inspector errors |

**Safety Requirements:**
- Safe Rust only; no `unsafe` blocks in buffering logic
- All inspectors must be `Send + Sync`

### 5.2 Configuration

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Max concurrent buffers | `100` | `THOUGHTGATE_MAX_CONCURRENT_BUFFERS` |
| Max request buffer | `2MB` | `THOUGHTGATE_REQ_BUFFER_MAX` |
| Max response buffer | `10MB` | `THOUGHTGATE_RESP_BUFFER_MAX` |
| Buffer timeout | `30s` | `THOUGHTGATE_BUFFER_TIMEOUT_SECS` |

### 5.3 Memory Management (CRITICAL)

**Accumulation:**
- Use `http-body-util::BodyExt::collect()` to gather frames efficiently
- Wrap body in `Limited` to enforce size limits before collection

**Zero-Copy Preference:**
- Do NOT allocate new `Vec<u8>` unless modification actually occurs
- If all inspectors return `Approve`, reuse original buffer
- Use `Cow<[u8]>` pattern for inspector chain

**Concurrency Limit:**
- Global semaphore prevents OOM from concurrent buffered requests
- If exhausted, return `503 Service Unavailable` immediately

### 5.4 Compression Handling

To avoid complex decompression logic:
- MUST strip `Accept-Encoding` header from request when entering Amber Path
- If upstream sends compressed data anyway (`Content-Encoding: gzip`), MUST return `502 Bad Gateway`

## 6. Interfaces

### 6.1 Input

```rust
/// Amber Path triggered when policy returns this decision
pub enum PolicyDecision {
    Green,
    Amber,  // This path
    Approval { ... },
    Red { ... },
}

/// Input: HTTP request/response with body to inspect
pub type IncomingRequest = Request<Incoming>;
pub type IncomingResponse = Response<Incoming>;
```

### 6.2 Output

```rust
/// Output: HTTP request/response with potentially modified body
pub type InspectedRequest = Request<Full<Bytes>>;
pub type InspectedResponse = Response<Full<Bytes>>;

/// Or on error/rejection
pub type ErrorResponse = Response<Full<Bytes>>;
```

### 6.3 Inspector Interface

```rust
/// Inspector decision after examining body
#[derive(Debug, Clone)]
pub enum InspectorDecision {
    /// Forward original bytes (zero-copy)
    Approve,
    /// Forward modified bytes (redaction/transformation)
    Modify(Bytes),
    /// Block request with specific status code
    Reject { status: StatusCode, reason: String },
}

/// Context provided to inspectors
pub enum InspectionContext<'a> {
    Request {
        parts: &'a http::request::Parts,
        phase: InspectionPhase,
    },
    Response {
        parts: &'a http::response::Parts,
        phase: InspectionPhase,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum InspectionPhase {
    /// Normal Amber Path inspection
    Standard,
    /// Before approval (approver will see this)
    PreApproval,
    /// After approval (before execution)
    PostApproval,
}

/// Async inspector trait
#[async_trait]
pub trait Inspector: Send + Sync {
    /// Inspect body and return decision
    async fn inspect(
        &self,
        body: &[u8],
        ctx: InspectionContext<'_>,
    ) -> Result<InspectorDecision, InspectorError>;
    
    /// Inspector name for logging/metrics
    fn name(&self) -> &'static str;
    
    /// Inspector behavior type
    fn behavior(&self) -> InspectorBehavior;
}

#[derive(Debug, Clone, Copy)]
pub enum InspectorBehavior {
    /// Can only observe, never modify or reject
    Observe,
    /// Can reject but not modify
    Validate,
    /// Can modify the request
    Transform,
}
```

### 6.4 Errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum AmberPathError {
    #[error("Body exceeds size limit: {size} > {limit}")]
    PayloadTooLarge { size: usize, limit: usize },
    
    #[error("Buffer timeout after {elapsed:?}")]
    Timeout { elapsed: Duration },
    
    #[error("Concurrency limit exceeded")]
    SemaphoreExhausted,
    
    #[error("Inspector '{name}' rejected: {reason}")]
    Rejected { name: String, status: StatusCode, reason: String },
    
    #[error("Inspector '{name}' panicked: {message}")]
    InspectorPanic { name: String, message: String },
    
    #[error("Compressed response not supported")]
    CompressedResponse,
    
    #[error("Internal error: {0}")]
    Internal(String),
}
```

| Error | HTTP Status | Error Code |
|-------|-------------|------------|
| PayloadTooLarge | 413 | -32010 |
| Timeout | 408 | -32001 |
| SemaphoreExhausted | 503 | -32013 |
| Rejected | (from inspector) | -32003 |
| InspectorPanic | 500 | -32603 |
| CompressedResponse | 502 | -32002 |

## 7. Functional Requirements

### F-001: Safe Buffering with Timeout

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        BUFFERED INSPECTION FLOW                                 │
│                                                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                    tokio::time::timeout(30s)                            │  │
│   │                                                                         │  │
│   │   ┌───────────────┐    ┌───────────────┐    ┌───────────────────────┐  │  │
│   │   │    Limited    │    │   collect()   │    │   Inspector Chain     │  │  │
│   │   │   (2MB max)   │───▶│   (buffer)    │───▶│                       │  │  │
│   │   │               │    │               │    │  [PII] → [Schema] →   │  │  │
│   │   └───────────────┘    └───────────────┘    └───────────────────────┘  │  │
│   │                                                                         │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
│   On timeout: Abort with 408 Request Timeout                                   │
│   On size exceeded: Abort with 413 Payload Too Large                           │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

- **F-001.1:** Wrap incoming body in `http-body-util::Limited` to enforce size limits
- **F-001.2:** Wrap entire operation in `tokio::time::timeout`
- **F-001.3:** If timeout expires → abort with `408 Request Timeout`
- **F-001.4:** If size exceeded → abort with `413 Payload Too Large`

### F-002: Fail-Closed & Panic Safety

- **F-002.1:** If buffering fails → return `500 Internal Server Error`
- **F-002.2:** If inspector panics → catch with `std::panic::catch_unwind` or spawn
- **F-002.3:** On panic, log inspector name and stack trace at `ERROR`
- **F-002.4:** Redact payload content from panic logs
- **F-002.5:** Never forward partial data on Amber Path

### F-003: Inspector Chain Execution

```rust
async fn run_inspector_chain(
    inspectors: &[Arc<dyn Inspector>],
    original_bytes: Bytes,
    ctx: InspectionContext<'_>,
) -> Result<Bytes, AmberPathError> {
    // Zero-copy pattern using Cow
    let mut current_bytes = Cow::Borrowed(original_bytes.as_ref());
    let mut modified_storage: Option<Bytes> = None;
    
    for inspector in inspectors {
        // Catch panics
        let result = catch_unwind_async(|| {
            inspector.inspect(current_bytes.as_ref(), ctx.clone())
        }).await;
        
        let decision = match result {
            Ok(Ok(d)) => d,
            Ok(Err(e)) => return Err(AmberPathError::Internal(e.to_string())),
            Err(panic) => return Err(AmberPathError::InspectorPanic {
                name: inspector.name().into(),
                message: format!("{:?}", panic),
            }),
        };
        
        match decision {
            InspectorDecision::Approve => continue,
            InspectorDecision::Modify(new_bytes) => {
                modified_storage = Some(new_bytes.clone());
                current_bytes = Cow::Owned(new_bytes.to_vec());
            }
            InspectorDecision::Reject { status, reason } => {
                return Err(AmberPathError::Rejected {
                    name: inspector.name().into(),
                    status,
                    reason,
                });
            }
        }
    }
    
    // Return final bytes (zero-copy if unmodified)
    Ok(match modified_storage {
        Some(bytes) => bytes,
        None => original_bytes,
    })
}
```

- **F-003.1:** Execute inspectors in registration order
- **F-003.2:** Short-circuit: if any returns `Reject`, halt immediately
- **F-003.3:** If Inspector A returns `Modify(Bytes)`, Inspector B receives new bytes
- **F-003.4:** If all return `Approve`, reuse original buffer (zero-copy)

### F-004: Header Management

- **F-004.1:** If `Modify(Bytes)` returned, update `Content-Length` header
- **F-004.2:** Preserve trailers from chunked streams
- **F-004.3:** Re-attach trailers to outgoing body unless policy drops them

### F-005: Empty Body Handling

- **F-005.1:** If `Content-Length: 0`, skip `collect()` buffering
- **F-005.2:** Still execute inspector chain with empty slice `&[]`
- **F-005.3:** Some inspectors may reject empty bodies based on policy

### F-006: Compression Handling

- **F-006.1:** Strip `Accept-Encoding` header from outgoing request
- **F-006.2:** If upstream response has `Content-Encoding`, return `502`
- **F-006.3:** Log compression rejection at `WARN`

## 8. Non-Functional Requirements

### NFR-001: Observability

**Tracing:**
- Emit OTel span `amber_path.buffer` with attributes:
  - `buffer_size_bytes`
  - `total_duration_ms`
  - `inspectors_run`
  - `decision` (approve/modify/reject)

**Metrics:**
```
amber_path_inspections_total{decision="approve|modify|reject"}
amber_path_errors_total{type="timeout|limit|semaphore|panic|compressed"}
amber_path_buffer_size_bytes (histogram)
amber_path_duration_seconds (histogram)
amber_inspector_duration_seconds{inspector_name} (histogram)
amber_path_buffers_active (gauge)
```

### NFR-002: Performance

| Metric | Target |
|--------|--------|
| Semaphore overhead | < 50µs under contention |
| Inspector chain overhead | < 1ms (empty inspectors) |
| Memory (peak) | ≤ 2.0x payload size |
| Allocations (no modify) | ≤ 1 allocation |

### NFR-003: Reliability

- Fail-closed: errors result in rejection, never partial forward
- Panic-safe: inspector panics don't crash the proxy
- Timeout-protected: slow drip attacks prevented

## 9. Verification Plan

### 9.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Oversized payload | Abort, return 413 Payload Too Large | EC-AMB-001 |
| Slow drip (Slowloris) | Abort after 30s, return 408 | EC-AMB-002 |
| Concurrent buffer limit | 101st request receives 503 | EC-AMB-003 |
| Compressed response | Detect Content-Encoding, return 502 | EC-AMB-004 |
| Length-changing redaction | Replace text, verify new Content-Length | EC-AMB-005 |
| Empty body | Skip buffering, run inspectors with `&[]` | EC-AMB-006 |
| Inspector panic | Catch panic, return 500, log stack trace | EC-AMB-007 |
| All inspectors approve | Zero-copy forward, no allocation | EC-AMB-008 |
| Chain modification | Inspector B sees Inspector A's changes | EC-AMB-009 |
| First inspector rejects | Chain halts, subsequent not called | EC-AMB-010 |

### 9.2 Assertions

**Unit Tests:**
- `test_buffer_limit_enforcement` — Memory doesn't spike beyond limit
- `test_inspector_chain_modification` — A's modification visible to B
- `test_zero_copy_on_approve` — No allocation when all approve
- `test_content_length_update` — Header updated after modification
- `test_panic_handling` — Panic caught, 500 returned

**Integration Tests:**
- `test_slowloris_timeout` — Connection aborted after timeout
- `test_concurrent_limit` — 503 returned when limit exceeded
- `test_compression_rejection` — 502 for gzipped response

**Benchmarks:**
- `bench_semaphore_contention` — < 50µs overhead
- `bench_inspector_chain` — Chain execution overhead

**Fuzzing:**
- `cargo fuzz run amber_path` — No panics on malformed chunks

## 10. Implementation Reference

### Buffered Forwarder

```rust
pub struct BufferedForwarder {
    inspectors: Vec<Arc<dyn Inspector>>,
    semaphore: Arc<Semaphore>,
    config: BufferConfig,
}

pub struct BufferConfig {
    pub max_request_size: usize,
    pub max_response_size: usize,
    pub timeout: Duration,
}

impl BufferedForwarder {
    pub async fn inspect_request(
        &self,
        request: Request<Incoming>,
    ) -> Result<Request<Full<Bytes>>, AmberPathError> {
        // Acquire semaphore
        let _permit = self.semaphore
            .try_acquire()
            .map_err(|_| AmberPathError::SemaphoreExhausted)?;
        
        let (parts, body) = request.into_parts();
        
        // Strip Accept-Encoding
        let mut parts = parts;
        parts.headers.remove(ACCEPT_ENCODING);
        
        // Buffer with limits and timeout
        let bytes = tokio::time::timeout(
            self.config.timeout,
            self.collect_limited(body, self.config.max_request_size),
        )
        .await
        .map_err(|_| AmberPathError::Timeout { 
            elapsed: self.config.timeout 
        })??;
        
        // Run inspector chain
        let ctx = InspectionContext::Request { 
            parts: &parts, 
            phase: InspectionPhase::Standard,
        };
        let final_bytes = run_inspector_chain(&self.inspectors, bytes, ctx).await?;
        
        // Update Content-Length if modified
        parts.headers.insert(
            CONTENT_LENGTH,
            HeaderValue::from(final_bytes.len()),
        );
        
        Ok(Request::from_parts(parts, Full::new(final_bytes)))
    }
    
    async fn collect_limited(
        &self,
        body: Incoming,
        limit: usize,
    ) -> Result<Bytes, AmberPathError> {
        let limited = Limited::new(body, limit);
        let collected = limited
            .collect()
            .await
            .map_err(|e| {
                if e.to_string().contains("limit") {
                    AmberPathError::PayloadTooLarge { size: 0, limit }
                } else {
                    AmberPathError::Internal(e.to_string())
                }
            })?;
        Ok(collected.to_bytes())
    }
}
```

### Anti-Patterns to Avoid

- **❌ Unbounded buffering:** Always use `Limited` wrapper
- **❌ No timeout:** Always wrap in `tokio::time::timeout`
- **❌ Ignoring panics:** Always catch inspector panics
- **❌ Logging payloads:** Never log body content, only metadata
- **❌ Allocating on approve:** Use `Cow` pattern for zero-copy

## 11. Definition of Done

- [x] `BufferedForwarder` implemented with `Limited` wrapper
- [x] Global semaphore for concurrency limiting
- [x] `InspectorDecision` enum and `Inspector` trait defined
- [x] Zero-copy pattern verified (≤ 1 allocation on approve)
- [x] Panic handling verified
- [x] Empty body logic verified
- [x] Timeout enforcement verified
- [x] Content-Length recalculation on modify
- [x] Prometheus metrics and OTel spans
- [x] All edge cases (EC-AMB-001 to EC-AMB-010) covered