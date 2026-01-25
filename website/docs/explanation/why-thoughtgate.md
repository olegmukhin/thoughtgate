---
sidebar_position: 1
---

# Why ThoughtGate

AI agents are increasingly capable of taking real-world actions. They can delete files, send emails, make purchases, modify databases, and interact with external APIs. This power creates a fundamental challenge: **how do you give agents autonomy while maintaining human oversight?**

## The Problem

Consider an AI agent tasked with managing customer support:

1. Agent receives a request: "Cancel user #12345's subscription"
2. Agent calls the `cancel_subscription` tool
3. Subscription is immediately cancelled
4. **No human ever reviewed this action**

What if the request was:
- From a malicious prompt injection?
- A misunderstanding of ambiguous instructions?
- An edge case the agent wasn't trained for?

Without oversight, mistakes are irreversible and potentially costly.

## Existing Approaches

### Option 1: No Tool Access

Remove dangerous capabilities entirely.

**Problem:** The agent becomes useless for meaningful work.

### Option 2: Trust the Agent

Let the agent decide what's safe.

**Problem:** Agents make mistakes. Prompt injections happen. Edge cases exist.

### Option 3: Approve Everything

Require human approval for every action.

**Problem:** Humans become bottlenecks. Agent autonomy is eliminated.

## The ThoughtGate Approach

ThoughtGate provides a **policy-based middle ground**:

```
┌─────────────────────────────────────────────────────────────────┐
│                         THOUGHTGATE                              │
│                                                                 │
│   Request ──▶ Policy Evaluation ──▶ Tier Classification        │
│                                                                 │
│   Green (safe)      → Forward immediately                       │
│   Amber (inspect)   → Check content, then forward               │
│   Red (sensitive)   → Require human approval or deny            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Key Principles

1. **Declarative Policies**: Define rules once, apply consistently
2. **Tiered Trust**: Not all actions are equal
3. **Human-in-the-Loop**: Humans approve sensitive operations
4. **Minimal Overhead**: Green path adds < 2ms latency

## When to Use ThoughtGate

ThoughtGate is valuable when:

- AI agents can take **consequential actions** (delete, modify, send)
- You need **audit trails** of agent decisions
- You want **human oversight** without blocking every request
- You need to **enforce policies** across multiple agents

## When Not to Use ThoughtGate

ThoughtGate may be overkill when:

- Agents only have read-only access
- All actions are easily reversible
- You're in a pure development/testing environment

## Design Philosophy

### 1. Fail-Safe Defaults

If ThoughtGate can't evaluate a request (policy error, connection issue), it denies the request rather than allowing potentially dangerous actions through.

### 2. Separation of Concerns

- **Policies** define what requires approval
- **ThoughtGate** enforces policies
- **Slack** provides the approval interface
- **Humans** make approval decisions

### 3. Low Operational Overhead

ThoughtGate is designed as a lightweight sidecar:
- < 15 MB binary
- < 20 MB memory footprint
- < 2 ms latency overhead (green path)
- Hot-reloadable policies

## Next Steps

- Understand the [Architecture](/docs/explanation/architecture)
- Learn about [Traffic Tiers](/docs/explanation/traffic-tiers)
- See the [Security Model](/docs/explanation/security-model)
