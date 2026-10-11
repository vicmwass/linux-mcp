# Spec 07 deployment topology and integration matrix

Step 07-01 defines deployable boundaries and the fixture-first release evidence matrix. It does not publish infrastructure or assume a particular cloud provider.

## Service topology and trust boundaries

```mermaid
flowchart LR
    B[Browser / MCP client] --> P[HTTPS reverse proxy]
    P --> D[Dashboard server]
    P --> M[MCP service]
    D --> DB[(PostgreSQL]
    M --> DB
    D --> I[Identity provider]
    M --> E[Execution/provider adapter]
    E --> H[User-owned Linux targets]
    D --> K[Secret provider / master key]
    M --> K
```

The reverse proxy is the only public ingress and terminates TLS, applies request/body limits and forwards an allowlisted `Origin`. Dashboard and MCP are separate processes and service identities. The management API/database owns installation, user, server metadata, revisions and audit references. The MCP service receives a verified principal and calls contracts; it cannot select an owner or read another user's credential. Provider adapters are the only path to SSH/remote machines. The browser never connects directly to SSH, the database, the secret provider or the provider network.

Hosted mode runs dashboard, MCP, database, identity integration and secret provider in one operator-controlled installation with private service networking. Self-hosted mode uses the same boundaries on the user's machine or private network; public ingress is optional and disabled by default. Neither profile binds to a developer's local path or assumes that a saved workstation is reachable. External target access is outbound from the provider adapter over an explicitly allowed network route (VPN/private link or restricted egress).

## Persistence, identity and permissions

Persist database data, migration state and audit records on a dedicated volume. Persist the installation master-key reference only through the configured secret provider; never bake it into an image or Compose file. Credential material and host trust belong to the secret/trust stores with separate read-only mounts or provider handles. Backups include encrypted database data and key metadata, never plaintext secrets.

Run dashboard, MCP and proxy containers as non-root UIDs with read-only images and writable paths limited to temporary files and explicitly mounted data. Database and secret-provider networks are not exposed to the public proxy network. Health probes check dependency reachability and schema readiness without returning credentials or target metadata. A clean self-hosted install creates its own installation identity and issuer; it never trusts another installation's tokens by hostname or copied volume alone.

## Route and profile matrix

| Route | Hosted profile | Self-hosted profile | Owner |
| --- | --- | --- | --- |
| `https://host.example/` | public HTTPS proxy to dashboard | localhost/private proxy or direct dashboard port | deployment/bootstrap |
| `https://host.example/api/v1/*` | proxy to dashboard management API | same route on the local installation | dashboard/identity |
| `https://host.example/mcp` | proxy to separate MCP service | same route, local/private only by default | MCP service |
| database/secret provider | private network only | local socket/private network only | operations |
| remote Linux targets | provider outbound allowlist/VPN | provider outbound allowlist/VPN | execution/inventory |

The hosted and self-hosted configuration schemas are the same. A profile selects ingress and provider wiring; it must not select a fixture identity, disable owner checks or enable global impersonation. Per-installation issuer, audience, installation ID, TLS origin and cookie settings are explicit configuration values.

## Fixture-first integration matrix

| Case ID | Scenario and assertion | Fixture owner | Release gate |
| --- | --- | --- | --- |
| F01 | Clean startup, migrations and readiness; no secret in logs | deployment | 07-02/03 |
| F02 | Alice/Bob inventories and identical host labels stay isolated | identity + inventory | 07-02/05 |
| F03 | Foreign server ID fails before credential/provider access | MCP + execution | 07-02/05 |
| F04 | Credential rotation/trust revision stops stale cache reuse | inventory + execution | 07-05/06 |
| F05 | Prepare/approve/consume once; confirm-only cannot dispatch | governance | 07-05 |
| F06 | Audit outage rejects new mutation and stores no output secret | governance/deployment | 07-05 |
| F07 | Disconnect before submission is definite failure; after submission is unknown and never replayed | execution | 07-05/06 |
| F08 | Grant revocation and session/account change clear dashboard/MCP caches | identity + dashboard | 07-05/06 |
| F09 | Oversized body/output, invalid origin, bad schema and unavailable provider are bounded/redacted | proxy + MCP | 07-04/05 |
| F10 | Backup/restore to isolated installation preserves ownership and rejects wrong key/issuer | operations | 07-07 |
| F11 | Hosted HTTPS named client completes discovery and approval flow against disposable target | MCP + deployment | 07-05 |
| F12 | Self-hosted profile starts with no cloud dependency and uses only configured target routes | deployment | 07-04/08 |

Each case records image/config revision, profile, service versions, identity installation ID, client/version, disposable target identifiers, command/outcome, redacted logs and teardown verification. Fixture passes unlock implementation work; real-token/disposable-target cases remain required for the later release gates.

## Release evidence template

Every integration run records: date, commit/image digests, profile, configuration schema version, migration version, proxy origin/TLS result, service health, test-case IDs, owner isolation result, secret-scan result, target/trust setup, failure/unknown outcome evidence and teardown. A failed or skipped real-adapter case is a blocker, not a fixture success. Public deployment or paid infrastructure remains a separate user-authorized action.
