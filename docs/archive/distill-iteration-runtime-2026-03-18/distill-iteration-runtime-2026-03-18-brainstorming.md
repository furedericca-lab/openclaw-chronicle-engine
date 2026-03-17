---
description: Brainstorming and decision framing for distill-iteration-runtime-2026-03-18.
---

# distill-iteration-runtime-2026-03-18 Brainstorming

## Problem

- Current backend distill is canonical but still too close to single-message truncation.
- Runtime persists transcript rows but does not yet support cadence-based automatic distill.
- The requested scope excludes language-adaptive extraction and wants all remaining deterministic distill upgrades plus every-N-turns automation.

## Scope

- Keep backend-native authority.
- Improve deterministic distill quality inside Rust.
- Add runtime-owned cadence bookkeeping only.

## Constraints

- English-only distill output for this scope.
- No model-backed extraction.
- No sidecar resurrection.

## Options

1. Runtime cadence only
2. Backend deterministic upgrades only
3. Combined backend deterministic upgrades + runtime cadence

## Decision

- Choose option 3 because it closes both the quality and automation gaps without breaking authority boundaries.

## Ownership

- Runtime: cadence counting and enqueue timing
- Backend: transcript source resolution, candidate synthesis, reduction, persistence

## Parity / Migration Notes

- Historical sidecar map-reduce topology remains rejected.
- The parity target is better deterministic extraction quality, not restoration of language-adaptive JSON prompting.

## Risks

- Over-merging neighboring messages can produce noisy spans.
- Runtime cadence is in-memory state and therefore process-local.

## Open Questions

- None blocking for this scope.
