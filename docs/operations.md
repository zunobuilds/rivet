# Run controls and recovery

Run these from the same qualified source checkout/project. `UUID`, `PATH`, `DIGEST` and other uppercase arguments below are placeholders, not runnable values. Inspection and preview come before mutation; never paste an old approval digest into a different request.

| Purpose | Current command | Effect / boundary |
| --- | --- | --- |
| Inspect active/durable run | `rivet run status UUID` | Queries current owned run/control state; not a global session browser. |
| Pause | `rivet run pause UUID` | Requests resumable bounded stop; inspect unresolved effects before continuation. |
| Stop | `rivet run stop UUID` | Requests run stop; does not undo completed effects. |
| Preview resume | `rivet run resume UUID` | Returns snapshot/digest; does not renew grants/deadline/budget. |
| Resume reviewed snapshot | `rivet run resume UUID --approve DIGEST` | Revalidates durable conditions and requires fresh operation reviews. |
| Inspect repair state | `rivet repair inspect` | Model-free evidence inspection; no automatic replay/deletion. |
| Backup | `rivet repair backup --approve DIGEST` | Exact inspected state; consistent evidence-backed backup. |
| Recover journals | `rivet repair recover --approve DIGEST` | Preserves raw evidence, fences old owner and reconciles; not silent completion. |
| Inspect uncertain operation | `rivet repair reconcile --operation UUID` | Shows observed effect for a later exact decision. |
| Preview restore | `rivet repair restore NAME` | Requires a known backup and safe ownership/artifacts; later approval is a mutation. |
| Inspect model reservations | `rivet run pool UUID` | Owner-declared instance/resource holds, not live hardware telemetry. |

The executable path in this checkout is `.build/target/debug/rivet`; table commands abbreviate that path. `--state-dir PATH` is supported by these operations and must remain safe and contained. Run each family's `--help` for exact required flags. Repair actions may stop owned workers and mutate recovery state, so they were not run against this repository's real state for documentation validation. Integration fixtures exercise the bounded semantics; native worker-dependent cases remain unrun in the latest audit.

A missing receipt may mean a command or provider request actually executed. Recovery preserves that uncertainty and its reservations; an acknowledgment is not a refund or proof of no effect. Never remove locks, WAL/SHM, blobs or journals to make a run look healthy. Restoring an old checkpoint cannot renew expired authority or safely erase later human edits.

The future confirmed Safe Quit, Force Stop, Recycle Bin and agent-selective `/undo` UX is described in [research](../planning/research/post-p3-memory-recovery.md). It is not implemented by this command table. For permission boundaries see [security](security.md).
