---
sidebar_position: 1
---

# Your First ThoughtGate Proxy

In this tutorial, you'll set up ThoughtGate to proxy requests between an AI agent and an MCP server, with a simple policy that requires approval for destructive operations.

## What You'll Learn

- How to run ThoughtGate as a proxy
- How to write a basic Cedar policy
- How to configure Slack approvals
- How to test the approval workflow

## Prerequisites

- Rust 1.75+ installed
- An MCP server running (or use the mock server)
- A Slack workspace with bot token (for approvals)

## Step 1: Build ThoughtGate

Clone the repository and build the release binary:

```bash
git clone https://github.com/thoughtgate/thoughtgate
cd thoughtgate
cargo build --release
```

The binary will be at `target/release/thoughtgate`.

## Step 2: Start a Mock MCP Server

For testing, ThoughtGate includes a mock MCP server:

```bash
cargo build --release --features mock
./target/release/mock_llm --port 3000
```

This server responds to MCP tool calls with mock data.

## Step 3: Write a Cedar Policy

Create a file `policy.cedar` with a simple policy:

```cedar
// Allow all tools/list requests
permit(
    principal,
    action == Action::"tools/list",
    resource
);

// Allow tools/call but require approval for destructive operations
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name in ["delete_user", "drop_table", "send_email"]
} advice {
    "require_approval": true
};

// Allow all other tools/call requests
permit(
    principal,
    action == Action::"tools/call",
    resource
);
```

## Step 4: Configure Slack Integration

1. Create a Slack app at [api.slack.com/apps](https://api.slack.com/apps)
2. Add Bot Token Scopes: `chat:write`, `reactions:read`, `channels:history`
3. Install to your workspace
4. Copy the Bot OAuth Token
5. Create a channel for approvals (e.g., `#thoughtgate-approvals`)
6. Invite the bot: `/invite @YourBotName`

## Step 5: Start ThoughtGate

Set environment variables and start the proxy:

```bash
export THOUGHTGATE_UPSTREAM_URL=http://localhost:3000
export THOUGHTGATE_OUTBOUND_PORT=8080
export THOUGHTGATE_ADMIN_PORT=8081
export THOUGHTGATE_CEDAR_POLICY_PATH=./policy.cedar
export THOUGHTGATE_SLACK_BOT_TOKEN=xoxb-your-token
export THOUGHTGATE_SLACK_CHANNEL="#thoughtgate-approvals"

./target/release/thoughtgate
```

You should see:

```
ThoughtGate listening on 0.0.0.0:8080
Admin server on 0.0.0.0:8081
Upstream: http://localhost:3000
```

## Step 6: Test the Proxy

Send a tools/list request (should pass through):

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'
```

Send a safe tools/call request (should pass through):

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {"name": "get_weather", "arguments": {"city": "London"}},
    "id": 2
  }'
```

## Step 7: Trigger an Approval

Send a destructive tools/call request:

```bash
curl -X POST http://localhost:8080 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {"name": "delete_user", "arguments": {"user_id": "12345"}},
    "id": 3
  }'
```

This request will block while waiting for approval.

Check your Slack channel — you should see a message like:

```
🔔 Approval Required

Tool: delete_user
Arguments: {"user_id": "12345"}

React 👍 to approve or 👎 to reject
```

React with 👍 to approve the request. The curl command will complete with the response.

## Step 8: Verify Health

Check the health endpoints:

```bash
curl http://localhost:8081/health
curl http://localhost:8081/ready
```

## What's Next?

- Learn how to [write more complex policies](/docs/how-to/write-policies)
- Understand [traffic tiers](/docs/explanation/traffic-tiers) in depth
- [Deploy to Kubernetes](/docs/how-to/deploy-kubernetes) for production
