---
description: Implementation research notes for remote-authority-reset.
---

# remote-authority-reset Implementation Research Notes

## Baseline (Current State)

Repository baseline:
- Rust backend implementation lives under `backend/`.
- OpenClaw plugin/runtime wiring lives primarily in `index.ts`, `src/backend-client/*`, `src/backend-tools.ts`, `src/tools.ts`, and `openclaw.plugin.json`.
- Local prompt-time context orchestration lives under `src/context/*`.
- Top-level integration and compatibility docs live in `README.md` and `README_CN.md`.
- Historical planning and architecture docs previously lived in:
  - `docs/context-engine-split/*`
  - `docs/remote-memory-backend/*`

Observed mismatch before this reset:
- `context-engine-split` described an internal extraction/refactor track.
- `remote-memory-backend` described the remote-authority target architecture.
- Together they were technically compatible, but the naming made them look like two active architecture tracks rather than one target architecture plus one enabling refactor.

## Gap Analysis

### Documentation gap
- The previous active docs did not present a single explicit architecture sentence that contributors could treat as the final truth.
- README references pointed at the two old active scopes, which reinforced the split narrative.
- Archive guidance existed, but archive contents and active canonical contents were still conceptually mixed.

### Architecture-language gap
- The desired architecture is a three-layer system:
  1. remote Rust backend,
  2. thin OpenClaw adapter,
  3. local context-engine.
- The previous naming hid this simple model behind project-history language.

### Cleanup-planning gap
- There was no single scoped plan that said what to archive, what to keep, what to rename, what code boundaries to preserve, and what to remove or simplify in follow-up work.
- Existing closeout docs (`docs/final-closeout-audit/*`, `docs/final-closeout-implementation/*`) identified cleanup themes but did not replace the architecture narrative.

## Candidate Designs and Trade-offs

### Design 1 — Preserve old scopes and add a meta-summary
Pros:
- small patch;
- minimal churn.

Cons:
- future contributors still need to map old track names onto the actual desired architecture;
- leaves ambiguity over whether context-engine is a product-level architecture track or a local subsystem.

### Design 2 — Create a new canonical architecture scope and archive the previous two scopes
Pros:
- makes the final architecture explicit;
- preserves history without treating it as current truth;
- gives implementation a clean cleanup/refactor plan.

Cons:
- requires moving docs and updating references;
- can temporarily leave stale references in auxiliary docs until follow-up cleanup is done.

### Design 3 — Make `remote-memory-backend` the only active scope and describe context-engine only inside technical docs
Pros:
- very simple top-level naming.

Cons:
- under-communicates the fact that prompt-time orchestration is intentionally local and first-class;
- increases the risk that future backend work drifts into prompt assembly responsibilities.

## Selected Design

Choose **Design 2**.

Implementation intent:
- archive `docs/context-engine-split/*` and `docs/remote-memory-backend/*` under a dated archive path;
- create `docs/remote-authority-reset/*` as the new canonical architecture and cleanup/refactor plan;
- update top-level references to point to the new scope;
- use the new scope to drive later code cleanup, naming simplification, and test reshaping.

## Validation Plan

Documentation validation for this batch:
- `find docs/archive/2026-03-15-architecture-reset -maxdepth 2 -type f | sort`
- `find docs/remote-authority-reset -maxdepth 2 -type f | sort`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/remote-authority-reset`
- `bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/remote-authority-reset README.md`
- `git diff --check`

Follow-up implementation validation expected after refactor phases land:
- `node --test test/remote-backend-shell-integration.test.mjs`
- `node --test test/memory-reflection.test.mjs`
- `node --test test/config-session-strategy-migration.test.mjs`
- `npm test`

## Risks and Assumptions

Assumptions:
- the repository should keep historical docs for auditability, not delete them;
- the target architecture keeps prompt rendering and session-local orchestration in TypeScript;
- the backend remains the sole authority for storage and retrieval semantics.

Risks:
- some non-updated docs may continue to reference archived paths;
- local-mode compatibility may remain broader in code than the target architecture wants;
- user-facing naming may still lag behind the new architecture until phased cleanup reaches README/schema/log strings.
