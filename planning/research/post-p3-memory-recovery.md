# Compact memory, human edits, Recycle Bin and export

Research only. Current durable receipts/artifacts and bounded contexts are implementation evidence; the memory layers, change journal, Recycle Bin and export below are designs, not existing commands.

## Memory and audit

Propose four explicit scopes: workspace opt-in shared facts, project facts/decisions, private session conversation/task state and agent working notes. Workspace sharing needs source-project and recipient grants; group membership alone grants nothing. Facts/approved decisions have stable IDs, source references, revision/time, confidence/authority and supersession links. Provisional notes never become trusted instructions through compaction. Agent caches can be discarded; required recovery/security evidence cannot.

Recommend project-owned SQLite rows and bounded FTS indexes for lexical retrieval before embeddings. [SQLite FTS5](https://www.sqlite.org/fts5.html) is an available search mechanism; selected bundled build/features and privacy deletion behavior require testing. No vector DB, embedding service or model is compulsory. Retrieve by current task, scope and source freshness; return a small cited context budget. Deduplicate by source/hash and semantic fact key, not by blindly merging similar-looking decisions. Avoid giant duplicate Markdown memory files. Render optional human-readable exports from structured events.

User inspection/correction/pinning/deletion must propagate to derived indexes/caches and invalidate stale retrieved contexts. Retention has separate classes for disposable notes, optional transcripts, pinned facts and required recovery/audit; disclose backup remnants and any retention conflict instead of promising immediate forensic erasure. Enforce source-project confidentiality on retrieval and workspace sharing. Test bounded index growth, stale facts, conflicting decisions, malicious note injection, cross-project query leakage and compaction retaining recovery/permission evidence.

## Human and unattributed edits

Watch authorized paths while running, debounce notifications and rescan after watcher overflow. Before every write, revalidate expected content/revision regardless of watcher state. Notify Solo in Solo, both participants in the brief's future Duo vocabulary, coordinating Lead/Director in larger hierarchies, plus actual affected writers. For current Pair, ensure both Lead and Operator receive relevant changes; keep Duo/Pair naming unresolved rather than redefining implemented roles.

A project-scoped durable change journal records file identity/path, before/after hashes, bounded diff/artifact references, observation time, source classification and provable actor/session/operation IDs. Rivet can attribute its own receipts; outside writes are external/unattributed unless independently established. Do not claim a watcher identifies a human process reliably. On launch after downtime, compare saved snapshots and current Git/filesystem state and present net changes before resume. Intermediate offline edits and exact edit times may be unknowable. Missing baseline means unknown, not unchanged.

Invalidate affected test/review evidence, cached proposals and contexts. Preserve human edits; never overwrite from a stale revision or auto-reset them. Compact SQLite events render per-project changelogs with agent identity, modifications, checks and decisions. Compress ordinary audit projections without deleting security/recovery authority. Test edit during inference/review/restore, rename/replacement, ignored/untracked changes, watcher overflow and offline net-zero edits (unobservable from snapshots).

## Permission-checked Rivet-owned Recycle Bin

Default proposal for **Rivet-managed** deletions only. No claim to intercept arbitrary external deletion. The file supervisor/broker authorizes deletion and restore; the Resource Controller only accounts storage and requests allowed cleanup. Store original path, project and workspace provenance, actor/session, time, reason, content/hash, mode/ownership metadata where meaningful and source revision. No fixed retention duration has been selected; owner must choose configurable age/caps/privacy policy before implementation defaults.

Proposed transaction: verify source/precondition and permission → reserve storage → stage protected content/metadata → flush and verify durable receipt → remove original only after revalidation → commit deletion receipt. Same-filesystem rename may reduce copying but changes link semantics; cross-filesystem copy requires durable verification before unlink. Crash windows remain reconciled explicitly, not magically atomic across SQLite/filesystems. If storage is full or protection fails, refuse deletion instead of silently permanently deleting. Protect bin roots from tool access and separate original evidence from expendable caches.

Restore previews destination and content; require permission, refuse unsafe parents/escaping links and choose conflict rename or explicit resolution rather than overwriting new content. Preserve symlink bytes without following them; hardlink restoration needs an explicit safe policy and must not mutate another link owner. Handle untracked/ignored files because Git cannot restore them reliably. Privacy rules cover secrets, access permissions, retention and export exclusion. Expiry cannot delete pinned/recovery-required data; significant/destructive/out-of-scope purges need appropriate consent. Test malicious metadata, duplicate restore, changed destination, link/mount replacement, disk-full, cancellation and every stage/receipt crash window.

[Freedesktop Trash specification](https://specifications.freedesktop.org/trash/latest/) is a reference for path/time metadata and trash limitations, not an adopted storage contract or permission to use host trash. Rivet's additional project/actor/recovery metadata and access control need their own versioned schema.

## Standard Git, selective rollback and updates

Use ordinary Git commits, branches, worktrees and diffs plus SQLite/checkpoints/Recycle Bin. Rust adds attribution, preconditions and recovery orchestration; do not invent another VCS or incompatible repository format. Selective rollback identifies one operation/agent's exact diff against its base and current descendants, computes inverse hunks and refuses conflicts/unknown attribution. Preserve later human/agent edits, untracked/ignored data and external effects. Preview is not authority to discard changes. No blanket reset/clean as “undo.”

| Git route | Recommendation and qualification |
| --- | --- |
| System Git via OS | Default candidate: OS maintains security/ABI integration. Upgrades are explicit host actions outside this run; detect version/capabilities and requalify selected operations. |
| Isolated managed stable Git | Optional later candidate from [git/git](https://github.com/git/git) and verified upstream stable release provenance. User-approved pin, source/build/dependency/license receipts, isolated activation and compatibility tests. Never moving master/nightly by default. |
| gitoxide | Rust library alternative for selected operations; [upstream](https://github.com/GitoxideLabs/gitoxide). Needs behavior/security/parity evidence, not assumed CLI equivalence. |
| libgit2 | C library with bindings; [upstream](https://libgit2.org/). Additional native/version surface and GPLv2 linking-exception review; not automatic parity with command-line Git. |

Recommend keeping the existing CLI lane authoritative initially. Qualified updates test object/ref/index/worktree behavior, helpers/hooks/filters/config suppression, errors, cancellation and restore. Pin per environment/project when needed; drain mutations, retain old verified executable for rollback and offer system fallback only if version/capability/grants match. Git remains independently updateable, with normal GitHub compatibility. No fork, upgrade or install occurred here.

## Export

Approved default: reproducible configuration, pinned tool/mod/model references and environment recipe. Export does not automatically fetch huge assets. Exclude secrets, ephemeral credentials, private tokens and host-specific absolute authority. Validate paths/OS/architecture and mark unavailable artifacts, mutable URLs and nonredistributable dependencies honestly; a model name is not a reproducible content pin.

Optional complete archive selects project files, sessions/checkpoints and permitted assets explicitly. Show estimated/actual sizes and licenses; avoid duplicate shared blobs through a manifest. Secret exclusion must cover ledgers/transcripts/artifacts as well as filenames. Untrusted import rejects traversal/links/devices/bombs and never executes hooks or restores old grants. Restore project identity/ownership through a fresh authorized import, not copied privilege. Test recipe replay offline with existing approved assets, archive completeness/hash failures, conflicting paths, incompatible architecture, omitted secrets and missing model/runtime without automatic acquisition.
