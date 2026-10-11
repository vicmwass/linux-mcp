# Scheduling topology migrations

## Independent design gate (2026-10-11)

- Registered Spec 12 with no prerequisites for 12-01; original registries for all other specs remain unchanged except the two additive gates below.
- Versioned `05-dashboard.steps.json` to v2: 05-02 now also requires `12-client-design/12-10`.
- Versioned `11-workflows.steps.json` to v2: 11-06 now also requires `05-dashboard/05-02`, supplying shared components and the transitive design prerequisite.
- Under the shared transaction lock, verified both affected plans were entirely pending and their old registry hashes matched live state. Preserved all existing tokens, claims, histories and completion evidence. Backed up pre-migration state/catalog/registries under the ignored local state directory before writing; validation checked the resulting graph before the state hash update committed.
- Active Spec 00 work was not reset or edited by this migration. All subsequent registries/documents pass the parity and graph validator.

Review/reopen changes attempts and acceptance state, not registry topology, and needs no hash migration. Every spec/step supports it. A future topology migration must stop affected workers, preserve unrelated state, validate the whole graph, retain a recovery copy and record its reason/evidence here.
