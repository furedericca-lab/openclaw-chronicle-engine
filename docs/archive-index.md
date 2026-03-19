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

- `governance-behavioral-closeout-2026-03-19/`: phased closeout record for removing governance/self-improvement legacy surfaces, restoring legacy `.learnings/` backlog continuity into `.governance/`, normalizing behavioral debug output, and documenting the remaining backend naming boundary. Collected: `2026-03-19` as an archive move after implementation and review-fix completion. Reference value: `8/10`.
  Why: high audit value because it captures the final public-surface cleanup plus the post-review fixes that preserved backlog continuity and eliminated the last active behavioral-vs-reflection debug mismatch.
- `autorecall-governance-unification-2026-03-18/`: phased unification record for introducing canonical `autoRecall` behavioral-guidance wording and governance backlog surfaces before the final alias/shim closeout. Collected: `2026-03-19` as an archive move from the former top-level scope. Reference value: `8/10`.
  Why: high audit value because it records the first semantic unification pass, but it is now superseded by the 2026-03-19 closeout that removed the last public alias/tool/module remnants and archived this scope.
- `distill-authority-closeout-2026-03-18/`: single-contract closeout record for removing the dead backend reflection-provider config surface, tightening manual reflection-row write paths, and demoting remaining top-level snapshot wording that could still be read as command-triggered reflection generation. Collected: `2026-03-18`. Reference value: `7/10`.
  Why: useful as a focused semantic-boundary audit trail because it records the final authority closeout after the larger distill/reflection ownership reset, while keeping misleading-risk lower through explicit superseded labeling rather than broad doc churn.
- `turns-stage-distill-unification-2026-03-18/`: phased contract, implementation, follow-up blocker closeout, and reviewer-audited archive for removing command-triggered reflection generation, consolidating trajectory-derived writes under cadence-driven distill, and introducing evidence-gated `stable decision` / `durable practice` promotion plus distill-owned `follow-up-focus` / `next-turn-guidance` subtypes. Collected: `2026-03-18`. Reference value: `9/10`.
  Why: high implementation and audit value because it records the semantic ownership reset from reflection-generation to distill-only writes, the exact deletion of `/new`/`/reset` reflection generation surfaces, and the post-review evidence gate added before acceptance.
- `distill-iteration-runtime-2026-03-18/`: phased implementation and closeout record for strengthening deterministic Rust distill quality with multi-message span aggregation, English-only rule-based summaries, stronger reduction heuristics, and cadence-based automatic `session-transcript` distill enqueue every configured user-turn interval. Collected: `2026-03-18`. Reference value: `8/10`.
  Why: high implementation and audit value because it records the point where backend-native distill moved beyond message-level truncation, and where runtime gained a bounded automatic distill trigger without reintroducing any sidecar or local transcript authority.
- `src-root-boundary-cleanup-2026-03-17/`: single-contract cleanup record for consolidating self-improvement support into one module, deleting the last thin local recall DTO shell, and moving prompt-time recall helpers fully under `src/context/` to match the context-engine split boundary. Collected: `2026-03-17`. Reference value: `6/10`.
  Why: still useful as a narrow code-structure audit trail, but its uniqueness is lower than the broader architecture scopes and it does not explain current runtime behavior on its own.
- `setwise-v2-removal-2026-03-17/`: single-contract cleanup record for deleting the final prompt-local `setwise-v2` auto-recall selector, collapsing the active runtime to backend-owned `mmr` ranking plus direct truncation, and removing the corresponding schema/test/doc surface. Collected: `2026-03-17`. Reference value: `6/10`.
  Why: useful as a narrow audit trail for the point where the repo stopped treating prompt-local auto-recall post-selection as a supported runtime seam and simplified the pre-release config surface before release.
- `src-test-residual-cleanup-2026-03-17/`: single-contract cleanup record for removing final `src/` and `test/` old-architecture residue after the remote-backend cutover, including backend-owned recall filter forwarding, stale test naming cleanup, consolidation of test-only reflection helpers, and consolidation of self-improvement registration into the main self-improvement tool module. Collected: `2026-03-17`. Reference value: `5/10`.
  Why: useful mainly as a low-level cleanup audit trail; it has limited standalone explanatory power for current architecture and a lower uniqueness score than broader cutover scopes.
