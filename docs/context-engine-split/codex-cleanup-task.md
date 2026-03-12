Continue the context-engine-split branch with a minimal cleanup patch only.

Repo:
- /root/verify/memory-lancedb-pro-context-engine-split
- branch: feat/context-engine-split

Previous state:
- Phase 1-4 docs/checklist are already completed in this worktree.
- Phase 2-4 verification already passed.
- Do NOT rework the architecture broadly.

Cleanup goals (only these):
1. Fix the corrupted/garbled text in `README_CN.md` around the `src/context/prompt-block-renderer.ts` description.
2. Apply a minimal structure cleanup for `src/context/reflection-prompt-planner.ts`:
   - move reflection error parsing / normalization / redaction helper logic into a dedicated file,
   - keep planner behavior unchanged,
   - keep public config/tool/plugin contracts unchanged.
3. Keep the patch as small and reviewable as possible.
4. Re-run verification:
   - `npm test`

Constraints:
- No new architecture changes beyond the helper extraction.
- No public config key changes.
- No plugin kind change.
- No README overclaiming.
- Keep claims grounded in actual file paths.

Suggested target shape:
- new file like `src/context/reflection-error-signals.ts` (or similarly precise name)
- `reflection-prompt-planner.ts` should delegate to it instead of owning all helper logic inline

Deliverable summary:
- exact touched files
- what was cleaned up
- test result
- any remaining tiny reviewer notes
