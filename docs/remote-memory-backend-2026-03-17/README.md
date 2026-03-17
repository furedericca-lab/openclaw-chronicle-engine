# remote-memory-backend Docs Index

Snapshot refresh: `2026-03-17`

Canonical documents (active):

- `remote-memory-backend-contracts.md`
- `remote-memory-backend-2026-03-17-technical-documentation.md`
- `remote-memory-backend-scope-milestones.md`
- `remote-memory-backend-implementation-research-notes.md`
- `remote-memory-backend-brainstorming.md`

Snapshot clarification:

- this 2026-03-17 snapshot now covers shipped runtime data-plane capabilities such as recall, auto-capture, memory mutation, reflection jobs, transcript persistence, and distill enqueue/status with both `inline-messages` and `session-transcript` execution;
- transcript append is caller-scoped via `POST /v1/session-transcripts/append`, and `agent_end` forwards runtime messages into that backend-owned source of truth;
- the old sidecar/example artifacts have been removed from the active repo runtime and no longer define any supported architecture.

Historical execution artifacts:

- `../archive/remote-memory-backend/`
