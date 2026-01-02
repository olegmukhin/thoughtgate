# ThoughtGate Architecture Specification

**Version:** 0.1.0
**Status:** Active
**Domain:** AI Traffic Governance (Sidecar)
**Traceability:** Mantra-Compatible

---

## 1. Executive Summary

ThoughtGate is a high-performance, memory-safe sidecar proxy designed to govern Agentic AI traffic. It enforces a **Hybrid Architecture**:
1.  **"Speedboat" Mode (Peeking):** Zero-copy streaming for high-volume, latency-sensitive LLM token generation.
2.  **"Governance" Mode (Termination):** Full message buffering for sensitive control-plane operations (MCP Tool Calls, HITL Tasks).

---

## 2. System Context (C4 Level 1)

```mermaid
graph LR
    subgraph Pod [Kubernetes Pod]
        Agent[Application Agent<br/>Python/Node/Go]
        Sidecar[ThoughtGate Sidecar<br/>Rust]
    end

    User[User / Client] -- "Ingress (HTTP)" --> Sidecar
    Sidecar -- "Localhost (UDS/TCP)" --> Agent

    Agent -- "Egress (HTTP)" --> Sidecar
    Sidecar -- "OpenAI / Anthropic" --> ExternalLLM[External LLM API]
    Sidecar -- "MCP Protocol" --> MCPServer[Upstream MCP Server]
    Sidecar -- "A2A Protocol" --> Agent2[Another agent]