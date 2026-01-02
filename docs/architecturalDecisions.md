# Architectural Decision Records (ADRs)

**Project:** ThoughtGate  
**Domain:** AI Traffic Governance Sidecar Proxy  
**Status:** Active

---

## ADR-001: Hybrid Proxy Architecture (Peeking + Termination)

**Date:** 2026-01-02  
**Status:** Accepted  
**Context:** REQ-CORE-001, REQ-CORE-002

### Decision
ThoughtGate implements a **hybrid proxy architecture** with two operational modes:

1. **Peeking Mode (Default)**: Zero-copy streaming for high-volume LLM token generation
2. **Termination Mode (Protocol-Triggered)**: Full message buffering for governance-critical operations

### Rationale
- **Performance**: LLM streaming responses (SSE, chunked encoding) require sub-10ms TTFB overhead
- **Governance**: MCP tool calls and HITL tasks require full message inspection for policy enforcement
- **Trade-off**: Accept mode-switching complexity to avoid forcing all traffic through a buffering bottleneck

### Consequences
- **Positive**:
  - Minimizes latency for 95% of traffic (LLM responses)
  - Enables deep inspection only when needed
  - Supports both forward and reverse proxy modes
- **Negative**:
  - Requires protocol detection logic (HTTP peeking)
  - Mode-switching adds implementation complexity
  - Must maintain two separate code paths

### Implementation
- `src/main.rs`: TCP peeking to detect protocol triggers
- `src/proxy_service.rs`: Zero-copy streaming via `hyper_util::BodyStream`
- Future: Protocol parsers for MCP/A2A detection

---

## ADR-002: Hyper v1.0 + Tower Middleware Stack

**Date:** 2026-01-02  
**Status:** Accepted  
**Context:** Blessed Stack (Section V)

### Decision
Use **Hyper v1.0** as the HTTP foundation with **Tower** for middleware composition.

### Rationale
- **Hyper v1.0**: 
  - Provides HTTP/1.1 and HTTP/2 support with zero-copy primitives
  - `hyper_util::server::conn::auto` handles protocol negotiation
  - Production-proven in Linkerd2-proxy
- **Tower**:
  - Composable middleware layers (Logging, future: RateLimit, Policy, Auth)
  - Service trait enables testability and extensibility
  - Industry standard (used by Tonic, Linkerd, etc.)

### Consequences
- **Positive**:
  - Zero-copy streaming via `bytes::Bytes`
  - Extensible middleware without core refactoring
  - Strong ecosystem support
- **Negative**:
  - Tower's learning curve (Service trait, poll_ready)
  - Hyper v1.0 still evolving (some utils in `hyper-util`)

### Implementation
- `Cargo.toml`: hyper = "1", hyper-util = "0.1", tower = "0.5"
- `src/logging_layer.rs`: First middleware layer (LoggingLayer)
- `src/main.rs`: ServiceBuilder composes layers

---

## ADR-003: Panic Safety - No `.unwrap()` or `.expect()` in Runtime Logic

**Date:** 2026-01-02  
**Status:** Accepted  
**Context:** Section I.4 (Safety First)

### Decision
**Strictly prohibit** `.unwrap()` and `.expect()` in runtime code paths. All errors must be:
1. Propagated via `?` operator, OR
2. Handled explicitly with recovery logic

### Rationale
- **Reliability**: A single panic can crash the entire sidecar, taking down the application pod
- **Observability**: Panics are not structured and bypass tracing
- **Kubernetes**: Crash loops trigger exponential backoff, degrading availability

### Consequences
- **Positive**:
  - Forces explicit error handling
  - Improves debuggability (errors are logged, not panicked)
  - Aligns with Rust best practices
- **Negative**:
  - Slightly more verbose code
  - Requires refactoring constructors to return `Result<T, E>`

### Implementation
- **Violations Identified**:
  - `src/proxy_service.rs:50`: `.expect("Failed to load native TLS roots")`
  - `src/proxy_service.rs:96`: `.unwrap()` on `headers_mut()`
- **Fix**: Refactor `ProxyService::new_with_upstream()` to return `Result<Self, ProxyError>`

---

## ADR-004: Mantra for Requirements Traceability

**Date:** 2026-01-02  
**Status:** Accepted  
**Context:** Section I.2 (Traceability is Mandatory)

