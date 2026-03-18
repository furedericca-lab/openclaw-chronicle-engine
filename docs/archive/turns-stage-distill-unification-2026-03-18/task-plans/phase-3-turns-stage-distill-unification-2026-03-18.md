---
description: Phase 3 plan for turns-stage-distill-unification-2026-03-18.
---

# Phase 3 — Backend distill absorption

## Goal
Make backend distill the single home for turns-stage lesson extraction, including the retained useful behavior that used to sit under reflection generation.

## Tasks
- Extend `session-lessons` semantics to own lesson/cause/fix/prevention/stable decision/durable practice.
- Keep `governance-candidates` as the promotion-oriented mode and absorb reflection governance value there.
- Represent `Derived` / `Open loops / next actions` as distill-owned artifact subtypes (`follow-up-focus` / `next-turn-guidance`) instead of a separate reflection-generation path.
- Keep source semantics centered on `session-transcript` and ordered turns/messages.
- Add/update backend contract tests for richer lesson extraction outputs and evidence-gated stable decision / durable practice promotion.
- Preserve deterministic evidence aggregation and optional memory persistence.

## Target files
- `backend/src/*`
- `backend/tests/phase2_contract_semantics.rs`
- `README.md`
- `README_CN.md`

## Verification
- `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`

## Done definition
- backend tests prove cadence-friendly turns-stage lesson extraction behavior
- retained reflection-like extraction value is represented as distill-owned output
- no backend contract depends on command-triggered reflection jobs for generation
