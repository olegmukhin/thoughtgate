# REQ-CORE-001: Zero-Copy Peeking Strategy

| Metadata | Value |
| --- | --- |
| **ID** | `REQ-CORE-001` |
| **Title** | Zero-Copy Peeking Strategy |
| **Type** | Core Mechanic |
| **Status** | Active |
| **Priority** | **Critical** |
| **Tags** | `#proxy` `#streaming` `#performance` |

## 1. Context

ThoughtGate operates primarily in "Peeking Mode" as a transparent bidirectional streaming proxy between a downstream client and upstream server. This usually involves HTTP/1.1 or HTTP/2 traffic with chunked `Transfer-Encoding` for LLM token streams.

## 2. Intent

The system must forward request and response body chunks immediately upon receipt to minimize Time-To-First-Byte (TTFB). It must operate with **zero additional application-level buffering**. Aggregation of body chunks into a full payload is strictly forbidden unless an explicit mode switch triggers `REQ-CORE-002` (Termination).

## 3. Constraints

* **Runtime:** Must use `tokio` (rt-multi-thread) with `hyper` v1.x.
* **Crates:** `bytes`, `http-body`, `http-body-util`, `tokio`.
* **Zero-Copy:** All data movement MUST use `bytes::Bytes` or `http-body::Frame`.
* **Forbidden:** `Vec<u8>` accumulation, `String` conversion, or JSON deserialization of the body stream.

* **Socket Options:** Both client and upstream `TcpStream` connections MUST have `TCP_NODELAY` enabled (to disable Nagle's algorithm).
* **Headers:**
* Preserve `Content-Length` and `Transfer-Encoding` exactly.
* Do not modify or strip hop-by-hop headers manually if `hyper` handles them, but ensure transparency.

* **Edge Cases:**
* **Trailers:** Handle chunked encoding trailers by forwarding them without buffering the stream.
* **Upgrades:** Support WebSocket/Upgrade requests by forwarding the opaque byte stream.
* **EOF:** Forward Early Connection Closure (EOF) immediately.

## 4. Functional Requirements

* **F-001 (Latency):** The implementation must strictly enforce `TCP_NODELAY` on both inbound and outbound TCP sockets.
* **F-002 (Zero-Copy):** The proxy must implement a `BodyStream` wrapper that yields `Frame<Bytes>` directly from the underlying transport.
* **F-003 (Transparency):** The proxy must act as a transparent pipe for headers and body chunks, modifying only connection-specific metadata required by the HTTP spec.

## 5. Verification Plan

* **Unit Test:** `test_peeking_forward_no_buffering`
* *Goal:* Assert no peak memory growth beyond baseline when proxying a 10MB stream.

* **Integration Test:** `test_bidirectional_stream`
* *Goal:* Verify chunk-by-chunk forwarding using a synthetic slow-sender client and server.

* **Benchmark:** `criterion` / `k6`
* *Goal:* P95 TTFB delta **< 10ms** vs direct connection (Baseline).

* **Memory Profile:**
* *Goal:* Peak RSS delta **< 5MB** when streaming a 100MB payload.

* **Fuzzing:** `cargo fuzz run peeking_fuzz`
* *Goal:* Malformed chunks or interrupted streams must not cause panics or unbounded buffering.

* **Traceability:**
* *Goal:* `mantra check` must pass. All relevant code must be annotated with `/// Implements: REQ-CORE-001`.

## 6. Definition of Done

* [ ] All verification items pass in CI.
* [ ] Code review confirms zero-copy `Bytes` usage (no `.to_vec()` or `.clone()` on body chunks).
* [ ] `mantra` traceability check passes.