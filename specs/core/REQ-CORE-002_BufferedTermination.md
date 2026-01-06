# REQ-CORE-002: Buffered Termination Strategy (Amber Path)

| Metadata | Value |
| --- | --- |
| **ID** | `REQ-CORE-002` |
| **Title** | Buffered Termination Strategy (Amber Path) |
| **Type** | Core Mechanic |
| **Status** | Active |
| **Priority** | **High** |
| **Tags** | `#proxy` `#buffering` `#security` `#inspection` `#redaction` |

## 1. Context & Decision Rationale

This requirement implements the **"Amber Path"** in ThoughtGate's traffic classification system.

* **Green Path (`REQ-CORE-001`):** Zero-copy streaming for trusted traffic.
* **Amber Path (This REQ):** Buffered inspection for traffic requiring validation (PII detection, Schema validation).
* **Red Path:** Immediate rejection.

Traffic enters the Amber Path when the Governance Engine (`REQ-SEC-001`) returns `Decision::Inspect`. This mode prioritizes **Safety** and **Visibility** over raw latency.

## 2. Intent

The system must capably switch to a "Termination Mode" where it safely accumulates the entire request or response body into memory. This mechanism must be robust against Denial of Service (DoS) attacks and must allow for the payload to be inspected, modified (redacted), or rejected by a chain of inspectors.

## 3. Constraints

### 3.1 Runtime & Dependencies

* **Runtime:** `tokio` (rt-multi-thread) with `hyper` v1.x and `hyper-util`.
* **Crates:** `bytes`, `http-body-util`, `thiserror`. Optional: `cow-utils`.
* **Safety:** Safe Rust only. No `unsafe` blocks for buffering logic.

### 3.2 Memory Management (CRITICAL)

* **Accumulation:** Use `http-body-util::BodyExt::collect()` to efficiently gather frames.
* **Zero-Copy Preference:** The system MUST NOT allocate a new `Vec<u8>` unless modification (redaction) actually occurs.
* **Concurrency Limit:** To prevent OOM attacks, the system MUST enforce a **Global Semaphore** limiting the number of concurrent buffered connections. If exhausted, return `503 Service Unavailable`.
* **Config Loading:** Limits must be loaded from environment variables **once at startup** (using `lazy_static` or `OnceLock`).
* `THOUGHTGATE_MAX_CONCURRENT_BUFFERS` (default: 100)
* `THOUGHTGATE_REQ_BUFFER_MAX` (default: 2MB)
* `THOUGHTGATE_RESP_BUFFER_MAX` (default: 10MB)
* **`THOUGHTGATE_BUFFER_TIMEOUT_SECS` (default: 30s):** This timeout applies to the **ENTIRE** Amber Path lifecycle.



### 3.3 Compression Handling

To avoid complex decompression logic:

* The proxy **MUST** strip the `Accept-Encoding` header from the Request when entering the Amber Path.
* If the Upstream Server sends compressed data anyway, the proxy **MUST** reject the response with `502 Bad Gateway`.

## 4. Functional Requirements

* **F-001 (Safe Buffering with Timeout):**
* The middleware must wrap the incoming body in `http-body-util::Limited` to enforce size limits.
* **Critical:** The entire buffering operation (`collect()`) and inspection chain MUST be wrapped in `tokio::time::timeout`. If the timeout expires, abort the connection with `408 Request Timeout`.


* **F-002 (Fail-Closed State & Panic Safety):**
* If buffering/inspection fails: Return `500 Internal Server Error` (or `408`/`413` where applicable).
* **Panic Safety:** Panics within inspectors MUST be caught using `std::panic::catch_unwind` or `tokio::task::spawn`.
* **Logging:** On panic, log the inspector name and stack trace at `ERROR` level, but **redact** the payload content.
* **Never** forward partial data on the Amber Path.


* **F-003 (Async Inspector Interface):**
The system must define the following types. Input is `&[u8]` for maximum compatibility.
```rust
#[derive(Debug)]
pub enum Decision {
    Approve,                 // Forward original bytes (Zero Copy)
    Modify(Bytes),           // Forward new bytes (Redaction)
    Reject(StatusCode),      // Block request
}

pub enum InspectionContext<'a> {
    Request(&'a http::request::Parts),
    Response(&'a http::response::Parts),
}

#[async_trait]
pub trait Inspector: Send + Sync {
    async fn inspect(&self, body: &[u8], ctx: InspectionContext<'_>) -> Result<Decision, Error>;
}

```


* **F-004 (Chain Semantics):**
* Inspectors are executed in registration order.
* **Short-Circuit:** If any inspector returns `Reject`, the chain halts immediately.
* **Flow:** If Inspector A returns `Modify(Bytes)`, Inspector B receives the *new* bytes.
* **Zero-Copy:** If all return `Approve`, the original buffer is reused.


