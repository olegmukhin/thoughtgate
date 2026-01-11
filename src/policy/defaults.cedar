// Default permissive policies for development
//
// Implements: REQ-POL-001/F-007 (Embedded Default Policy)
//
// WARNING: These policies permit ALL actions on ALL resources.
// DO NOT USE IN PRODUCTION.
//
// For production, create custom policies in a ConfigMap or
// set THOUGHTGATE_POLICIES environment variable.

// Permit all StreamRaw actions (Green Path)
permit(
    principal,
    action == ThoughtGate::Action::"StreamRaw",
    resource
);

// Permit all Inspect actions (Amber Path)
permit(
    principal,
    action == ThoughtGate::Action::"Inspect",
    resource
);

// Permit all Approve actions (Approval Path)
permit(
    principal,
    action == ThoughtGate::Action::"Approve",
    resource
);
