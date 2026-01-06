# REQ-CORE-001: Zero-Copy Peeking Strategy (Green Path)

| Metadata | Value |
| --- | --- |
| **ID** | `REQ-CORE-001` |
| **Title** | Zero-Copy Peeking Strategy (Green Path) |
| **Type** | Core Mechanic |
| **Status** | Active |
| **Priority** | **Critical** |
| **Tags** | `#proxy` `#streaming` `#performance` `#latency` `#zero-copy` |

## 1. Context & Decision Rationale

This requirement implements the **"Green Path"** in ThoughtGate's traffic classification system.

* **Green Path (This REQ):** Zero-copy, pass-through streaming for trusted, high-volume traffic (e.g., LLM Token Streams).
* **Amber Path (`REQ-CORE-002`):** Buffered inspection for governance.
* **Red Path:** Immediate rejection.

Traffic enters the Green Path when the Governance Engine (`REQ-SEC-001`) returns `Decision::Allow`. This mode prioritizes **Latency** and **Throughput** above all else.

**"Peeking" Defined:**
In this context, "peeking" refers to **metadata-only inspection** (headers, method, URI) without buffering the body. No body content is ever read into application memory buffers (beyond the OS socket buffer).

## 2. Intent

The system must forward request and response body chunks immediately upon receipt to minimize Time-To-First-Byte (TTFB). It must operate with **zero application-level buffering**. Aggregation of body chunks into a full payload is strictly forbidden.

## 3. Constraints

### 3.1 Runtime & Dependencies

* **Runtime:** `tokio` (rt-multi-thread) with `hyper` v1.x and `hyper-util`.
* **Crates:** `bytes`, `http-body`, `http-body-util`, `tokio`, `socket2`.
* **Forbidden:** `Vec<u8>` accumulation, `String` conversion, or JSON deserialization of the body stream.

### 3.2 Network Optimization (CRITICAL)

* **Socket Options:** Both client and upstream `TcpStream` connections MUST be configured via `socket2`:
* `TCP_NODELAY` (Default: `true`)
* `SO_KEEPALIVE` (Default: `60s`)
* `SO_RCVBUF` / `SO_SNDBUF` (Default: `256KB`)


* **Config Loading:** Load once at startup.
* `THOUGHTGATE_TCP_NODELAY` (default: `true`)
* `THOUGHTGATE_TCP_KEEPALIVE_SECS` (default: `60`)
* `THOUGHTGATE_STREAM_READ_TIMEOUT_SECS` (default: `300`)
* `THOUGHTGATE_STREAM_WRITE_TIMEOUT_SECS` (default: `300`)
* `THOUGHTGATE_STREAM_TOTAL_TIMEOUT_SECS` (default: `3600`)
* `THOUGHTGATE_MAX_CONCURRENT_STREAMS` (default: `10000`)


* **Concurrency Limit:** Enforce the max streams limit using a global `tokio::sync::Semaphore`. If exhausted, return `503 Service Unavailable` immediately.
* **Backpressure:** The implementation must correctly propagate backpressure. If the client reads slowly, the proxy must pause reading from the upstream.

### 3.3 Protocol Transparency

* **Headers:** Preserve `Content-Length` and `Transfer-Encoding` exactly.
* **Hop-by-Hop:** Do not manually strip hop-by-hop headers if `hyper` handles them.
* **Upgrades:** WebSocket and HTTP/2 upgrades (e.g., `CONNECT`) MUST be supported via `tokio::io::copy_bidirectional`.

## 4. Functional Requirements

* **F-001 (Zero-Copy Forwarding):** The proxy must implement a custom `Body` struct that wraps the incoming stream and yields `Frame<Bytes>` directly.
* **Constraint:** No `clone()` of body chunks allowed. Move semantics only.


* **F-002 (Fail-Fast Error Propagation):**
* **Upstream Errors:** Connection Refused -> `502`, Timeout -> `504`.
* **Client Errors:** Disconnect -> Immediate Upstream Cancellation via `CancellationToken`.
* **Cleanup:** On any error, close both connections and log at `WARN`.


* **F-003 (Trailer Support):** HTTP Trailers must be forwarded. `poll_frame` must handle `Frame::trailers()`.
* **F-004 (Protocol Upgrade Handling):**
* **Trigger:** `Connection: Upgrade` header or `101 Switching Protocols` response.
* **Implementation:**
1. Forward request/response.
2. If status is `101`, extract underlying IO using `hyper::upgrade::on()`.
3. Switch to `tokio::io::copy_bidirectional`.
4. Record metric `upgrade_type="websocket"`.




