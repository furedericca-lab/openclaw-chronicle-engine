---
description: Implementation research notes for distill-iteration-runtime-2026-03-18.
---

# distill-iteration-runtime-2026-03-18 Implementation Research Notes

## Baseline (Current State)

- Runtime already appends transcript rows at `agent_end`.
- Backend distill already owns source cleaning, noise filtering, artifact persistence, and optional memory persistence.
- The remaining quality gap is extraction granularity, not transport.

## Gap Analysis

- Candidate generation is still too close to one-message-at-a-time extraction.
- Summary generation is still shallow truncation.
- Runtime has no built-in cadence-based automatic distill.

## Candidate Designs and Trade-offs

- Deterministic span synthesis is lower risk than reintroducing model-backed extraction.
- Runtime cadence on top of transcript append reuses existing actor/session identity and avoids new authority paths.

## Selected Design

- Add deterministic multi-message span synthesis and merge inside backend windows.
- Keep summaries English-only and rule-based.
- Add optional automatic `session-transcript` distill enqueue every configured N user turns in runtime.

## Ownership Boundary Notes

- Runtime never reads persisted transcript state directly.
- Backend never depends on plugin-local transcript files or queue files.

## Parity / Migration Notes

- Historical map-reduce quality is partially absorbed through deterministic span/window logic, not through prompt-based extraction.

## Residue / Debt Disposition Notes

- Language-adaptive JSON extraction remains deferred debt.

## Validation Plan

- `npm test`
- `cargo test distill_ -- --nocapture`
- scope doc scans

## Risks and Assumptions

- Automatic cadence state is process-local.
- Span heuristics may need later tuning if they over-merge unrelated adjacent messages.
