# strict-parity-gap-2026-03-17 Brainstorming

## Problem

The repository has already closed the scoped `rust-rag-completion` work, but that scope explicitly stopped at a narrower parity boundary than the historical TypeScript retrieval stack. The current question is stricter, but it still needs architectural judgment: what capabilities from the old TS system still need acceptable parity in the current Rust + remote-authority design, and which old TS implementation shapes should not be reproduced literally.

Success means:

- the repo has one explicit strict-parity gap register;
- completed items are separated from true remaining gaps;
- implementation phases target closing the remaining gaps rather than reopening already-closed migration work.

## Scope

In scope:

- compare current `backend/src/*`, `index.ts`, `src/context/*`, and retained `src/*.ts` retrieval helpers against historical TS capabilities recorded in archive docs;
- define strict remaining gaps across backend retrieval behavior, diagnostics/traceability, local orchestration residue, and validation coverage, while allowing Rust-native / remote-native acceptable equivalents;
- produce implementation-ready docs and phased task plans whose goal is to close the remaining gaps.

Out of scope:

- changing the public plugin kind;
- inventing new product directions unrelated to old TS parity;
- rewriting archive history;
- implementing the fixes in this documentation batch.

## Constraints

- current remote-authority architecture remains canonical;
- no local-authority fallback may be reintroduced;
- stable `/v1` DTO boundaries must remain narrow unless a gap explicitly requires an additive contract change;
- admin/debug surfaces must stay internal or explicitly contract-scoped, not accidental log leaks;
- historical TS parity must be inferred from preserved docs, current retained TS helper modules, and archived references, not from wishful feature creep;
- parity means capability equivalence or acceptable replacement under the remote-authority Rust architecture, not literal recreation of TS-local implementation shapes.

## Options

### Option A: Treat archive `rust-rag-completion` closeout as sufficient and only note minor polish

Complexity: low.
Migration impact: none.
Reliability impact: low immediate risk, but leaves ambiguity about what "parity" means.
Operational burden: medium, because future work will keep re-litigating whether old TS telemetry/orchestration behavior matters.
Rollback: trivial.

Rejected:

- too weak for the user's request;
- preserves the current ambiguity between "scoped Rust parity" and "strict historical TS parity".

### Option B: Write a strict-gap audit only, without executable follow-up phases

Complexity: medium.
Migration impact: none.
Reliability impact: medium; findings become clearer, but engineering still lacks an agreed closure path.
Operational burden: medium.
Rollback: trivial.

Rejected:

- not sufficient because the user asked for task plans whose goal is to close the gaps.

### Option C: Produce a phased strict-parity doc set with explicit gap register, architecture constraints, and closure plan

Complexity: medium-high.
Migration impact: controlled; only documentation in this batch, but implementation later can follow the phase boundaries.
Reliability impact: high value because it prevents reopening completed migration work while still targeting real residual gaps.
Operational burden: lowest long-term because parity language becomes concrete and testable.
Rollback: doc-only.

Selected.

## Decision

Use a dedicated phased scope `strict-parity-gap-2026-03-17/` that:

- freezes the strict parity definition;
- records evidence for what is already closed;
- lists only real remaining gaps;
- turns those gaps into milestone-driven implementation phases.

This avoids misusing the historical `rust-rag-completion` docs, which were written against a narrower acceptance bar.

## Risks

- archive history contains mixed terminology (`memory-lancedb-pro`, older repo paths, earlier assumptions) and can overstate gaps that are now closed;
- retained TS helper modules may look like stale residue even when they are intentionally part of the current prompt-time orchestration layer;
- "strict parity" can drift into feature inflation unless tied back to historical TS behavior with explicit evidence.

## Open Questions

- whether strict parity should require a first-class admin/debug retrieval trace surface or whether stable internal structured diagnostics plus test-visible hooks are an acceptable Rust-native replacement;
- whether any retained local TS ranking helpers should eventually move into Rust, or remain local because they are prompt-time orchestration rather than backend authority.