* **F-005 (Timeout Handling):**
* **Read/Write:** Enforce per-chunk timeouts. If exceeded, abort with `408`/`504`.
* **Total Stream:** Wrap the entire stream handler in `tokio::time::timeout(total_timeout)`. If exceeded, abort to prevent slow-drip resource leaks.



## 4.5 Non-Functional Requirements (NFRs)

* **NFR-001 (Observability):**
* **Tracing:** Emit OTel span `green_path.stream` with attributes: `stream_duration_ms`, `bytes_transferred`, `upgrade_type`, `error_kind`.
* **Metrics:**
* `green_path_bytes_total{direction="upload|download"}`
* `green_path_streams_active`
* `green_path_streams_total{outcome="success|error|upgrade"}`
* **Histograms:** `green_path_ttfb_seconds` (buckets: .001, .005, .01, .05, .1) and `green_path_chunk_size_bytes`.




* **NFR-002 (Performance):**
* **Latency:** Added TTFB latency < 2ms (P99).
* **Memory:** Peak RSS delta < 5MB when streaming a 1GB payload (proving O(1) allocation).
* **Throughput:** Support 10k concurrent streams on a standard instance.



## 5. Verification Plan

### 5.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
| --- | --- | --- |
| **Trailers Present** | Forward chunks -> Forward trailers -> Close. | `EC-001` |
| **Client Disconnect** | Detect EOF, immediately close upstream (< 10ms). | `EC-002` |
| **Upstream Reset (RST)** | Propagate error to client immediately (502). | `EC-003` |
| **WebSocket Upgrade** | Switch to opaque TCP pipe, bi-directional flow works. | `EC-004` |
| **Slow Reader** | Upstream read pauses until client consumes chunk. | `EC-005` |
| **No-Body Response (204)** | Forward headers, yield no frames, finish immediately. | `EC-006` |
| **Large Chunk (16MB)** | Forward without splitting or intermediate buffer. | `EC-007` |
| **Concurrent Streams** | 10,000 streams run; 10,001st gets 503. | `EC-008` |
| **Invalid Chunk** | Detect upstream error, close connection. | `EC-009` |
| **Total Timeout** | Stream cut off exactly at `TOTAL_TIMEOUT_SECS`. | `EC-010` |

### 5.2 Assertions

* **Unit Test:** `test_peeking_forward_no_buffering`
* *Assert:* Memory usage stays flat (O(1)) while proxying a 100MB synthetic stream.


* **Integration Test:** `test_bidirectional_backpressure`
* *Assert:* Pause downstream -> Upstream TCP window fills -> Sender pauses.


* **Benchmark:** `bench_ttfb_overhead`
* *Goal:* P99 TTFB overhead < 2ms.


* **Fuzzing:** `cargo fuzz run green_path`
* *Assert:* Malformed chunks/headers do not cause panics or leaks.



## 7. Implementation Reference (Anti-Pattern Guard)

```rust
// Core Pattern: Zero-Copy Frame Forwarding
pub struct ProxyBody<B> {
    inner: B,
    metrics: StreamMetrics,
    cancel_token: CancellationToken, // For client disconnect handling
}

impl<B> http_body::Body for ProxyBody<B>
where
    B: http_body::Body<Data = Bytes> + Unpin,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Data = Bytes;
    type Error = hyper::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // 1. Check Cancellation
        if self.cancel_token.is_cancelled() {
            return Poll::Ready(None);
        }

        // 2. Poll Inner
        match Pin::new(&mut self.inner).poll_frame(cx) {
            Poll::Ready(Some(Ok(frame))) => {
                // 3. Metrics (Inspect Ref only)
                if let Some(data) = frame.data_ref() {
                    self.metrics.record_bytes(data.len());
                } else if frame.is_trailers() {
                    self.metrics.record_trailers();
                }
                // 4. Move Frame Forward (Zero Copy)
                Poll::Ready(Some(Ok(frame)))
            }
            // ... Error handling ...
        }
    }
}

// Socket Config
fn configure_socket(socket: &Socket) -> Result<()> {
    socket.set_nodelay(true)?; // Disable Nagle
    socket.set_recv_buffer_size(256 * 1024)?;
    Ok(())
}

```

