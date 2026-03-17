---
description: Brainstorming for finishing distill parity migration under the remote Rust backend architecture.
---

# distill-parity-migration-2026-03-17

## Problem

This document records the scope-open brainstorming state before implementation landed.

At scope open, the repo already shipped backend-native distill jobs for enqueue/status plus an initial `inline-messages` executor, but two meaningful residue pockets still existed:

- transcript-source ingest/filter behavior was represented by `scripts/jsonl_distill.py`;
- richer reduce/dedupe/evidence-aware lesson shaping was represented by `examples/new-session-distill/worker/lesson-extract-worker.mjs`.

Those residues should not be copied blindly into Rust. The target is acceptable parity under the current remote-authority architecture, not a 1:1 resurrection of the sidecar pipeline.

## Migration question

What is the smallest backend-native implementation that:

- closes the remaining distill parity gap;
- keeps authority, persistence, ACL, and job ownership in Rust;
- avoids reviving batch-file outputs, systemd sidecars, or `memory-pro import` persistence;
- enables cleanup/archival of old sidecar residue?

## Frozen planning stance

- backend-native `POST /v1/distill/jobs` and `GET /v1/distill/jobs/{jobId}` remain canonical;
- `inline-messages` support already shipped and is not reopened in this scope except for parity-hardening;
- `session-transcript` should be implemented in a backend-native way, not by exposing file-batch compatibility;
- historical reducer behavior may be replaced by deterministic backend-native summarization/deduping if it preserves useful operator outcomes.

## Scope themes

1. transcript-source parity
   - ingest transcript content without reintroducing a local sidecar authority path;
   - preserve important filtering and cursor semantics where they still matter;
   - reject batch-file output as non-goal.
2. reducer parity
   - absorb useful lesson-candidate normalization/deduping/reduction ideas;
   - do not preserve Gemini-specific sidecar worker shape as architecture.
3. residue cleanup readiness
   - define what can be archived once transcript parity lands;
   - define what can be archived once reducer parity lands.

## Non-goals

- rebuilding the old queue-file inbox;
- reviving external systemd distill worker deployment as supported architecture;
- preserving `memory-pro import` as a supported persistence path;
- exposing transcript files directly as a client-owned authority surface.

## Deliverable

A phased execution plan that makes the remaining parity gap implementable and explicitly classifies any historical behavior as:

- absorb into backend;
- replace with acceptable Rust parity;
- reject and archive.
