---
sidebar_position: 2
---

# Architecture

ThoughtGate is a transparent proxy that sits between AI agents and MCP servers.

## High-Level Overview

```
┌─────────────┐     ┌─────────────────────────────────────┐     ┌─────────────┐
│  AI Agent   │────▶│           ThoughtGate               │────▶│  MCP Server │
│  (Claude,   │◀────│                                     │◀────│  (Tools)    │
│   GPT, etc) │     │  Outbound: 8080    Admin: 8081      │     │             │
└─────────────┘     └─────────────────────────────────────┘     └─────────────┘
```

## Components

### 1. Transport Layer

Handles JSON-RPC 2.0 message parsing and routing:

- Parses incoming MCP requests
- Preserves request ID types (string, number, null)
- Routes responses back to clients
- Manages connection pooling to upstream

### 2. Policy Engine

Evaluates requests against Cedar policies:

- Loads policies from file
- Watches for policy changes (hot reload)
- Returns tier classification (Green, Amber, Red)
- Extracts advice metadata for approval routing

### 3. Governance Layer

Manages approval workflows:

- Creates approval tasks for Red-tier requests
- Posts messages to Slack
- Polls for reactions (👍/👎)
- Handles timeouts and cancellation

### 4. Admin Server

Provides operational endpoints:

- `/health` — Liveness probe
- `/ready` — Readiness probe
- `/metrics` — Prometheus metrics

## Request Flow

```
                    ┌──────────────────────────────────────────┐
                    │               THOUGHTGATE                 │
                    │                                          │
  Request ─────────▶│  1. Parse JSON-RPC                       │
                    │  2. Evaluate Cedar Policy                │
                    │  3. Classify Tier                        │
                    │                                          │
                    │     ┌─────────────────────────────────┐  │
                    │     │         TIER ROUTER             │  │
                    │     │                                 │  │
                    │     │  Green ──▶ Forward to upstream  │──┼──▶ Upstream
                    │     │  Amber ──▶ Inspect, then fwd    │  │
                    │     │  Red ────▶ Approval workflow    │  │
                    │     │         ──▶ or Deny             │  │
                    │     └─────────────────────────────────┘  │
                    │                                          │
  Response ◀────────│  4. Return upstream response or error   │
                    │                                          │
                    └──────────────────────────────────────────┘
```

## Port Model

ThoughtGate uses a two-port model:

| Port | Name | Purpose |
|------|------|---------|
| 8080 | Outbound | Proxy traffic (agent → upstream) |
| 8081 | Admin | Health checks, metrics |

This separation ensures health checks don't interfere with proxy traffic and allows different security policies per port.

## State Management

### v0.1: In-Memory

All state is held in memory:
- Pending approval tasks
- Connection pools
- Policy cache

**Implication:** Pending approvals are lost on restart.

### Future: Persistent State

Planned for v0.2:
- Redis-backed task storage
- Approval state survives restarts
- Multi-instance coordination

## Deployment Model

ThoughtGate is designed as a **sidecar**:

```yaml
spec:
  containers:
    - name: agent          # Your AI agent
    - name: thoughtgate    # Sidecar proxy
```

Benefits:
- No network hop (localhost)
- Per-agent isolation
- Independent scaling
- Simple security model

## Performance Characteristics

| Path | Latency Overhead | Description |
|------|------------------|-------------|
| Green | < 2 ms | Policy eval + forward |
| Amber | < 5 ms | Policy eval + inspect + forward |
| Red (deny) | < 1 ms | Policy eval + error |
| Red (approve) | Seconds to minutes | Waiting for human |

## Failure Modes

### Upstream Unavailable

- Request fails with `-32000 UpstreamConnectionFailed`
- Readiness probe fails
- Agent receives clear error

### Policy Error

- Request denied (fail-safe)
- Logged as error
- Operator alerted via metrics

### Slack Unavailable

- Approval requests fail
- Falls back to timeout behavior
- Logged as error

## Next Steps

- Understand [Traffic Tiers](/docs/explanation/traffic-tiers) in depth
- Learn about the [Security Model](/docs/explanation/security-model)