### Decision
Use **Mantra** (https://github.com/moosichu/mantra) to enforce bidirectional traceability between:
- **Specs**: `specs/*.md` files containing `REQ-XXX` identifiers
- **Code**: Rustdoc comments with `/// Implements: REQ-XXX` tags

### Rationale
- **Auditability**: Can prove which requirements are implemented
- **Coverage**: Can identify orphaned code (no requirement) or missing implementations
- **CI Integration**: `mantra check` can block merges for missing traceability

### Consequences
- **Positive**:
  - Automated verification of spec-to-code linkage
  - Forces disciplined requirements management
  - Provides coverage reports
- **Negative**:
  - Adds overhead to development (must tag all public APIs)
  - Mantra is relatively new (less mature than rustdoc)

### Implementation
- `mantra.toml`: Configure patterns for Rust (`/// - Implements: (REQ-[0-9]+)`)
- `specs/`: Markdown files with `REQ-XXX` identifiers
- **Rustdoc**: Add traceability sections to all public structs/functions

---

## ADR-005: thiserror for Library, anyhow for Binaries

**Date:** 2026-01-02  
**Status:** Accepted  
**Context:** Section V (Blessed Stack - Error)

### Decision
- **Library code** (`src/*.rs`): Use `thiserror` for structured error types
- **Binary code** (`main.rs`, tests): Use `anyhow` for ergonomic error propagation

### Rationale
- **thiserror**: Generates `Display` and `Error` impls, preserves type information
- **anyhow**: Provides context chaining (`context()`), good for application-level errors
- **Separation**: Libraries should define concrete error types; binaries can be more flexible

### Consequences
- **Positive**:
  - Library errors are typed and matchable
  - Binary errors have rich context for debugging
  - Aligns with Rust ecosystem best practices
- **Negative**:
  - Two error crates to maintain
  - Context switching between error styles

### Implementation
- `src/error.rs`: `ProxyError` enum with `#[derive(Error)]`
- `src/main.rs`: Uses `anyhow` in `main()` return type (already implemented)
- `tests/*`: Use `anyhow::Result` for test helpers

---

## ADR-006: Structured JSON Logging with Sensitive Header Redaction

**Date:** 2026-01-02  
**Status:** Accepted  
**Context:** Section IV (Observability & Security)

### Decision
- **Format**: JSON-structured logs via `tracing-subscriber`
- **Redaction**: Automatically strip sensitive headers (`Authorization`, `Cookie`, `x-api-key`)
- **Cardinality**: Max 5 span attributes to prevent metric explosion

### Rationale
- **JSON**: Machine-parseable for log aggregation (Splunk, Elasticsearch)
- **Security**: Prevents credential leakage in logs (compliance requirement)
- **Cardinality**: High-cardinality fields (e.g., UUIDs) cause storage/query issues

### Consequences
- **Positive**:
  - Logs are structured and queryable
  - Prevents accidental credential exposure
  - Reduces storage costs
- **Negative**:
  - Less human-readable than plain text
  - Redaction logic must be maintained

### Implementation
- `src/logging_layer.rs`: `SENSITIVE_HEADERS` constant + `sanitize_headers()`
- `src/main.rs`: `tracing_subscriber::fmt().json()`

---

## ADR-007: TCP_NODELAY for Low-Latency Streaming

**Date:** 2026-01-02  
**Status:** Accepted  
**Context:** REQ-CORE-001 F-001 (Latency)

### Decision
Enforce `TCP_NODELAY` on all connections (client → proxy, proxy → upstream).

### Rationale
- **Problem**: Nagle's algorithm buffers small packets (up to 200ms delay)
- **LLM Streaming**: SSE/chunked tokens arrive in ~50-byte chunks
- **User Experience**: 200ms buffering is perceptible and degrades UX

### Consequences
- **Positive**:
  - Minimizes TTFB for token streaming
  - Improves perceived responsiveness
- **Negative**:
  - Slightly increases network overhead (more packets)
  - Not configurable (always-on)

### Implementation
- `src/main.rs:148`: `stream.set_nodelay(true)?`
- Future: Set on upstream connections as well

---

## Decision Log

| ADR | Title | Date | Status |
|-----|-------|------|--------|
| ADR-001 | Hybrid Proxy Architecture | 2026-01-02 | Accepted |
| ADR-002 | Hyper v1.0 + Tower | 2026-01-02 | Accepted |
| ADR-003 | Panic Safety | 2026-01-02 | Accepted |
| ADR-004 | Mantra Traceability | 2026-01-02 | Accepted |
| ADR-005 | thiserror + anyhow | 2026-01-02 | Accepted |
| ADR-006 | Structured JSON Logging | 2026-01-02 | Accepted |
| ADR-007 | TCP_NODELAY | 2026-01-02 | Accepted |

---

## Future Decisions (Pending)

- **ADR-008**: Protocol detection strategy (HTTP peeking vs. TLS ALPN)
- **ADR-009**: Policy engine selection (Cedar vs. OPA vs. custom)
- **ADR-010**: IPC framing protocol (NDJSON vs. length-prefixed)
- **ADR-011**: Metrics backend (Prometheus vs. OpenTelemetry)
- **ADR-012**: xDS integration for dynamic configuration

