# strict-parity-gap-2026-03-17 Scope and Milestones

## In Scope

- define a strict historical-TS capability baseline distinct from the narrower archived Rust-closeout baseline;
- classify current capabilities into closed items, strict remaining gaps, and non-gaps;
- plan closure work for:
  - backend diagnostics / traceability parity,
  - TS-vs-Rust ownership cleanup for retained selection/orchestration helpers,
  - stricter parity verification coverage.

## Out of Scope

- restoring deleted local-authority runtime modules;
- changing plugin kind from `memory`;
- expanding normal `/v1` runtime DTOs with internal trace payloads;
- rewriting archive history or deleting preserved historical docs.

## Milestones

### Milestone 1 — Strict parity baseline freeze

Acceptance gate:

- the scope docs define what counts as a strict gap versus a closed item or acceptable Rust-native replacement;
- archive references and current source evidence are tied to concrete file paths;
- implementation phases target only real remaining gaps.

### Milestone 2 — Backend observability/trace parity closure

Acceptance gate:

- backend has a structured retrieval trace story stronger than current event-only diagnostics;
- any new trace surface is explicitly internal/admin/debug scoped;
- tests prove trace visibility does not leak into stable runtime DTOs.

### Milestone 3 — Ownership boundary cleanup for retained TS retrieval helpers

Acceptance gate:

- retained TS helpers are either migrated to Rust or explicitly frozen as prompt-local orchestration only;
- no ambiguous authority-layer ranking logic remains split between backend and local TS;
- index/runtime docs describe the final seam accurately.

### Milestone 4 — Strict parity verification closeout

Acceptance gate:

- fixture-driven tests prove the selected strict parity scenarios and accepted equivalent replacements;
- docs/checklists record the final gap disposition with exact evidence;
- any deliberate non-parity decisions are explicitly accepted as non-goals rather than silent omissions.

## Dependencies

- Milestone 1 blocks all others.
- Milestone 2 depends on Milestone 1 because trace parity cannot be implemented before the target bar is frozen.
- Milestone 3 depends on Milestone 1 and should use Milestone 2 results when trace/ownership concerns overlap.
- Milestone 4 depends on Milestones 2-3.

## Exit Criteria

- the repo has one canonical archived strict-parity gap register under `docs/archive/strict-parity-gap-2026-03-17/`;
- backend observability and retained TS helper ownership are no longer ambiguous;
- engineering can tell, from docs plus tests, what is fully at old-TS parity, what is intentionally different, and why.

## Closeout

Status: completed on `2026-03-17`.

Closeout result:

- Milestone 1 completed: strict parity criteria, representative scenarios, and retained-helper ownership were frozen with architecture-aware acceptance rules.
- Milestone 2 completed: backend traceability parity is now provided by explicit debug-scoped recall trace routes with principal-boundary enforcement and DTO non-leakage coverage.
- Milestone 3 completed: retained TS retrieval helpers were reclassified with explicit ownership; `setwise-v2` remains an acceptable prompt-local post-selection seam rather than hidden backend debt.
- Milestone 4 completed: backend and plugin tests now cover the selected strict-parity scenarios and the final gap disposition is documented across checklist, contracts, technical documentation, and research notes.

Verification evidence:

- `cargo test --manifest-path /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
  - result: `40 passed / 0 failed`
- `node --test --test-name-pattern='.' /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/memory-reflection.test.mjs /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/remote-backend-shell-integration.test.mjs`
  - result: `74 passed / 0 failed`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/strict-parity-gap-2026-03-17`
  - result: `[OK] placeholder scan clean`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/docs/archive/strict-parity-gap-2026-03-17 /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/README.md`
  - result: `[OK] post-refactor text scan passed`
