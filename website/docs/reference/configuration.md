---
sidebar_position: 1
---

# Configuration Reference

ThoughtGate is configured via environment variables.

## Core Settings

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `THOUGHTGATE_UPSTREAM_URL` | Yes | — | Upstream MCP server URL |
| `THOUGHTGATE_OUTBOUND_PORT` | No | `8080` | Port for proxy traffic |
| `THOUGHTGATE_ADMIN_PORT` | No | `8081` | Port for health/metrics endpoints |
| `THOUGHTGATE_REQUEST_TIMEOUT_SECS` | No | `30` | Upstream request timeout |

## Policy Settings

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `THOUGHTGATE_CEDAR_POLICY_PATH` | No | (embedded) | Path to Cedar policy file |
| `THOUGHTGATE_POLICY_RELOAD_SECS` | No | `10` | Policy file watch interval |

## Slack Integration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `THOUGHTGATE_SLACK_BOT_TOKEN` | For approvals | — | Slack Bot OAuth token (`xoxb-...`) |
| `THOUGHTGATE_SLACK_CHANNEL` | For approvals | — | Channel for approval messages (e.g., `#approvals`) |
| `THOUGHTGATE_APPROVAL_TIMEOUT_SECS` | No | `300` | Max time to wait for approval (5 min) |
| `THOUGHTGATE_APPROVAL_POLL_INTERVAL_SECS` | No | `5` | Slack polling interval |

## Observability

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `THOUGHTGATE_LOG_LEVEL` | No | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `THOUGHTGATE_LOG_FORMAT` | No | `text` | Log format: `text` or `json` |
| `THOUGHTGATE_METRICS_ENABLED` | No | `true` | Enable Prometheus metrics |

## Example Configuration

### Minimal

```bash
export THOUGHTGATE_UPSTREAM_URL=http://mcp-server:3000
./thoughtgate
```

### With Slack Approvals

```bash
export THOUGHTGATE_UPSTREAM_URL=http://mcp-server:3000
export THOUGHTGATE_CEDAR_POLICY_PATH=/etc/thoughtgate/policy.cedar
export THOUGHTGATE_SLACK_BOT_TOKEN=xoxb-your-token
export THOUGHTGATE_SLACK_CHANNEL="#approvals"
export THOUGHTGATE_APPROVAL_TIMEOUT_SECS=600
./thoughtgate
```

### Docker

```bash
docker run -d \
  -p 8080:8080 \
  -p 8081:8081 \
  -e THOUGHTGATE_UPSTREAM_URL=http://host.docker.internal:3000 \
  -e THOUGHTGATE_SLACK_BOT_TOKEN=xoxb-your-token \
  -e THOUGHTGATE_SLACK_CHANNEL="#approvals" \
  -v /path/to/policy.cedar:/etc/thoughtgate/policy.cedar \
  -e THOUGHTGATE_CEDAR_POLICY_PATH=/etc/thoughtgate/policy.cedar \
  ghcr.io/thoughtgate/thoughtgate:latest
```

## Admin Endpoints

Available on `THOUGHTGATE_ADMIN_PORT`:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Liveness check (returns 200 if running) |
| `/ready` | GET | Readiness check (returns 200 if ready for traffic) |
| `/metrics` | GET | Prometheus metrics |

## Prometheus Metrics

```prometheus
# Request counts by tier
thoughtgate_requests_total{tier="green|amber|red"}

# Request latency histogram
thoughtgate_request_duration_seconds{tier="..."}

# Approval outcomes
thoughtgate_approvals_total{result="approved|rejected|timeout"}

# Upstream request results
thoughtgate_upstream_requests_total{status="success|error|timeout"}

# Policy evaluation latency
thoughtgate_policy_eval_duration_seconds
```
