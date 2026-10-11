# 09 — Independently delivered administration modules

Implementation: [step-by-step plan and dependencies](plans/09-administration-implementation.md). Follow the [coordination workflow](plans/README.md).

Status: proposed. Implements the broader administration roadmap; no requirement to deliver all modules together.

## Work packages

| Module | Read scope | Guarded mutation scope | Special constraints |
|---|---|---|---|
| Processes | Process details and resource use | Signal/terminate | Bind to process identity/start time, not reusable PID alone |
| Packages | Installed packages, versions, pending updates | Install/remove/update | Distro adapters, exact packages/options, bounded package-manager execution |
| Files | Allowed-root listing, metadata, bounded read | Edit/write and supported permission changes | Canonical path/symlink controls, size limits, content revision checks, defined backups |
| Storage | Disks, mounts, filesystem usage | Separately approved supported changes | Read-first release; destructive operations remain denied unless explicitly designed/tested |
| Network | Interfaces, addresses, listeners, firewall rules | Supported firewall/interface changes | Connectivity-loss warnings, explicit target/rule; recovery only when implemented |
| Scheduled jobs | Owned/authorized cron and systemd timer inspection | Create/update/remove supported entries | Schedule/timezone validation, identity/privilege, revision checks |
| Access | Groups, sudo grants, SSH-key metadata | Supported account/group/key changes | Protected accounts, key validation and clear lock/revocation semantics |

Each row is an independently assignable server-module and dashboard-module pair. Start with Processes, Packages and Files; other rows can begin after their contracts and safety boundaries are agreed. Each owner writes a module-level design listing exact tools, DTO schemas, supported distributions, policy decisions and tests before implementation. This umbrella spec does not authorize unrestricted storage/network/access operations.

## Shared implementation contract

Use 00 types, 03 validation/execution, 04 tool registration, 05 page components and 06 approvals/audit. Freeze schema fixtures first; build tools and pages independently against mocks. Dedicated structured tools are the UI integration boundary, not terminal-output parsing in the browser.

Every mutation binds exact arguments and target revision, rechecks permissions at submission, reports bounded outcomes and performs a supported post-change query. A batch is not automatically atomic. Backups/reversal are described per operation and never assumed. Remote output and file content remain untrusted data.

## Per-module completion gate

Contract tests, parser/validation tests, unsupported-capability fixtures, cross-user access tests, concurrent/stale-resource handling, refusal/approval tests, timeout/unknown-outcome UI and disposable-target verification pass. Navigation enables only delivered tools. The deployment suite runs each selected module without making undelivered modules a release dependency.