- `recall-boundary-tightening-2026-03-17/`: single-contract cleanup record for removing dead runtime noise-filter residue, pushing backend-visible recall filter semantics fully into the backend contract/execution path, and separating self-improvement registration from the remote memory adapter surface. Collected: `2026-03-17`. Reference value: `6/10`.
  Why: still useful as a boundary-tightening checkpoint, but more limited in uniqueness than the broader remote-authority and distill/reflection contract-reset archives.
- `reflection-src-residual-cleanup-2026-03-17/`: single-contract cleanup record for deleting dead `src/reflection*` local-authority residue, inlining the last planner-only type, and moving the remaining test-only reflection helpers under `test/helpers/`. Collected: `2026-03-17`. Reference value: `4/10`.
  Why: still retains some cleanup provenance, but later reflection responsibility changes significantly reduced its direct explanatory value and increased the risk of reading it as a current reflection boundary reference.
- `residual-debt-cleanup-2026-03-17/`: single-contract cleanup record for reconciling historical-snapshot docs wording, deleting an empty top-level docs residue, converting the Chinese noise-filter script into automated coverage, and removing the last stale compatibility phrasing from the active schema text. Collected: `2026-03-17`. Reference value: `3/10`.
  Why: worth keeping only as low-priority cleanup history; its implementation relevance and uniqueness are limited, and most of its useful content is superseded by broader archive scopes.
- `test-reference-cleanup-2026-03-17/`: single-contract cleanup record for deleting obsolete local-authority reference tests after the Rust backend became the sole memory authority. Collected: `2026-03-17`. Reference value: `4/10`.
  Why: keeps some narrow test-history provenance, but its uniqueness is low and it is easy to summarize from broader cleanup/cutover scopes if future condensation is needed.
- `post-beta-cleanup-2026-03-17/`: single-contract cleanup record for demoting misleading top-level historical docs, renaming the last migration-era cutover test path, and removing residual placeholder/template wording after the `1.0.0-beta.0` baseline reset. Collected: `2026-03-17`. Reference value: `4/10`.
  Why: still a valid cleanup record, but largely procedural and less valuable than the larger cutover and architecture archives it follows.
- `memory-v1-beta-cutover-2026-03-17/`: phased cutover record for resetting the plugin/package line to `1.0.0-beta.0`, removing migration-only config aliases, relocating test-only helper residue, and aligning active docs with the post-migration baseline. Collected: `2026-03-17`. Reference value: `8/10`.
  Why: high implementation and audit value because it records the exact break from migration-era compatibility, the release-line reset, and the repository-layout cleanup that clarified which remaining helpers are test-only.
- `memory-backend-gap-closeout-2026-03-17/`: phased implementation and closeout record for the final reflection-source authority transfer, reflection status surface exposure, and compatibility residue cleanup after the backend migration. Collected: `2026-03-17`. Reference value: `6/10`.
  Why: still useful as migration history, but its direct explanatory value dropped after command-triggered reflection generation, reflection-source loading, and reflection-status surfaces were later removed; its misleading risk is now materially higher than a top-tier archive.
- `adapter-surface-closeout-2026-03-17/`: phased closeout record for finishing the plugin/adapter-facing management/debug surfaces and demoting misleading residual TS/runtime artifacts after the remote-authority cutover. Collected: `2026-03-17`. Reference value: `7/10`.
  Why: useful for understanding what the repo considered “adapter-complete” at closeout time, especially the shipped `memory_distill_*` and `memory_recall_debug` surfaces plus the compatibility/debt disposition around residual TS helpers.
- `distill-backend-scope/`: phased planning, contract freeze, remote-backend alignment, and initial runtime implementation closeout for backend-native distill jobs. Collected: `2026-03-17`. Reference value: `7/10`.
  Why: still an important archive for the first backend-native distill contract, but later distill parity work and the 2026-03-18 turns-stage unification now explain the active semantics more directly.
