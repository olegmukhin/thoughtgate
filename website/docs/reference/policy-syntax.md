---
sidebar_position: 2
---

# Policy Syntax Reference

ThoughtGate uses [Cedar](https://www.cedarpolicy.com/) for policy definitions.

## Policy Structure

```cedar
permit|forbid(
    principal,
    action == Action::"<action>",
    resource
) when {
    <conditions>
} advice {
    <metadata>
};
```

## Actions

| Action | Description |
|--------|-------------|
| `Action::"tools/list"` | List available tools |
| `Action::"tools/call"` | Call a specific tool |
| `Action::"prompts/list"` | List available prompts |
| `Action::"prompts/get"` | Get a specific prompt |
| `Action::"resources/list"` | List available resources |
| `Action::"resources/read"` | Read a specific resource |

## Resource Attributes

For `tools/call` actions:

| Attribute | Type | Description |
|-----------|------|-------------|
| `resource.tool_name` | String | Name of the tool being called |
| `resource.arguments` | Object | Tool arguments (JSON) |

## Conditions

### String Comparisons

```cedar
when { resource.tool_name == "delete_user" }
when { resource.tool_name != "safe_tool" }
when { resource.tool_name.startsWith("admin_") }
when { resource.tool_name.endsWith("_dangerous") }
when { resource.tool_name.contains("delete") }
```

### Set Membership

```cedar
when { resource.tool_name in ["delete_user", "drop_table", "send_email"] }
```

### Numeric Comparisons

```cedar
when { resource.arguments.amount > 10000 }
when { resource.arguments.count >= 5 }
when { resource.arguments.priority < 3 }
```

### Boolean Logic

```cedar
when {
    resource.tool_name == "transfer" &&
    resource.arguments.amount > 1000
}

when {
    resource.tool_name == "delete" ||
    resource.tool_name == "remove"
}

when {
    !(resource.tool_name in ["safe_tool_1", "safe_tool_2"])
}
```

## Advice (Metadata)

Advice tells ThoughtGate how to handle matching requests:

```cedar
advice {
    "require_approval": true,
    "approval_channel": "#high-value-ops",
    "approval_timeout": "10m"
}
```

| Key | Type | Description |
|-----|------|-------------|
| `require_approval` | Boolean | Require human approval |
| `approval_channel` | String | Override default Slack channel |
| `approval_timeout` | String | Override default timeout (e.g., `5m`, `1h`) |

## Policy Evaluation Order

1. Policies are evaluated in file order
2. First matching `forbid` denies the request
3. First matching `permit` allows (with advice)
4. No match = implicit deny

## Examples

### Allow everything except admin tools

```cedar
forbid(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name.startsWith("admin_")
};

permit(
    principal,
    action,
    resource
);
```

### Tiered approval thresholds

```cedar
// High-value: requires approval
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name == "transfer_funds" &&
    resource.arguments.amount > 10000
} advice {
    "require_approval": true,
    "approval_channel": "#finance-approvals"
};

// Medium-value: just log
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name == "transfer_funds" &&
    resource.arguments.amount > 1000
};

// Low-value: allow
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name == "transfer_funds"
};
```

### Deny list with exceptions

```cedar
// Allow safe operations explicitly
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name in ["get_balance", "list_accounts"]
};

// Deny all other tools/call
forbid(
    principal,
    action == Action::"tools/call",
    resource
);

// Allow tools/list
permit(
    principal,
    action == Action::"tools/list",
    resource
);
```

## Validation

Validate policies before deployment:

```bash
cedar validate --policies policy.cedar
```

## Hot Reload

ThoughtGate watches the policy file and reloads on changes. No restart required.
