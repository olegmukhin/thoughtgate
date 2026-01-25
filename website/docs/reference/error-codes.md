---
sidebar_position: 3
---

# Error Codes Reference

ThoughtGate returns JSON-RPC 2.0 compliant errors.

## Standard JSON-RPC Errors

| Code | Name | Description |
|------|------|-------------|
| `-32700` | Parse Error | Invalid JSON |
| `-32600` | Invalid Request | Not a valid JSON-RPC request |
| `-32601` | Method Not Found | Unknown method |
| `-32602` | Invalid Params | Invalid method parameters |
| `-32603` | Internal Error | Internal server error |

## ThoughtGate-Specific Errors

### Upstream Errors (-32000 to -32002)

| Code | Name | Description |
|------|------|-------------|
| `-32000` | Upstream Connection Failed | Cannot connect to upstream server |
| `-32001` | Upstream Timeout | Upstream request timed out |
| `-32002` | Upstream Error | Upstream returned an error |

### Policy Errors (-32003)

| Code | Name | Description |
|------|------|-------------|
| `-32003` | Policy Denied | Request denied by Cedar policy |

### Approval Errors (-32007 to -32008)

| Code | Name | Description |
|------|------|-------------|
| `-32007` | Approval Rejected | Human rejected the request |
| `-32008` | Approval Timeout | Approval timed out |

### Rate Limiting (-32009)

| Code | Name | Description |
|------|------|-------------|
| `-32009` | Rate Limited | Too many requests |

### Service Errors (-32013)

| Code | Name | Description |
|------|------|-------------|
| `-32013` | Service Unavailable | ThoughtGate not ready |

## Error Response Format

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32003,
    "message": "Policy denied",
    "data": {
      "tool_name": "admin_console",
      "reason": "Administrative tools are not permitted"
    }
  },
  "id": 1
}
```

## Handling Errors

### Retry-able Errors

These errors may succeed on retry:

- `-32001` Upstream Timeout
- `-32008` Approval Timeout (if resubmitted)
- `-32009` Rate Limited (after backoff)
- `-32013` Service Unavailable

### Non-Retry-able Errors

These errors will not succeed on retry:

- `-32003` Policy Denied
- `-32007` Approval Rejected
- `-32000` Upstream Connection Failed (unless upstream comes back)

## Example Error Handling

```python
import json

def handle_response(response):
    data = json.loads(response)

    if "error" in data:
        code = data["error"]["code"]
        message = data["error"]["message"]

        if code == -32003:
            print(f"Request denied by policy: {message}")
        elif code == -32007:
            print(f"Request rejected by human: {message}")
        elif code == -32008:
            print(f"Approval timed out: {message}")
        elif code in [-32001, -32009, -32013]:
            print(f"Temporary error, retrying: {message}")
            # Implement retry logic
        else:
            print(f"Error {code}: {message}")
    else:
        return data["result"]
```
