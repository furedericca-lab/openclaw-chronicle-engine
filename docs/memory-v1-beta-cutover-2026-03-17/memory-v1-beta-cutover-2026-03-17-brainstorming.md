---
description: Brainstorming and decision framing for memory-v1-beta-cutover-2026-03-17.
---

# memory-v1-beta-cutover-2026-03-17 Brainstorming

## Problem

- the codebase still carries migration-era compatibility and naming debt even though the supported runtime is already backend-owned;
- the published version is still `1.1.0-beta.6`, which implies incremental evolution instead of a reset as a new product line;
- `src/query-expander.ts` and `src/reflection-store.ts` still live under top-level `src/`, which makes them look like active authority modules even though only tests import them;
- `src/self-improvement-tools.ts` still ships placeholder `[TODO]` output text.

## Scope

- cut the plugin/package version to `1.0.0-beta.0`;
- remove legacy config compatibility parsing for `sessionMemory.*` and deprecated `memoryReflection.*` fields that were kept only for migration;
- clean up test-only helper placement or naming so retained files stop presenting as runtime authority;
- remove explicit placeholder checklist text from shipped runtime output and align docs/tests with the new baseline.

## Constraints

- keep the backend-owned runtime architecture unchanged;
- do not re-open archived migration scopes or reintroduce local authority paths;
- preserve current supported `sessionStrategy`, reflection enqueue, and prompt-local recall behavior;
- treat this as an intentional breaking beta cutover, not an additive compatibility release.

## Options

### Option 1: Keep compatibility fields but document them harder

Pros:

- lowest implementation risk;
- minimal config churn for operators.

Cons:

- contradicts the new-project cutover intent;
- preserves parsing and test debt that no longer serves the shipped architecture.

### Option 2: Remove legacy config support but keep test-only helpers in top-level `src/`

Pros:

- solves the largest semantic debt first;
- smaller file movement.

Cons:

- still leaves misleading repo layout;
- keeps future audit noise around runtime ownership.

### Option 3: Breaking beta cutover across config, versioning, helper placement, and placeholder-text residue

Pros:

- aligns code, docs, and release semantics;
- removes the remaining migration breadcrumbs that are now misleading;
- gives a clean baseline for future `1.0` stabilization work.

Cons:

- requires coordinated updates across schema, parser, tests, README, changelog, and docs.

## Decision

- choose Option 3;
- treat the work as a breaking beta cutover with explicit version reset to `1.0.0-beta.0`;
- remove migration-only config parsing instead of keeping warn-and-ignore compatibility;
- either relocate or sharply rename retained test-only helpers so the root `src/` surface better reflects runtime ownership;
- remove shipped placeholder checklist output in the same scope because it is low-risk, user-visible debt.

## Ownership

- `index.ts`, `openclaw.plugin.json`, `README.md`, `README_CN.md`, `package.json`, and `CHANGELOG.md` own the version and operator-facing config contract;
- `test/config-session-strategy-migration.test.mjs`, `test/memory-reflection.test.mjs`, and `test/remote-backend-shell-integration.test.mjs` own the cutover regression boundary;
- `src/query-expander.ts`, `src/reflection-store.ts`, and their importing tests own the test-only helper residue decision;
- `src/self-improvement-tools.ts` owns the placeholder-text cleanup.

## Parity / Migration Notes

- this scope explicitly drops migration compatibility instead of extending it;
- archived scopes remain as history, but active docs should stop telling operators to expect legacy field mapping;
- prompt-local recall and reflection-selection seams remain intentional and are not in scope for backend migration changes.

## Risks

- breaking config parsing may invalidate old operator examples or untested private configs;
- helper relocation can create noisy diff churn in tests if not done surgically;
- version reset must stay consistent across package metadata, plugin metadata, and changelog/release notes.

## Open Questions

- whether `src/query-expander.ts` and `src/reflection-store.ts` should move under `test/helpers/` or stay under `src/` with stronger naming;
- whether `CHANGELOG.md` should start a new `1.0.0-beta.0` section or fully reframe prior beta history as pre-reset legacy context.
