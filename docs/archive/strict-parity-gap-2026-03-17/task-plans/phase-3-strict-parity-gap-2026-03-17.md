---
description: Resolve retained TS helper ownership and close strict parity verification gaps.
---

# Tasks: strict-parity-gap-2026-03-17

## Input

- `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-implementation-research-notes.md`
- `docs/archive/strict-parity-gap-2026-03-17/technical-documentation.md`
- `docs/archive/strict-parity-gap-2026-03-17/strict-parity-gap-2026-03-17-contracts.md`
- `index.ts`
- `src/context/*`
- `src/recall-engine.ts`
- `src/auto-recall-final-selection.ts`
- `src/reflection-recall.ts`
- `test/memory-reflection.test.mjs`
- `test/remote-backend-shell-integration.test.mjs`

## Canonical architecture / Key constraints

- prompt-time rendering/session gating may remain local;
- backend-authority ranking/selection logic should not remain ambiguously split across TS and Rust;
- no deleted local-authority module may be recreated;
- changes must preserve remote-only runtime and current hook behavior unless docs/contracts explicitly change.

## Format

- `[ID] [P?] [Component] Description`
- `[P]` means parallelizable.
- Valid `Component` values: `Backend`, `Frontend`, `Agentic`, `Docs`, `Config`, `QA`, `Security`, `Infra`.
- Every task must include a clear DoD.

## Phase 3: Ownership Cleanup and Strict Verification

Goal: eliminate ambiguity about whether retained TS helpers are intentional prompt-local seams or unfinished backend parity debt, then prove the final strict-parity position with tests.

Definition of Done: TS-vs-Rust ownership is explicit in code/docs/tests, representative strict-parity fixtures pass, and residual non-parity decisions are documented as deliberate acceptable equivalents or non-goals.

Tasks:

- [x] T201 [Backend] Resolve whether any retained TS final-selection helper still owns backend-authority semantics.
  - DoD: touched backend/plugin/docs evidence shows whether migration is actually required; helpers proven prompt-local remain local by explicit acceptable-equivalence rationale, with no new local-authority path introduced.
- [x] T202 [P] [Docs] Freeze explicit disposition for any retained TS helper that stays local.
  - DoD: docs explain why each retained helper is prompt-local only, why it does not violate strict backend authority, and why it is an acceptable parity implementation under the remote Rust design.
- [x] T203 [P] [QA] Add representative strict-parity scenario tests across backend and plugin boundaries.
  - DoD: repo tests cover duplicate-heavy recall, reinforced stale memories, rerank fallback, reflection grouping/selection, and trace visibility with concrete suites/commands.
- [x] T204 [Security] Re-confirm remote-only invariants after ownership cleanup.
  - DoD: tests/docs confirm scope authority, principal handling, and DTO boundaries remain backend-owned and unchanged by parity cleanup.
- [x] T205 [Docs] Update checklist and closeout docs with final strict-gap disposition.
  - DoD: `3phases-checklist.md` records evidence commands/results, closed gaps, accepted non-goals, and any residual risk.

Checkpoint: after Phase 3, strict parity discussion is reduced to explicit accepted differences or hidden implementation debt, not architecture-shape confusion.

## Dependencies & Execution Order

- Phase 3 depends on Phases 1-2.
- `T201` informs `T202` and `T203`.
- `T204` runs after code changes stabilize.
- `T205` closes the scope after evidence is recorded.

## Execution Record

### Implemented

- re-evaluated `src/auto-recall-final-selection.ts` against the remote-authority contract and concluded that current usage is prompt-local post-selection rather than backend-authority ranking;
- kept the helper local and explicitly documented that no Rust migration is required while the module only trims/injects rows already returned by backend recall;
- added plugin-side proof in `test/memory-reflection.test.mjs` showing `setwise-v2` still calls the standard remote recall dependency and only post-processes the returned rows before prompt injection;
- closed the representative scenario matrix using existing backend access-reinforcement/MMR/rerank suites plus new backend trace tests and plugin reflection/auto-recall tests.

### Evidence

- backend:
  - `cargo test --manifest-path /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
- plugin:
  - `node --test --test-name-pattern='.' /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/memory-reflection.test.mjs /root/.openclaw/workspace/plugins/openclaw-chronicle-engine/test/remote-backend-shell-integration.test.mjs`
- key files:
  - `src/context/auto-recall-orchestrator.ts`
  - `src/auto-recall-final-selection.ts`
  - `test/memory-reflection.test.mjs`

### Phase 3 checkpoint result

- completed: retained TS helper ownership is now explicit, `setwise-v2` is frozen as acceptable prompt-local post-selection, and the strict parity scenario matrix is covered by concrete backend/plugin tests.