- `distill-parity-migration-2026-03-17/`: phased parity-closeout record for backend-owned transcript persistence, `session-transcript` execution, deterministic reducer alignment, and removal of final JSONL/worker residue. Collected: `2026-03-17`. Reference value: `7/10`.
  Why: still strong history for the backend-owned transcript and reducer transition, but part of its contract surface has since been refined by the 2026-03-18 distill/reflection ownership reset.
- `ts-residual-debt-cleanup-2026-03-17/`: phased audit and cleanup record for relocating test-only TS recall helpers and renaming retained prompt-local seams. Collected: `2026-03-17`. Reference value: `6/10`.
  Why: still helpful for explaining some surviving TS prompt-local seams, but narrower and less unique than the main architecture and ownership-reset scopes.
- `documentation-refresh/`: closeout contract for the documentation cleanup that demoted stale operator docs and surfaced the reduced canonical docs set. Collected: `2026-03-17`. Reference value: `3/10`.
  Why: keep as low-priority documentation housekeeping history only; most of its useful context is summarized or superseded by broader architecture and cleanup scopes.
- `strict-parity-gap-2026-03-17/`: phased strict-parity audit, implementation, and closeout materials for verifying acceptable historical TS capability parity under the Rust + remote architecture. Collected: `2026-03-17`. Reference value: `7/10`.
  Why: useful for understanding why some prompt-local TS seams were retained, but less directly explanatory than the newer ownership-reset scopes and somewhat more vulnerable to over-reading as current guidance.
- `context-engine-split/`: historical phase plans and task docs for the internal context-engine separation work. Collected: `2026-03-12`. Reference value: `6/10`.
  Why: preserves useful rationale for the plugin-side orchestration split, but much of its value is background/context rather than direct explanation of today’s active runtime contract.
- `remote-memory-backend/`: historical implementation handoff, sign-off, verification, and closeout materials for the remote backend migration. Collected: `2026-03-17`. Reference value: `6/10`.
  Why: useful as migration history, but now secondary to the current `remote-memory-backend-2026-03-18/` snapshot and more prone to contract drift if treated as a current design reference.
- `remote-authority-reset/`: phased plans, contracts, research notes, and closeout docs for the remote-only authority cleanup. Collected: `2026-03-16`. Reference value: `7/10`.
  Why: still an important migration archive for the remote-only authority reset, but it now sits one layer behind the active runtime snapshot and later distill/reflection ownership refinements.
- `rust-rag-completion/`: contracts, milestones, parity-gap tracking, and task plans for finishing the Rust backend retrieval pipeline. Collected: `2026-03-16`. Reference value: `8/10`.
  Why: still a high-value archive because it explains the backend retrieval stack’s major convergence path, but it is no longer as directly explanatory for the current runtime contract as the newer 2026-03-18 distill/reflection ownership reset docs.
- `rust-backend-completion-check/`: a focused completion-check contract for validating Rust backend readiness at a specific checkpoint. Collected: `2026-03-16`. Reference value: `4/10`.
  Why: still worth keeping as a checkpoint artifact, but narrower and less reusable than the fuller Rust backend and architecture archives.
- `final-closeout-audit/`: final audit contract used to verify the refactor landed cleanly. Collected: `2026-03-16`. Reference value: `4/10`.
  Why: concise checkpoint-style audit evidence, but comparatively low in uniqueness and largely overlapped by the larger implementation/cutover archives.
- `final-closeout-implementation/`: implementation closeout contract summarizing final cleanup and consolidation work. Collected: `2026-03-16`. Reference value: `4/10`.
  Why: still a valid closeout artifact, but much of its implementation narrative overlaps with broader archive scopes that better explain the current system shape.

Notes:

- `docs/context-engine-split-2026-03-17/` remains a top-level historical architecture/design snapshot.
- `docs/remote-memory-backend-2026-03-18/` is the single current remote backend architecture/design snapshot after the turns-stage distill unification refresh.
- `docs/archive/autorecall-governance-unification-2026-03-18/` is intentionally archived after the 2026-03-19 governance/behavioral closeout removed the remaining active compatibility surfaces.
- Older archive folders remain unchanged as execution/history evidence.
