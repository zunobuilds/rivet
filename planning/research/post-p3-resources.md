# Shared Resource Controller and tool ownership

Research only. One lightweight logical controller integrated with the Rust supervisor/broker/registry is the approved direction. Detailed algorithms, process placement and metrics are recommendations requiring later implementation authorization. Existing P3b reservations are owner-declared bounded inputs, not this global controller.

## Ownership and crash behavior

Recommend one host-local controller endpoint hosted with trusted core supervision, in-process where projects share a core; private authenticated local IPC otherwise. Do not create a daemon per project/mod. Configurable eager start versus on-demand start, bounded idle shutdown and explicit status are required. Lifecycle/ownership must outlive a detached terminal while any authorized run needs service. Restart verifies controller generation, project owners, grants, active reservations and actual workers/runtime state. Pause fail-closed on uncertainty; reconnect automatically only after safe recovery is established. Never infer a worker died merely because its client disappeared.

A resource request carries protocol version, trusted project/session/agent/request IDs, capability/resource identity, quantity/duration, owner epoch, policy/grant reference, budget reservation and idempotency key. Project supervision authenticates identity and authorizes scope before forwarding; text supplied by a model is not authentication. Controller returns a bounded lease with expiry and fencing generation. Cancellation/revocation invalidates pending admissions immediately; active work checkpoints/terminates under policy. Replayed/stale requests cannot acquire a second lease; orphan cleanup proves exact ownership and cannot kill another project's worker. Audit decisions in each owning project, with minimal global allocation metadata.

## Accounting and fair admission

Track selected tools/dependencies, owned workers, CPU/native RSS/PSS where available, local models, RAM/VRAM and disk/cache/official mods. Remote cloud-model servers are excluded: only local HTTP client overhead belongs here. Provider spending/rate limits stay separate, though both must permit an inference request. A compatible endpoint can be locally hosted or remote; classify explicitly rather than by product name.

| Profile | Proposed policy, not measured performance |
| --- | --- |
| Speed | Keep likely selected assets/models warm within explicit headroom; prioritize interactive latency without exceeding limits. |
| Balanced | Bounded warm set and fair request queues, evict on sustained pressure. |
| Resource Conservation | Smaller warm set, serial admission where needed, earlier idle worker shutdown and conservative preload. |

Profiles are separate from Cookbook Efficient/Balanced/Ambitious preferences. Equal project priority is default. Propose weighted deficit round-robin with aging, bounded per-request quanta where the runtime permits, global/project/agent ceilings and a maximum queue-age diagnostic. Test starvation rather than asserting fairness from algorithm names. User-designated lower priorities may be safely throttled or checkpoint-paused; otherwise queue the new request. Do not suspend an arbitrary process holding locks or interrupt a tool at an unsafe mutation boundary merely to improve priority. CPU/memory/PID cgroups do not provide a universal GPU/VRAM scheduler; runtime-specific support is required. [Kernel controller semantics](https://docs.kernel.org/admin-guide/cgroup-v2.html).

## Model instances and prediction

Support shared full weights with separate session contexts/KV reservations and dedicated instances. Two sessions using an 8B model each access the full 8B, not 4B. Account shared resident weights once per verified actual instance, per-request/session KV, batch scratch, allocator fragmentation, load overlap and OS/tool headroom. Dedicated copies count independently. Shared runtime failure affects all its consumers: mark their possibly sent requests unknown, retain usage holds and re-establish residency safely before retry.

The actual inference runtime owns batching/slots/preemption. Rivet admits authorized requests fairly and respects observed capabilities; it cannot promise fair GPU kernel interleaving or extra throughput. Compare serial versus concurrent prefill/decode, time-to-first-token, tail latency, batch size, KV pressure and cancellation with fixed workloads. [llama.cpp server](https://raw.githubusercontent.com/ggml-org/llama.cpp/master/tools/server/README.md) documents slots/batching and context-related options; these are runtime-specific controls, not Rivet measurements. Provider switching/retries require remaining model/provider/data/cost/usage grants; possibly sent ambiguous requests are not retryable just because another endpoint is available.

Start with deterministic task-aware hints: selected language/tool recipes and bounded recent approved task sequences predict preload/warm/evict. Optional local-only learning can later adjust those hints; expose/reset/delete observations, default no remote telemetry and no compulsory model. Preload consumes memory/I/O and must itself satisfy policy. Use hysteresis/minimum residency to avoid thrashing; retain a negative cache for unsupported assets. Unknown fit queues or reports unsupported rather than guesses.

Libraries linked into the main Rust binary cannot generally be reclaimed independently; allocator arenas and GPU drivers may retain memory. Prefer short-lived isolated specialist workers when their lifecycle warrants it; unload only through qualified runtime APIs after all leases drain. Distinguish cache retention from leaks using repeated quiescent measurements and ownership counters. Under pressure stop preloads, deny new admissions and evict only idle unprotected assets. Recovery/checkpoints are not disposable caches. Disk caps reserve space for receipts/checkpoints before accepting work.

## Tools and shared assets

Recommend a verified supported catalog with immutable tool ID/version/platform/digest/license/provenance, declared effects, dependency closure and smoke tests. Individual/group toggles remove all operational surfaces when disabled, preserving bytes/settings. Acquisition uses compressed upstream distributions only after authorization; bound archive bytes/file counts/expanded size and reject traversal, links, devices and conflicting paths. Retain notices and authenticity evidence; a checksum fetched with an untrusted artifact does not authenticate its author.

Store content-addressed immutable assets with owner references and active leases; maintain global defaults and project pins. User-mandated project-local installation wins over deduplication. No task's executable changes under it. Notifications should verify upstream identity/version/signature policy, show changes and request update approval by default. Configurable later policies may allow signed patch-only updates in a selected window, but compatibility tests, draining, rollback and denial on incomplete trust remain mandatory. Separate tool binaries, rules/databases, model weights and manifests.

Approved agent-installed **unofficial** tools use distinct project/session-scoped namespaces and provenance status. Sandbox/smoke-test them without project secrets, account their resources and never silently add them to the official catalog or grant execution. Unrivet disable/uninstall releases only its ownership references; shared dependencies in use by Rivet survive. Reference counts alone are insufficient after a crash: reconcile ledger owners and leases before garbage collection. No acquisition/update occurred here.

## Proposed qualification

On generic older CPU-only hardware and modern CPU/GPU hardware, record exact configurations without private inventory. Measure startup/idle overhead, p50/p95 queue delay, per-project service, CPU/RAM/VRAM, cold/warm loading, storage, pressure and failure recovery. Include equal/unequal priority, long jobs, cancellation storms, controller crash, replay, stale epochs, unavailable telemetry, leaked workers, disk-full and disabled mods. Compare deterministic predictor off/on with reproducible tasks and no speed claim before measurement. Bound controller queues/indexes and idle timers; disabled features cause zero unnecessary work. Safe recovery and anti-starvation are acceptance gates, not optimization extras.
