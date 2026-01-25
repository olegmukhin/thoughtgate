---
sidebar_position: 1
---

# Quickstart

Get ThoughtGate running in under 5 minutes.

## Prerequisites

- Rust 1.75+ or Docker
- An MCP server to proxy

## Option 1: From Source

```bash
# Clone and build
git clone https://github.com/thoughtgate/thoughtgate
cd thoughtgate
cargo build --release

# Run
export THOUGHTGATE_UPSTREAM_URL=http://your-mcp-server:3000
./target/release/thoughtgate
```

## Option 2: Docker

```bash
docker run -d \
  -p 8080:8080 \
  -p 8081:8081 \
  -e THOUGHTGATE_UPSTREAM_URL=http://host.docker.internal:3000 \
  ghcr.io/thoughtgate/thoughtgate:latest
```

## Verify It Works

```bash
# Health check
curl http://localhost:8081/health

# Proxy a request
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'
```

## Add Slack Approvals

```bash
export THOUGHTGATE_SLACK_BOT_TOKEN=xoxb-your-token
export THOUGHTGATE_SLACK_CHANNEL="#approvals"
./target/release/thoughtgate
```

## Next Steps

- Follow the full [tutorial](/docs/tutorials/first-proxy)
- Learn to [write policies](/docs/how-to/write-policies)
- Read about [traffic tiers](/docs/explanation/traffic-tiers)
