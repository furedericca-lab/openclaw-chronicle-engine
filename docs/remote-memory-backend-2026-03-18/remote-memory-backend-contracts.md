---
description: Snapshot contract refresh for the remote memory backend after turns-stage distill unification.
---

# remote-memory-backend 2026-03-18 Contracts

## Snapshot purpose

This document records the backend/shell contract after the `turns-stage-distill-unification-2026-03-18` change set.

It exists because the prior `2026-03-17` remote-memory-backend snapshot no longer matches the active contract in several important areas.

## Confirmed divergences from the 2026-03-17 snapshot

### Removed backend/runtime surfaces
The following are no longer part of the supported runtime/backend contract:
- `POST /v1/reflection/source`
- `POST /v1/reflection/jobs`
- `GET /v1/reflection/jobs/{jobId}`
- plugin/runtime command-triggered reflection generation on `/new` and `/reset`
- plugin management tool `memory_reflection_status`
- `memoryReflection.messageCount` config field

### Retained reflection surface
Reflection remains supported only as:
- reflection recall
- prompt-time injection planning
- local reflection session/error state used by prompt orchestration

Reflection no longer owns:
- trajectory-derived knowledge generation
- independent write authority
- async reflection job execution contracts

## Frozen ownership split

### Distill
Distill is the only supported write path that derives new knowledge from trajectories.

#### `session-lessons` owns:
- lesson
- cause
- fix
- prevention
- stable decision
- durable practice

Promotion rule:
- `stable decision` / `durable practice` require at least two distinct evidence messages and either:
  - repeated target phrasing across at least two messages; or
  - corroborating `cause` / `fix` / `prevention` context spanning at least two messages
- otherwise the output must fall back to ordinary `Lesson`

#### `governance-candidates` owns:
- worth-promoting learnings
- skill extraction candidates
- AGENTS/SOUL/TOOLS promotion candidates

#### Distill artifact subtypes
The following are distill-owned artifact subtypes, not reflection persistence kinds:
- `follow-up-focus`
- `next-turn-guidance`

## Trigger contract

### Supported generation trigger
- runtime `agent_end`
- append transcript rows via backend-owned transcript persistence
- evaluate `distill.everyTurns`
- enqueue backend-native distill when the cadence boundary is crossed

### Unsupported generation trigger
- `/new`
- `/reset`
- any command-bound reflection enqueue path

## Config contract

### `memoryReflection`
`memoryReflection` is now recall/injection-only configuration.

Removed generation-era fields are rejected rather than silently ignored:
- `agentId`
- `maxInputChars`
- `timeoutMs`
- `thinkLevel`
- `messageCount`

### Distill config
The active distill config remains centered on:
- `enabled`
- `mode`
- `persistMode`
- `everyTurns`
- `maxMessages`
- `maxArtifacts`
- `chunkChars`
- `chunkOverlapMessages`

## Data-plane contract summary

### Backend-owned and active
- `POST /v1/session-transcripts/append`
- `POST /v1/distill/jobs`
- `GET /v1/distill/jobs/{jobId}`
- `POST /v1/recall/generic`
- `POST /v1/recall/reflection`
- `POST /v1/debug/recall/generic`
- `POST /v1/debug/recall/reflection`
- memory store/update/delete/list/stats routes

### Removed from active contract
- reflection generation/job routes

## Compatibility posture

This snapshot assumes:
- no rollback compatibility for removed reflection-generation behavior
- no compatibility shim for removed config or job surfaces
- active docs should describe the current boundary directly rather than preserving historical dual-surface wording
