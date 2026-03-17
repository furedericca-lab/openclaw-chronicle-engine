---
description: Milestones for distill parity migration.
---

# Scope Milestones

## In-scope

- backend-owned transcript persistence for `session-transcript` distill sources;
- backend-native `session-transcript` execution and replacement tests;
- deterministic reducer parity for duplicate suppression, vague-advice filtering, evidence gating, and stable artifact ranking;
- removal of repo-side sidecar residue and refresh of canonical docs.

## Out-of-scope

- reviving queue-file inboxes, local cursor files, or systemd sidecars;
- provider-driven map/reduce extraction workers;
- changing reflection or ordinary auto-capture semantics beyond the transcript append support needed for distill.

## Milestones With Acceptance Gates

### M1. Contract Freeze

Acceptance gate:

- acceptable parity vs rejected historical shape is frozen in scope docs;
- cleanup gates are explicit for script, test, and example residue;
- backend-owned authority boundaries are explicit.

Status: completed on 2026-03-17.

### M2. Transcript-Source Parity

Acceptance gate:

- `index.ts` forwards `agent_end` transcript rows to `POST /v1/session-transcripts/append`;
- backend persists ordered session transcript rows in SQLite and loads them for `source.kind = session-transcript`;
- backend tests prove transcript-source success and structured source-unavailable failure only when no persisted transcript exists.

Status: completed on 2026-03-17.

### M3. Reducer Parity And Residue Cleanup

Acceptance gate:

- backend reducer applies deterministic dedupe, evidence gating, vague-advice filtering, and ranking;
- old sidecar residue (`scripts/jsonl_distill.py`, `test/jsonl-distill-slash-filter.test.mjs`, `examples/new-session-distill/*`) is removed;
- README and remote backend docs describe the post-cleanup steady state.

Status: completed on 2026-03-17.

## Dependencies Across Milestones

- M1 blocked all implementation work.
- M2 depended on M1 because transcript ownership and cleanup boundaries had to be frozen first.
- M3 depended on M2 because residue removal was only valid after backend transcript execution and tests existed.

## Exit Criteria

- all three milestones are completed;
- active docs no longer describe `session-transcript` as deferred;
- no active repo paths remain for the removed sidecar script/example implementation;
- verification commands in the checklist complete successfully.
