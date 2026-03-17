# Docs Archive

This index maps the historical planning, phase, task, and superseded architecture materials preserved under `docs/archive/`.

## Archive Value Rule

Use this simple reference score when deciding whether an old document set should be kept, condensed, or eventually removed.

Score formula: `value = U + I + R - M`

- `U` (uniqueness, `0..4`): how much irreplaceable historical context or decision record the item keeps.
- `I` (implementation relevance, `0..3`): how directly the item helps explain code that still exists or current architecture that evolved from it.
- `R` (review / audit value, `0..3`): how useful the item is for postmortem, migration tracing, or verifying why a refactor happened.
- `M` (misleading risk, `0..2`): how likely the item is to confuse readers if treated as current guidance.

Interpretation:

- `8..10`: high-value archive, keep intact.
- `5..7`: useful archive, keep but do not surface as active guidance.
- `2..4`: low-value archive, candidate for future condensation into summaries.
- `0..1`: minimal value, candidate for removal if duplicated elsewhere.

Archived scopes:

- `memory-v1-beta-cutover-2026-03-17/`: phased cutover record for resetting the plugin/package line to `1.0.0-beta.0`, removing migration-only config aliases, relocating test-only helper residue, and aligning active docs with the post-migration baseline. Collected: `2026-03-17`. Reference value: `8/10`.
  Why: high implementation and audit value because it records the exact break from migration-era compatibility, the release-line reset, and the repository-layout cleanup that clarified which remaining helpers are test-only.
- `memory-backend-gap-closeout-2026-03-17/`: phased implementation and closeout record for the final reflection-source authority transfer, reflection status surface exposure, and compatibility residue cleanup after the backend migration. Collected: `2026-03-17`. Reference value: `8/10`.
  Why: high implementation and audit value because it records the final removal of plugin-local reflection source recovery, the shipped `POST /v1/reflection/source` contract, and the disposition of the remaining compatibility-only memory configuration surface.
- `adapter-surface-closeout-2026-03-17/`: phased closeout record for finishing the plugin/adapter-facing management/debug surfaces and demoting misleading residual TS/runtime artifacts after the remote-authority cutover. Collected: `2026-03-17`. Reference value: `7/10`.
  Why: useful for understanding what the repo considered “adapter-complete” at closeout time, especially the shipped `memory_distill_*` and `memory_recall_debug` surfaces plus the compatibility/debt disposition around residual TS helpers.
- `distill-backend-scope/`: phased planning, contract freeze, remote-backend alignment, and initial runtime implementation closeout for backend-native distill jobs. Collected: `2026-03-17`. Reference value: `8/10`.
  Why: high implementation and audit value because it records the transition from sidecar distill residue to the canonical backend-native distill contract, including the first shipped `inline-messages` executor slice. Remaining transcript-source and reducer parity work moved to `docs/archive/distill-parity-migration-2026-03-17/`.
- `distill-parity-migration-2026-03-17/`: phased parity-closeout record for backend-owned transcript persistence, `session-transcript` execution, deterministic reducer alignment, and removal of final JSONL/worker residue. Collected: `2026-03-17`. Reference value: `8/10`.
  Why: high implementation and audit value because it captures the final distill parity boundary, the backend-owned transcript authority decision, and the exact cleanup that retired the last local sidecar/script assumptions.
- `ts-residual-debt-cleanup-2026-03-17/`: phased audit and cleanup record for relocating test-only TS recall helpers and renaming retained prompt-local seams. Collected: `2026-03-17`. Reference value: `7/10`.
  Why: useful for understanding why residual TS files still exist, which ones were moved to tests, and which names were normalized to reflect prompt-local ownership.
- `documentation-refresh/`: closeout contract for the documentation cleanup that demoted stale operator docs and surfaced the reduced canonical docs set. Collected: `2026-03-17`. Reference value: `5/10`.
  Why: useful as a narrow audit trail for the docs reduction pass, but lower-value than the architecture and implementation scopes it summarizes.
- `strict-parity-gap-2026-03-17/`: phased strict-parity audit, implementation, and closeout materials for verifying acceptable historical TS capability parity under the Rust + remote architecture. Collected: `2026-03-17`. Reference value: `8/10`.
  Why: strong audit and implementation value because it records the final traceability parity decision and the accepted ownership boundary for retained TS prompt-local helpers.
- `context-engine-split/`: historical phase plans and task docs for the internal context-engine separation work. Collected: `2026-03-12`. Reference value: `7/10`.
  Why: preserves rationale for the plugin-side orchestration split, though much of it is execution-detail-heavy.
- `remote-memory-backend/`: historical implementation handoff, sign-off, verification, and closeout materials for the remote backend migration. Collected: `2026-03-17`. Reference value: `8/10`.
  Why: still helpful for understanding backend API migration and rollout sequencing.
- `remote-authority-reset/`: phased plans, contracts, research notes, and closeout docs for the remote-only authority cleanup. Collected: `2026-03-16`. Reference value: `8/10`.
  Why: high audit value and strong linkage to the current remote-only architecture, but misleading if read as current execution guidance.
- `rust-rag-completion/`: contracts, milestones, parity-gap tracking, and task plans for finishing the Rust backend retrieval pipeline. Collected: `2026-03-16`. Reference value: `9/10`.
  Why: strongest code-history linkage among the archive sets; explains how the current backend retrieval stack reached its present shape.
- `rust-backend-completion-check/`: a focused completion-check contract for validating Rust backend readiness at a specific checkpoint. Collected: `2026-03-16`. Reference value: `5/10`.
  Why: narrow checkpoint value, useful for audit, but less comprehensive than the full Rust RAG archive.
- `final-closeout-audit/`: final audit contract used to verify the refactor landed cleanly. Collected: `2026-03-16`. Reference value: `6/10`.
  Why: concise audit evidence with moderate historical value.
- `final-closeout-implementation/`: implementation closeout contract summarizing final cleanup and consolidation work. Collected: `2026-03-16`. Reference value: `6/10`.
  Why: useful closing summary, but much of its detail overlaps with other archived scope docs.

Notes:

- The 2026-03-15/2026-03-17 architecture-reset snapshots were moved to top-level `docs/context-engine-split-2026-03-15/` and `docs/remote-memory-backend-2026-03-17/` for easier direct access.
- Older archive folders remain unchanged as execution/history evidence.
