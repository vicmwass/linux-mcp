# Operation fixtures (step 00-05)

- `intent.json` — Alice's `delete_user` intent on her server at revision 2. `intent.digest` pins its `lmcp-canon-v1` binding digest; a change means the canonicalization or binding changed and requires a version bump.
- `outcomes.json` — one example of every `OperationOutcome` variant, including non-zero exit (still `succeeded`), policy refusal, approval required, pre-submission timeout/host-key failure and unknown-after-submission (disconnect and timeout with truncated partial output).

Fixture values are synthetic; no fixture contains a real secret, host or user.
