---
sidebar_position: 3
---

# Traffic Tiers

ThoughtGate classifies all traffic into three tiers based on policy evaluation. Each tier has different handling characteristics.

## Overview

| Tier | Trust Level | Behavior | Latency |
|------|-------------|----------|---------|
| **Green** | High | Forward immediately | < 2 ms |
| **Amber** | Medium | Inspect, then forward | < 5 ms |
| **Red** | Low | Deny or require approval | Variable |

## Green Tier

**Purpose:** Fast path for trusted, low-risk operations.

### Characteristics

- Minimal processing
- No content inspection
- Direct forwarding to upstream
- Responses passed through unchanged

### Typical Use Cases

- `tools/list` — Listing available tools
- Read-only queries
- Idempotent operations
- Internal/trusted tool calls

### Policy Example

```cedar
permit(
    principal,
    action == Action::"tools/list",
    resource
);

permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name in ["get_balance", "list_items", "search"]
};
```

## Amber Tier

**Purpose:** Balanced path for operations that need inspection but not approval.

### Characteristics

- Request/response buffering
- Content inspection (PII detection, schema validation)
- Transformation possible
- Slightly higher latency

### Typical Use Cases

- Responses containing user data
- Operations with compliance requirements
- Logging/audit requirements
- Data transformation needs

### Policy Example

```cedar
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name == "get_user_profile"
} advice {
    "inspect": true,
    "redact_pii": true
};
```

:::note

Amber tier inspection is planned for v0.2. In v0.1, Amber tier requests are treated as Green tier.

:::

## Red Tier

**Purpose:** High-scrutiny path for sensitive operations.

### Characteristics

Red tier has two sub-paths:

#### Deny Path

- Immediate rejection
- No upstream communication
- Clear error response

#### Approval Path

- Request blocks until human approval
- Message posted to Slack
- Human reacts with 👍 (approve) or 👎 (reject)
- On approval: forwards to upstream
- On rejection: returns error
- On timeout: returns error

### Typical Use Cases

- Destructive operations (delete, drop, remove)
- Financial transactions
- PII modifications
- Privilege escalation
- Administrative actions

### Policy Examples

**Deny outright:**

```cedar
forbid(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name.startsWith("admin_")
};
```

**Require approval:**

```cedar
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name in ["delete_user", "transfer_funds"]
} advice {
    "require_approval": true
};
```

## Tier Selection Logic

Policy evaluation determines the tier:

```
┌─────────────────────────────────────────────────────────┐
│                  POLICY EVALUATION                       │
│                                                         │
│  1. Evaluate all policies in order                      │
│  2. First matching forbid → RED (deny)                  │
│  3. First matching permit with require_approval         │
│     → RED (approval)                                    │
│  4. First matching permit with inspect                  │
│     → AMBER                                             │
│  5. First matching permit (no advice)                   │
│     → GREEN                                             │
│  6. No match → RED (deny) [fail-safe]                  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Choosing the Right Tier

### Use Green When

- Operation is read-only
- Failure is easily recoverable
- No sensitive data involved
- High-frequency operation

### Use Amber When

- Need to inspect content
- Compliance/audit requirements
- Data transformation needed
- Moderate sensitivity

### Use Red (Deny) When

- Operation should never be allowed
- Clear policy violation
- Known attack pattern

### Use Red (Approval) When

- Operation is legitimate but sensitive
- Human judgment required
- Irreversible consequences
- High-value transactions

## Performance Implications

| Tier | p50 Latency | p99 Latency | Notes |
|------|-------------|-------------|-------|
| Green | 1-2 ms | 5 ms | Network bound |
| Amber | 3-5 ms | 15 ms | Inspection bound |
| Red (deny) | < 1 ms | 2 ms | No upstream call |
| Red (approve) | 10s-5m | Timeout | Human bound |

## Monitoring Tiers

Prometheus metrics by tier:

```prometheus
thoughtgate_requests_total{tier="green"}
thoughtgate_requests_total{tier="amber"}
thoughtgate_requests_total{tier="red"}

thoughtgate_request_duration_seconds{tier="green"}
thoughtgate_request_duration_seconds{tier="amber"}
thoughtgate_request_duration_seconds{tier="red"}
```

Healthy distribution varies by use case, but typical patterns:

- **High automation:** 90% green, 8% amber, 2% red
- **High oversight:** 60% green, 20% amber, 20% red
- **Strict governance:** 40% green, 30% amber, 30% red