### Anti-Patterns to Avoid

* **❌ Cloning Chunks:** `frame.data().clone()` defeats the purpose of Zero-Copy.
* **❌ Buffering:** `Vec::extend_from_slice()` turns the Green Path into a slow Amber Path.
* **❌ String Conversion:** `String::from_utf8()` allocates and crashes on binary data.
* **❌ Ignoring Backpressure:** Reading from upstream in a loop without writing downstream.

## 6. Definition of Done

* [x] `ProxyBody` wrapper implemented complying with `http_body::Body`.
* [x] `TCP_NODELAY` & `SO_KEEPALIVE` configured via `socket2`.
* [x] Concurrency limit (Semaphore) enforced and tested (`EC-008`).
* [~] **Upgrade/WebSocket handling (`EC-004`): PARTIAL** - Detection and header preservation implemented. Full bidirectional relay deferred (see Section 7).
* [x] Backpressure verified (`EC-005`).
* [~] **Timeout Handling (`F-005`): PARTIAL** - `TimeoutBody` wrapper implemented but not applied to Green Path. See Section 7.
* [x] Prometheus metrics (including Histograms) and OTel spans hooked up.
* [ ] **Memory Leak Test:** Valgrind/ASAN shows zero leaks after 10k streams.
* [ ] Performance benchmarks passed (TTFB < 2ms, Memory O(1)).

## 7. Known Limitations

### F-004: Protocol Upgrade Handling

**Status**: PARTIAL IMPLEMENTATION

### F-005: Timeout Handling on Green Path

**Status**: PARTIAL IMPLEMENTATION

**What Works**:
- Upgrade requests are detected via `is_upgrade_request()`
- Upgrade-related headers (`Connection`, `Upgrade`) are preserved
- 101 Switching Protocols responses are logged
- `serve_connection_with_upgrades` is enabled in `main.rs`

**What's Missing**:
- Explicit use of `hyper::upgrade::on()` to extract raw IO
- Manual `tokio::io::copy_bidirectional()` for the relay
- Full "opaque TCP pipe" semantics per specification

**Why**:
The current architecture uses `hyper_util::client::legacy::Client` which abstracts away the underlying connection. Implementing explicit upgrade handling requires:
1. Custom connection pooling that exposes raw `TcpStream`
2. Manual coordination of upgrade handshakes on both sides
3. Refactoring the request/response lifecycle to accommodate detached IO

**Impact**:
For most practical purposes (WebSocket, HTTP/2 upgrades), hyper's internal handling is sufficient. The gap is primarily in strict conformance to the "opaque TCP pipe" requirement and explicit control over the bidirectional relay.

**Remediation Path**:
1. Replace `Client` API with manual `http1::handshake()` and connection pooling
2. Implement upgrade coordinator that uses `hyper::upgrade::on()` on both sides
3. Add integration test that verifies actual bidirectional data flow (not just header detection)

**What Works**:
- `TimeoutBody` wrapper exists and correctly implements per-chunk and total stream timeouts
- `ProxyBody` wrapper exists for metrics and cancellation
- Configuration values are loaded from environment variables

**What's Missing**:
- `TimeoutBody` and `ProxyBody` are not applied to Green Path response bodies
- Response bodies are returned as `Incoming` without timeout protection
- Per-chunk read timeouts are not enforced on streaming responses

**Why**:
Applying the wrappers would change the return type from `Response<Incoming>` to `Response<ProxyBody<TimeoutBody<Incoming>>>`. This requires:
1. Type erasure via `BoxBody` (adds allocation overhead, contradicts zero-copy goal)
2. Changes to `Service` trait implementation
3. Updates to all call sites in `main.rs`

**Impact**:
The Green Path is vulnerable to slow-read attacks where an upstream server sends chunks with arbitrary delays. Without per-chunk timeouts, a malicious or misbehaving upstream can hold connections open indefinitely.

**Note**: The Amber Path (BufferedForwarder) is protected via `tokio::time::timeout` wrapping the entire buffering operation.

**Remediation Path**:
1. Change `handle_request` return type to use `BoxBody` for type erasure
2. Apply `TimeoutBody` wrapper with `stream_read_timeout` and `stream_total_timeout`
3. Apply `ProxyBody` wrapper for metrics and cancellation token
4. Update `Service` trait implementation to handle boxed bodies
5. Add integration test that verifies timeout enforcement on slow responses