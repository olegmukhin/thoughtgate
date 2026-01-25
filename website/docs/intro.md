---
sidebar_position: 1
slug: /
---

# Introduction

ThoughtGate is a sidecar proxy that intercepts MCP (Model Context Protocol) tool calls and routes them through policy-based approval workflows before execution. It ensures AI agents can't perform sensitive operations without human oversight.

## What ThoughtGate Does

```
┌─────────────┐     ┌─────────────────────────────────────┐     ┌─────────────┐
│  AI Agent   │────▶│           ThoughtGate               │────▶│  MCP Server │
│  (Claude,   │◀────│  • Policy evaluation (Cedar)        │◀────│  (Tools)    │
│   GPT, etc) │     │  • Approval workflows (Slack)       │     │             │
└─────────────┘     │  • Traffic classification           │     └─────────────┘
                    └─────────────────────────────────────┘
```

When an AI agent attempts to call a tool, ThoughtGate:

1. **Evaluates the request** against Cedar policies
2. **Classifies the traffic** into Green, Amber, or Red tiers
3. **Routes appropriately** — forward immediately, inspect first, require approval, or deny
4. **Maintains audit trails** of all decisions and outcomes

## Key Features

| Feature | Description |
|---------|-------------|
| **Cedar Policies** | Flexible policy language for defining approval rules |
| **Traffic Tiers** | Green (pass-through), Amber (inspect), Red (deny/approve) |
| **Slack Approvals** | Human-in-the-loop approval via Slack reactions |
| **Low Overhead** | < 2ms p50 latency, < 20MB memory footprint |

## Quick Links

- **New to ThoughtGate?** Start with the [Quickstart](/docs/how-to/quickstart)
- **Want to understand the concepts?** Read [Why ThoughtGate](/docs/explanation/why-thoughtgate)
- **Ready to deploy?** See [Deploy to Kubernetes](/docs/how-to/deploy-kubernetes)
- **Looking up configuration?** Check the [Configuration Reference](/docs/reference/configuration)