* **F-005 (Header & Trailer Management):**
* If `Modify(Bytes)` is returned, the `Content-Length` header MUST be updated.
* **Trailers:** Buffered trailers (common in `Transfer-Encoding: chunked` streams) MUST be preserved and re-attached to the outgoing body unless explicitly dropped by policy.
* **Empty Bodies:** If `Content-Length: 0`, skip the `collect()` buffering step but **MUST** still execute the inspector chain with an empty slice `&[]`.



## 4.5 Non-Functional Requirements (NFRs)

* **NFR-001 (Observability):**
* **Tracing:** Emit OTel span `amber_path.buffer` with attributes: `buffer_size_bytes`, `total_duration_ms`.
* **Granular Metrics:** Emit a histogram `amber_inspector_duration_seconds` labeled by `inspector_name` to identify slow plugins.
* **Counters:** `amber_path_inspections_total{decision="..."}` and `amber_path_errors_total{type="timeout|limit|semaphore|panic"}`.



## 5. Verification Plan

### 5.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
| --- | --- | --- |
| **Oversized Payload** | Abort conn, return `413 Payload Too Large`. | `EC-001` |
| **Slow Drip (Slowloris)** | Abort conn after timeout (30s total), return `408`. | `EC-002` |
| **Memory Exhaustion** | 101st concurrent request receives `503`. | `EC-003` |
| **Compressed Response** | Detect `Content-Encoding: gzip`, return `502`. | `EC-004` |
| **Length-Changing Redaction** | Replace text, verify new `Content-Length`. | `EC-005` |
| **Empty Body** | Skip buffering, run inspectors with `&[]`. | `EC-006` |
| **Inspector Panic** | Catch panic, return `500`, log stack trace. | `EC-007` |

### 5.2 Assertions

* **Unit Test:** `test_buffer_limit_enforcement`
* *Assert:* Memory usage does not spike beyond `MAX_BUFFER + overhead`.


* **Integration Test:** `test_inspector_chain_modification`
* *Assert:* Inspector A's modification is visible to Inspector B.


* **Benchmark:** `bench_semaphore_contention`
* *Goal:* Verify semaphore adds < 50µs overhead under high contention.


* **Fuzzing:** `cargo fuzz run amber_path`
* *Assert:* No panics on malformed chunks.



## 7. Implementation Reference (Hybrid Zero-Copy)

To ensure max performance and zero-copy:

```rust
// Recommended Flow
// 1. Setup pointers.
let mut current_bytes = Cow::Borrowed(original_bytes.as_ref());
// Optimization: Track owned Bytes separately to avoid Vec->Bytes conversion at the end
let mut modified_storage: Option<Bytes> = None;

// Wrap entire loop in timeout
let result = tokio::time::timeout(config.timeout, async {
    for inspector in inspectors {
        // 2. Run Inspector
        match inspector.inspect(current_bytes.as_ref(), &ctx).await? {
            Decision::Approve => continue,
            Decision::Modify(new_bytes) => {
                // Store the efficient Bytes handle
                modified_storage = Some(new_bytes.clone());
                // Update the Cow. This incurs a Vec allocation, but it is necessary
                // to satisfy the Cow<'_, [u8]> type for the next inspector in the chain.
                current_bytes = Cow::Owned(new_bytes.to_vec());
            }
            Decision::Reject(code) => return Ok(Err(code)),
        }
    }
    Ok(Ok(()))
}).await;

// ... Handle timeout/errors ...

// 3. Reconstruction: The "Hybrid" Zero-Copy Finish
let final_bytes = match modified_storage {
    Some(bytes) => bytes,               // Use the Stored Bytes directly (No final allocation)
    None => original_bytes.clone(),     // Zero-Copy (Arc Bump only)
};

// 4. Update Headers
if modified_storage.is_some() {
    parts.headers.insert("content-length", final_bytes.len().into());
}

let final_body = Body::from(final_bytes);

```

## 6. Definition of Done

* [ ] `BufferedForwarder` implemented with `Limited` wrapper and Global Semaphore.
* [ ] `Decision` enum and `Inspector` trait defined with `&[u8]` input.
* [ ] **Memory Profile Verified:**
* ≤ 1 allocation when all inspectors approve.
* Memory usage ≤ 2.0x payload size at peak.


* [ ] Panic handling verified (inspectors that panic do not crash the proxy).
* [ ] Empty body logic (`EC-006`) verified.
* [ ] Prometheus metrics (including per-inspector histograms) and OTel spans hooked up.
* [ ] All Edge Cases (`EC-001` to `EC-007`) covered by unit tests.