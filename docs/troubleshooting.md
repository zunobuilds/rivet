# Troubleshooting

| Symptom | Action |
| --- | --- |
| Offline build lacks a crate | Stop and obtain approval for source acquisition; do not switch to an uncontained Cargo invocation. |
| Runner lacks a system path/tool | Report the missing prerequisite. Do not install or widen mounts automatically. |
| Provider absent | Review project configuration; no model is acquired automatically. |
| Configured but inference unsupported | Check profile, exact endpoint, context ceiling and supported streaming/tool behavior. Configuration checks do not call the endpoint. |
| Busy state | An owner may still hold the ledger. Use active-run status; do not delete lock/WAL files. |
| Recovery required / unknown usage | Inspect with `repair inspect`; review exact reconciliation. Do not replay a possibly sent request or discard evidence. |
| Resume denied | Check original deadline, budget, permissions and unresolved effects. Resume does not renew them. |
| Worker unsupported | Host enforcement/delegation is not qualified. A sandbox directory probe alone is insufficient; never bypass isolation to pass a check. |
| Stale revision / Mesh conflict | Preserve the changed file, inspect the diff, then explicitly review continuation. Approval cannot make stale evidence current. |
| Copied binary rejects a project | Current build is checkout-bound. General installation and arbitrary external roots remain unqualified. |

Include sanitized command, build digest, failure kind, expected result and minimal repository-contained reproduction when reporting a problem. Remove credentials and private project content. Avoid deleting `.rivet-state/` as a generic repair strategy.

## Exit results

Current typed error mapping: invalid input/configuration exits 2; unsupported capability or denied permission exits 3; interruption exits 130; state/busy/recovery/conflict/Git failures exit 4. JSON diagnostic kinds distinguish these cases where structured output is supported. Capture stderr as well as the exit code and avoid retry loops based only on “nonzero.”

The administration field `execution_available: false` can coexist with `adapters_available: true`; this reports readiness conservatively and does not mean the source has no provider adapter. `sandbox check` probes only the explicit contained directory route; it is not the native systemd worker's readiness certificate. See [build qualification](building.md).
