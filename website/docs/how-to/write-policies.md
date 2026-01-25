---
sidebar_position: 3
---

# Write Cedar Policies

This guide shows you how to write Cedar policies to control which MCP tool calls require approval.

## Basic Structure

A Cedar policy has three parts:

```cedar
permit(
    principal,           // Who is making the request
    action == Action::"tools/call",  // What action
    resource             // The tool being called
) when {
    // Conditions
} advice {
    // Metadata for ThoughtGate
};
```

## Allow All Tool Listings

```cedar
permit(
    principal,
    action == Action::"tools/list",
    resource
);
```

## Require Approval for Specific Tools

```cedar
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name in ["delete_user", "drop_table", "send_email"]
} advice {
    "require_approval": true,
    "approval_channel": "#sensitive-ops"
};
```

## Deny Specific Tools

```cedar
forbid(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name == "admin_console"
};
```

## Conditional Approval Based on Arguments

```cedar
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name == "transfer_funds" &&
    resource.arguments.amount > 10000
} advice {
    "require_approval": true,
    "approval_timeout": "10m"
};
```

## Multiple Policies

Policies are evaluated in order. The first matching policy wins.

```cedar
// 1. Always deny admin tools
forbid(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name.startsWith("admin_")
};

// 2. Require approval for destructive operations
permit(
    principal,
    action == Action::"tools/call",
    resource
) when {
    resource.tool_name in ["delete", "drop", "remove"]
} advice {
    "require_approval": true
};

// 3. Allow everything else
permit(
    principal,
    action == Action::"tools/call",
    resource
);
```

## Testing Policies

Use the Cedar CLI to validate syntax:

```bash
cedar validate --schema schema.cedarschema --policies policy.cedar
```

## Hot Reloading

ThoughtGate watches the policy file for changes. Updates are applied without restart.

```bash
# Modify policy.cedar
# ThoughtGate logs: "Policy reloaded successfully"
```

## Next Steps

- See the full [Policy Syntax Reference](/docs/reference/policy-syntax)
- Understand [Traffic Tiers](/docs/explanation/traffic-tiers)
