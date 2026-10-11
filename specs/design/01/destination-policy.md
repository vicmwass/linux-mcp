# Spec 01 outbound destination policy

## Policy inputs

Hosted deployments use an explicit allow policy containing permitted ports,
CIDR ranges, and DNS behavior. The default policy denies private, loopback,
link-local, multicast, unspecified, documentation, and cloud-metadata
infrastructure ranges unless the deployment explicitly enables the relevant
self-hosted allowance. Self-hosted deployments may configure private ranges but
still use the same deny-by-default evaluation and connection-time recheck.

The configured policy is not inferred from a user's server record. A syntactically
valid hostname or IP address is only an input to policy evaluation.

## Resolution and connection checks

1. Validate the hostname syntax at the contract boundary.
2. Resolve DNS using the server-side resolver; do not trust client-supplied
   resolved addresses.
3. Evaluate every returned IPv4 and IPv6 address against the configured network
   policy and requested port. If any required address is prohibited or the result
   is empty, fail closed.
4. Keep the policy evidence (hostname, port, resolved addresses, policy revision,
   and evaluation time) with the authorized target, without storing secrets.
5. Resolve and validate again immediately before opening the SSH connection.
   Recheck after DNS changes; never rely solely on an earlier resolver result.

Redirects, proxy settings, alternate address families, and SSH hostnames cannot
weaken this check. The actual address selected by the connection layer must be
covered by the same evidence and policy revision.

## Failure behavior

DNS failures, ambiguous results, blocked ranges, disallowed ports, policy
configuration errors, and policy-revision changes fail closed with sanitized
`unavailable` or `forbidden`/validation errors as appropriate. Provider,
resolver, and OS error strings do not reach public responses or logs.

The destination policy is independent from host-key trust. A permitted network
destination still requires an independently verified host key for the exact
owner/hostname/port target.
