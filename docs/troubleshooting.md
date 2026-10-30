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
