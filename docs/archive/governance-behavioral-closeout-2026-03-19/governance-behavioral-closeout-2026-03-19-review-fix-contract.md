# Review Fix Contract: governance-behavioral-closeout-2026-03-19

## Context
A review of the closeout branch found two important issues that should be fixed before merge.

## Findings
1. Removing `.learnings/` read-through/copy compatibility can orphan existing governance backlog history in older workspaces that still only have `.learnings/LEARNINGS.md` and `.learnings/ERRORS.md`.
2. The behavioral debug tool normalizes `details.trace.kind` to `behavioral`, but the user-visible text summary can still print `Trace kind: reflection`, creating contradictory output.

## Goals
- Preserve governance backlog continuity for legacy `.learnings/` workspaces without reopening broad legacy alias support.
- Make the behavioral debug user-visible output consistent with the normalized behavioral surface.
- Add or update regression tests for both fixes.

## Non-goals
- Reintroduce `self_improvement_*` tool aliases.
- Rename backend reflection compatibility routes or storage fields.
- Expand this scope beyond the two reviewed findings.

## Target files / modules
- `src/governance-tools.ts`
- `src/backend-tools.ts`
- `test/governance-tools.test.mjs`
- `test/remote-backend-shell-integration.test.mjs`
- Any nearby helper/test files required for the smallest safe fix

## Constraints
- Keep active public/runtime surface governance + behavioral only.
- Backend compatibility boundary may still retain `reflection` naming internally.
- Prefer the smallest safe patch.

## Verification plan
- `node --test test/governance-tools.test.mjs test/remote-backend-shell-integration.test.mjs`
- If touched assertions require it, run any additional smallest relevant test file(s).

## Rollback
- Revert only the compatibility-import and debug-summary normalization changes if they create regressions.

## Open questions
- None. Use one-time legacy import/read-through behavior only as needed to preserve old backlog continuity.

## Implementation status
- 2026-03-19: completed as a narrow follow-up patch on top of the existing closeout worktree.
- `src/governance-tools.ts`: restore one-time `.learnings/{LEARNINGS,ERRORS}.md` import into `.governance/` when the canonical backlog file is missing or empty.
- `src/backend-tools.ts`: normalize the debug summary trace before rendering so the text surface matches the behavioral `details.trace.kind` surface.
- `test/governance-tools.test.mjs`: add a regression test covering legacy backlog continuity.
- `test/remote-backend-shell-integration.test.mjs`: add a regression assertion that behavioral debug output never prints `Trace kind: reflection`.

## Verification evidence
- `node --test test/governance-tools.test.mjs test/remote-backend-shell-integration.test.mjs`
  Result: passed, 28 tests, 0 failures.

## Boundary note
- Legacy continuity is limited to seeding canonical backlog files from `.learnings/` when `.governance/` has not been populated yet; this patch does not merge divergent canonical and legacy histories.
