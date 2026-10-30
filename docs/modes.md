# Modes and model policies

| Mode | Enforced composition |
| --- | --- |
| Solo | One Independent |
| Pair | One Lead and one Operator |
| Crew | One Lead, one Reviewer, at least one Operator |
| Fleet | One Director, at least one Lead, Reviewer and Operator |
| Mesh | Configured Independent peers, 1–32 in the current template |
| Adaptive | Explicit policy selecting permitted concrete compositions at safe boundaries |

Minimal accepts Solo only. Hierarchies are built into Rivet, not mods. Current graphs are explicitly reviewed; models do not have unrestricted authority to invent tasks or agents.

Preview a model-free graph without executing it:

```bash
.build/target/debug/rivet run template --mode pair --task 'Review a bounded change'
```

`run hierarchy --plan PATH` validates and previews an explicit JSON graph and approval digest. `--execution PATH` adds a model policy to that digest. Execution requires exact approval, configured providers and fresh per-operation reviews. See [P3a](../planning/research/p3a-hierarchies.md), [P3b](../planning/research/p3b-models.md) and [P3c](../planning/research/p3c-mesh-adaptive.md) for the qualified profile.

Shared Model counts full resident weights once per actual declared instance, plus each session's context/KV reservation. Two agents using an 8B model each use the whole 8B; they do not each receive 4B. Dedicated instances count separately. Extra request slots require owner capacity evidence; this is not physical capacity measurement. Concurrent request overlap is synthetic evidence, not a throughput claim. Relay uses serial phases and evidence handoffs; unloaded runtimes require explicit owner-observed activation.

Cookbook ranks whole-hierarchy allocation alternatives from a bounded llmfit 1.1.17 JSON export. Efficient/Balanced/Ambitious express recommendation preferences, distinct from the proposed Resource Controller profiles. `cookbook --help` shows required input files. Cookbook does not execute llmfit, download models or measure hardware.
