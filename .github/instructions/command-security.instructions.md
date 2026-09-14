---
name: Command and security rules
description: "Apply when editing command parsing, operation classification, resolution, execution analysis, or security policy."
applyTo: "src/commands/**/*.rs,src/execution/analyzer.rs,src/security/**/*.rs"
---

# Command and security rules

- Fail closed for empty, unknown, unsupported, or ambiguously classified commands.
- Keep parser, analyzer, resolver, and policy behavior aligned. A newly classified operation is incomplete until capability validation and policy behavior are defined.
- Preserve the distinction between `Denied`, `RequiresConfirmation`, and an internal processing error.
- Treat compound-command support as security-sensitive. The current parser rejects shell constructs even though the analyzer can inspect some separators; do not widen that surface casually.
- Treat high-risk and destructive operations as confirmation-sensitive, and critical operations as denied unless an explicit policy design changes that rule.
- Normalize command syntax only with tests for prefixes, paths, flags, quoting, and unsupported forms.