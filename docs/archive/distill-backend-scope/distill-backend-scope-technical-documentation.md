---
description: Canonical technical architecture for distill-backend-scope.
---

# distill-backend-scope Technical Documentation

## Canonical Architecture

Current canonical capability split:

1. `reflection`
   - backend-owned async job family
   - input source: `/new` and `/reset` capture payloads
   - output: persisted reflection rows for later recall

2. `auto-capture`
   - backend-owned write/mutation surface
   - input source: ordinary runtime transcript/tool items
   - output: ordinary memory mutation results

3. `distill` (enqueue/status plus initial executor slice shipped)
   - backend-native async job family for transcript-wide lesson extraction or governance-oriented summarization
   - shipped in current batch:
     - `POST /v1/distill/jobs`
     - `GET /v1/distill/jobs/{jobId}`
     - dedicated `distill_jobs` table
     - dedicated `distill_artifacts` table
     - background executor for `inline-messages`
     - artifact persistence population
     - optional persisted lesson-row writes for `persist-memory-rows`
   - still deferred:
     - `session-transcript` source resolution
     - provider-driven extraction/reduce beyond the current deterministic reducer

The existing sidecar distiller example is not canonical runtime architecture.
It is also an explicit cleanup target once backend-native distill parity exists.

## Key Constraints and Non-Goals

Constraints:

- remote backend remains the only authority for persistence, ACL, scope, and job ownership;
- shell may enqueue or provide transcript context, but must not own distill persistence semantics;
- distill enqueue/status must remain non-blocking for user interaction;
- distill must not silently reuse old local `memory-pro import` authority paths.

Non-goals for this scope:

- implementing `session-transcript` source resolution now;
- implementing provider-driven extraction/reduce beyond the current deterministic reducer now;
- replacing reflection with distill;
- replacing auto-capture with distill;
- preserving the sidecar queue directory / systemd worker model as canonical architecture.
- deleting residue immediately before backend-native replacement strategy is frozen.

## Module Boundaries and Data Flow

### Current residue

- `scripts/jsonl_distill.py`
  - useful as transcript-ingestion reference logic
  - not a canonical runtime module
  - temporary migration-reference debt
- `examples/new-session-distill/*`
  - useful as a historical sidecar example
  - not a canonical runtime module
  - example drift debt

Cleanup policy for current residue:

1. keep only what still provides migration-reference value;
2. mark sidecar/example artifacts as non-canonical immediately;
3. archive/remove them once backend-native distill parity exists for the capability they currently illustrate.

### Future backend-native distill boundary

Planned backend-owned stages:

1. transcript source resolution or transcript payload acceptance;
2. incremental ingest / dedupe / cursor semantics;
3. transcript cleaning and noise filtering;
4. chunking;
5. extraction provider call(s);
6. reduce / dedupe / score;
7. persistence and caller-scoped job visibility.

Current shipped executor slice:

1. enqueue request validation;
2. `distill_jobs.status = queued`;
3. background executor transitions `queued -> running`;
4. `inline-messages` source is cleaned, filtered, distilled, and persisted into `distill_artifacts`;
5. optional `persist-memory-rows` writes ordinary backend-owned memory rows;
6. caller-scoped status polling returns `completed|failed` with frozen DTO shape.

Frozen follow-up job model:

1. enqueue request validation;
2. executor transitions job to `running`;
3. backend resolves transcript source and prepares cleaned chunks;
4. provider extraction and reducer produce artifacts;
5. backend persists artifacts and optional memory-row mappings;
6. job transitions to `completed` or `failed`.

Shell/local responsibilities should remain narrow:

- trigger enqueue;
- pass actor context and any explicit transcript input;
- poll status only for caller-scoped diagnostics;
- never persist distilled outputs locally.

Planned cleanup boundary after backend-native distill:

- transcript ingestion/cleaning logic moves into backend-owned modules;
- sidecar queue-file hooks and worker/systemd deployment examples move to archive or are removed;
- sidecar-specific tests are replaced by backend-focused ingest/filter/reduce tests.

Frozen initial source modes:

- `session-transcript`
  - backend resolves transcript from actor/session context
- `inline-messages`
  - shell/runtime supplies explicit transcript items in the enqueue request

Frozen initial distill modes:

- `session-lessons`
- `governance-candidates`

Frozen initial persistence modes:

- `artifacts-only`
- `persist-memory-rows`

## Interfaces and Contracts

Current contract direction:

- distill now follows explicit backend enqueue/status endpoints rather than example queue files;
- transcript cleaning rules and reduction heuristics may be ported from the sidecar, but persistence and ownership must be backend-native.

Contract distinction from existing capabilities:

| Capability | Trigger style | Cost profile | Output shape | Canonical owner |
| --- | --- | --- | --- | --- |
| reflection | `/new` / `/reset` async jobs | moderate | reflection rows | backend |
| auto-capture | ordinary write path | low to moderate | memory mutation results | backend |
| distill | async transcript jobs | moderate to high | lessons / governance artifacts / optional persisted rows | backend |

Frozen initial DTO field expectations:

| DTO | Required fields | Optional fields |
| --- | --- | --- |
| `DistillJobRequest` | `actor`, `mode`, `source.kind`, `options.persistMode` | `source.sessionKey`, `source.sessionId`, `source.messages`, `options.maxMessages`, `options.chunkChars`, `options.chunkOverlapMessages`, `options.maxArtifacts` |
| `DistillJobStatus` | `jobId`, `status`, `mode`, `sourceKind`, `createdAt`, `updatedAt` | `result`, `error` |
| `DistillArtifact` | `artifactId`, `jobId`, `kind`, `category`, `importance`, `text`, `evidence`, `tags` | `persistence.persistMode`, `persistence.persistedMemoryIds` |

