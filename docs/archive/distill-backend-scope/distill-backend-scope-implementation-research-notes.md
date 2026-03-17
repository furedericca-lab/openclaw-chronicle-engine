---
description: Implementation research notes for distill-backend-scope.
---

# distill-backend-scope Implementation Research Notes

## Baseline (Current State)

Current canonical backend-owned capabilities already shipped:

- `reflection`
  - enqueue/status routes in `backend/src/lib.rs`
  - backend-owned async execution and persistence
- `auto-capture`
  - `POST /v1/memories/store` with `mode = "auto-capture"`
  - backend-owned extraction/mutation semantics
- `generic recall` / `reflection recall`
  - backend-owned retrieval and ranking
- `distill enqueue/status skeleton`
  - `POST /v1/distill/jobs`
  - `GET /v1/distill/jobs/{jobId}`
  - backend-owned async job-family contract
  - `inline-messages` executor and artifact persistence shipped
  - `session-transcript` source resolution still deferred

Current distill-related residue still in the repo:

| Path | Current role | Current status |
| --- | --- | --- |
| `scripts/jsonl_distill.py` | incremental transcript-tail extractor and batch builder | standalone sidecar preprocessor; not on active runtime path |
| `test/jsonl-distill-slash-filter.test.mjs` | verifies slash/control filtering in the Python script | test coverage for the sidecar preprocessor |
| `examples/new-session-distill/hook/enqueue-lesson-extract/handler.ts` | `/new` hook that writes queue tasks | example-only hook |
| `examples/new-session-distill/worker/lesson-extract-worker.mjs` | Gemini Map-Reduce lesson extractor and importer | example-only worker |
| `examples/new-session-distill/worker/systemd/lesson-extract-worker.service` | systemd deployment example | example-only deployment artifact |

Current cleanup/disposition view:

| Path | Debt class | Recommended disposition |
| --- | --- | --- |
| `scripts/jsonl_distill.py` | migration-reference debt | keep temporarily as reference until backend-native transcript ingest/cleaning parity exists; then archive or delete |
| `test/jsonl-distill-slash-filter.test.mjs` | migration-reference test debt | keep only while `jsonl_distill.py` remains; later port its cases into backend ingest/filter tests |
| `examples/new-session-distill/hook/enqueue-lesson-extract/handler.ts` | example drift debt | demote explicitly as non-canonical example; archive/remove after backend-native distill enqueue exists |
| `examples/new-session-distill/worker/lesson-extract-worker.mjs` | example drift debt | keep as historical reduction reference until backend-native distill reducer exists; then archive |
| `examples/new-session-distill/worker/systemd/lesson-extract-worker.service` | deployment example debt | archive/remove once the sidecar example is no longer recommended |

Important current-state distinction:

- `reflection` and `auto-capture` are active backend contracts;
- `distill` is now an active backend contract for enqueue/status plus `inline-messages` execution;
- the old distiller path still assumes a local/sidecar worker plus `openclaw memory-pro import`.

## Gap Analysis

1. **Transcript distillation now has a backend-native execution slice, but not full source parity yet.**
   - The repo now has backend enqueue/status routes, dedicated `distill_jobs` / `distill_artifacts` tables, and an `inline-messages` executor.
   - This closes the first real execution gap, but `jsonl_distill.py` still carries the `session-transcript` ingest/filter behavior reference until source parity lands.

2. **Useful transcript-ingestion logic still exists only in the sidecar script.**
   - `scripts/jsonl_distill.py` already solves:
     - cursor-based tail extraction;
     - truncation/rotation handling;
     - slash/control/noise filtering;
     - self-ingestion avoidance.
   - These are backend-appropriate primitives if transcript distillation becomes a first-class capability.

3. **The example worker contains useful reduction logic but the wrong authority shape.**
   - The example worker already models:
     - chunking;
     - extraction prompt construction;
     - lesson dedupe;
     - evidence-aware filtering;
   - final shortlist reduction.
   - But it persists through `openclaw memory-pro import`, which belongs to the old sidecar/tooling model rather than the current backend authority model.

3a. **The plan also needs a code-ready DTO and job-state freeze, not only architectural intent.**
   - Without a frozen request/response shape, backend implementation would still reopen design questions around source modes, persistence modes, and artifact/result schemas.
   - Without a frozen state machine, backend implementation could accidentally overload reflection job rows or invent incompatible retry semantics.

4. **The remote backend docs must continue distinguishing reflection, auto-capture, and transcript distill clearly even after the skeleton ships.**
   - Reflection:
     - asynchronous reflective generation around `/new` and `/reset`;
     - output is reflection rows for later recall.
   - Auto-capture:
     - request-time transcript-to-memory mutation path;
     - output is ordinary memory mutations.
   - Distill:
     - enqueue/status plus `inline-messages` execution now shipped;
     - `session-transcript` resolution and richer provider-driven extraction still deferred.

5. **There is cleanup debt, not just capability debt.**
   - The sidecar pipeline is still visible in active repo paths.
   - Without a frozen disposition plan, these files keep reading like semi-supported alternative architecture.

## Candidate Designs and Trade-offs

### Option 1: transcript-source-only distill

Backend-native distill would focus on:

- transcript ingestion;
- cleaning;
- chunk preparation;
- but would still rely on external processing for extraction.

Pros:

- smaller initial backend surface;
- easiest migration from `jsonl_distill.py`.

Cons:

- preserves split execution ownership;
- weak long-term alignment with the remote authority model.

### Option 2: full backend-native distill jobs

Backend-native distill would own:

- transcript ingestion or transcript input contract;
- job enqueue/status;
- provider-driven extraction;
- reduce/dedupe;
- persistence.

Pros:

- strongest authority alignment;
- best observability/auditability story;
- cleanest future architecture.

Cons:

- larger first implementation;
- requires new DTOs and storage decisions.

### Option 3: keep distill example-only and document that decision explicitly

Pros:

- no implementation load;
- simplest present-state story.

Cons:

- throws away useful migration leverage from the existing script/worker logic;
- leaves a long-term capability gap if transcript distill is desired.

## Selected Design

Use **Option 2** as the target architecture for planning.

Planned capability absorption map:

| Existing residue | Keep as-is | Absorb into backend | Reject from target design |
| --- | --- | --- | --- |
| `jsonl_distill.py` cursor/offset/inode handling | no | yes | no |
| `jsonl_distill.py` noise/slash/control filtering | no | yes | no |
| `jsonl_distill.py` batch-file output | no | no | yes |
| example hook non-blocking enqueue idea | no | yes, as backend-native job enqueue semantics | no |
| example worker chunking and reduction logic | no | yes | no |
| example worker `memory-pro import` persistence path | no | no | yes |
| example systemd worker deployment | yes, as historical example only | no | no |

Planned cleanup map:

| Residue | Near-term disposition | Long-term disposition |
| --- | --- | --- |
| `scripts/jsonl_distill.py` | keep as migration reference only | archive/delete after backend-native ingest/filter parity |
| `test/jsonl-distill-slash-filter.test.mjs` | keep as migration-reference coverage | replace with backend ingest/filter tests, then remove |
| `examples/new-session-distill/hook/enqueue-lesson-extract/handler.ts` | mark example-only, non-canonical | archive/remove after backend-native distill enqueue |
| `examples/new-session-distill/worker/lesson-extract-worker.mjs` | keep as reduction-reference example | archive once backend-native reduction logic exists |
| `examples/new-session-distill/worker/systemd/lesson-extract-worker.service` | keep as historical deployment example | archive/remove with the rest of the sidecar example |

Frozen implementation-prep decisions:

| Topic | Frozen decision |
| --- | --- |
| enqueue endpoint | `POST /v1/distill/jobs` |
| status endpoint | `GET /v1/distill/jobs/{jobId}` |
| initial modes | `session-lessons`, `governance-candidates` |
| initial source kinds | `session-transcript`, `inline-messages` |
| initial persist modes | `artifacts-only`, `persist-memory-rows` |
| initial job states | `queued`, `running`, `completed`, `failed` |
| job-table strategy | dedicated `distill_jobs` table |
| artifact persistence strategy | dedicated `distill_artifacts` store/table |
| reflection-table reuse | rejected |
| direct sidecar batch-file compatibility | rejected |

Current implementation batch completed:

| Area | Implemented now |
| --- | --- |
| backend routes | `POST /v1/distill/jobs`, `GET /v1/distill/jobs/{jobId}` |
| validation | source-mode validation, persist-mode restriction, positive-option checks |
| storage | `distill_jobs` and `distill_artifacts` schema creation plus artifact inserts |
| execution | async `inline-messages` cleaning/filtering, deterministic artifact build, optional memory-row persistence |
| security | runtime principal matching and caller-scoped status visibility |
| tests | enqueue/status, validation, execution, persistence, and source-unavailable tests in `backend/tests/phase2_contract_semantics.rs` |

## Validation Plan

Documentation validation:

```bash
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/doc_placeholder_scan.sh docs/distill-backend-scope
bash /root/.openclaw/workspace/skills/repo-task-driven/scripts/post_refactor_text_scan.sh docs/distill-backend-scope README.md
```

Discovery / evidence commands:

```bash
rg -n "jsonl_distill|new-session-distill|distiller|reflection/jobs|auto-capture" docs src test README.md README_CN.md examples scripts
sed -n '1,260p' scripts/jsonl_distill.py
sed -n '1,260p' examples/new-session-distill/worker/lesson-extract-worker.mjs
sed -n '1,260p' docs/remote-memory-backend-2026-03-17/remote-memory-backend-2026-03-17-technical-documentation.md
sed -n '1,260p' docs/remote-memory-backend-2026-03-17/remote-memory-backend-contracts.md
sed -n '1,260p' docs/distill-backend-scope/distill-backend-scope-contracts.md
sed -n '1,240p' test/jsonl-distill-slash-filter.test.mjs
```

Future backend test-porting matrix:

| Current sidecar coverage | Future backend destination |
| --- | --- |
| `test/jsonl-distill-slash-filter.test.mjs` slash/control suppression | backend ingest/filter unit tests |
| `_clean_text` relevant-memory and metadata stripping | backend transcript-cleaning unit tests |
| `_is_noise` startup/log/code-fence suppression | backend ingest/filter unit tests |
| cursor/inode/offset handling | backend stateful ingest integration tests |
| excluded-agent filtering | backend transcript-source unit tests |

## Risks and Assumptions

Assumptions:

- transcript distillation is valuable enough to justify a future backend-native surface;
- the current example worker captures enough useful reduction behavior to inform backend planning without being copied verbatim.

Risks:

- transcript ingestion may require a clearer runtime ownership model than the current session-JSONL sidecar approach;
- the future distill surface could overlap too heavily with reflection unless output semantics are explicitly separated;
- current docs may still over-emphasize the sidecar example unless the remote backend docs and cleanup plan are updated in the same batch.
- the initial DTO freeze may still need one follow-up implementation scope if transcript-source ownership turns out to require gateway/runtime changes.