Frozen initial job-state machine:

| State | Meaning | Allowed next states |
| --- | --- | --- |
| `queued` | accepted, waiting for executor pickup | `running` |
| `running` | transcript resolution / provider execution / reduction in progress | `completed`, `failed` |
| `completed` | artifacts persisted and optional memory writes finalized | none |
| `failed` | terminal execution failure recorded with structured error | none |

Frozen initial storage recommendation:

- `distill_jobs`
  - primary key `job_id`
  - owner principal fields
  - `mode`
  - `source_kind`
  - `session_key`
  - `session_id`
  - `status`
  - `created_at`
  - `updated_at`
  - `result_summary_json`
  - `error_json`
- `distill_artifacts`
  - primary key `artifact_id`
  - foreign key `job_id`
  - `kind`
  - `category`
  - `importance`
  - `text`
  - `evidence_json`
  - `tags_json`
  - `persistence_json`

Current implementation note:

- `distill_jobs` and `distill_artifacts` tables are now created in SQLite;
- `inline-messages` execution populates both `distill_jobs` terminal state and `distill_artifacts`;
- `session-transcript` requests currently fail with a structured source-unavailable error until backend transcript resolution ships.

## Security and Reliability

- distill jobs must inherit the same caller-principal and ownership discipline as reflection jobs;
- transcript sourcing must not create a shadow principal model through `sessionKey` alone;
- transcript cleaners must strip slash/control/injected-memory noise before provider execution;
- any future cursor/checkpoint state must be backend-owned and auditable;
- idempotency and replay behavior should follow the same write/job-enqueue discipline as other backend async surfaces.
- cleanup must avoid deleting migration-reference logic before equivalent backend tests or implementation plans exist.
- the initial implementation should not reuse reflection job rows or reflection artifact storage for distill state.

## Test Strategy

When implementation begins, tests should be grouped into:

1. transcript ingest tests
   - tail extraction
   - truncation/rotation recovery
   - no duplicate ingestion
2. cleaning/filtering tests
   - slash/control suppression
   - injected memory block stripping
   - oversized dump filtering
3. async contract tests
   - enqueue/status ownership
   - non-blocking behavior
   - idempotency
   - state machine progression `queued -> running -> completed|failed`
4. reduction tests
   - deterministic dedupe
   - evidence-required filtering
   - stable top-k selection
5. persistence/model tests
   - artifact row persistence
   - optional memory-row persistence mapping
   - no reflection-table coupling

Current shipped contract tests:

1. enqueue/status contract tests
   - `POST /v1/distill/jobs` returns `202` + queued DTO
   - `GET /v1/distill/jobs/{jobId}` is caller-scoped
2. validation tests
   - inline-messages source must be non-empty
   - `persist-memory-rows` is rejected for `governance-candidates`
3. executor tests
   - inline-messages distill reaches `completed`
   - artifacts are persisted
   - optional memory rows are persisted
   - slash/control-only input is filtered down to zero artifacts
   - `session-transcript` reaches `failed` with source-unavailable semantics

Future backend test matrix derived from `jsonl_distill.py`:

| Current residue behavior | Current evidence source | Future backend test class | Future backend assertion |
| --- | --- | --- | --- |
| slash-command messages are excluded | `test/jsonl-distill-slash-filter.test.mjs` | ingest/filter unit test | `/note`, `/new`, `/reset`, and slash-prefixed text do not enter distill candidate transcript |
| session-start boilerplate is excluded | `scripts/jsonl_distill.py` noise filters | ingest/filter unit test | startup banners such as `✅ New session started` are dropped before chunking |
| injected memory blocks are stripped | `scripts/jsonl_distill.py` `_clean_text` | transcript cleaning unit test | `<relevant-memories>...</relevant-memories>` is removed before extraction |
| transcript metadata headers are stripped | `scripts/jsonl_distill.py` `_clean_text` | transcript cleaning unit test | `Conversation info` / `Replied message` headers do not survive cleaned transcript output |
| JSON/code-fence metadata blocks are stripped | `scripts/jsonl_distill.py` `_clean_text` | transcript cleaning unit test | fenced JSON blocks are removed before provider input assembly |
| oversized dump/log blocks are skipped | `scripts/jsonl_distill.py` `_is_noise` | ingest/filter unit test | payloads above the configured max text threshold are excluded from distill input |
| excluded agent ids are not ingested | `scripts/jsonl_distill.py` `EXCLUDED_AGENT_IDS` | transcript-source unit test | `memory-distiller` or equivalent excluded agents are never considered transcript sources |
| incremental tail reading avoids re-reading old bytes | `scripts/jsonl_distill.py` cursor logic | stateful ingest integration test | repeated extraction after unchanged file state yields no new transcript work items |
| truncation/rotation resets cursor safely | `scripts/jsonl_distill.py` inode/size checks | stateful ingest integration test | file truncation or rotation does not duplicate old transcript lines and resumes from the correct point |
| partial JSONL line at chunk boundary is not emitted | `scripts/jsonl_distill.py` `_read_jsonl_lines` | transcript reader unit test | incomplete trailing line is deferred until complete rather than emitted as malformed transcript content |
