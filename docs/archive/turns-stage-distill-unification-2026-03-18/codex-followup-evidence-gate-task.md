# Codex Follow-up Task — evidence gate for stable decision / durable practice

The previous implementation completed the main scope, but review found one blocker:

## Blocker
`stable decision` / `durable practice` promotion currently appears to rely mainly on keyword classification, without a true evidence gate. That violates the approved contract: these outputs should be promoted only when evidence is sufficient.

## Required fix
Add a real evidence gate for `session-lessons` promotion to:
- `Stable decision`
- `Durable practice`

Preferred direction:
- use repeated signal count and/or corroboration across multiple turns/messages
- promotion should require stronger evidence than ordinary `Lesson`
- if evidence is insufficient, fall back to ordinary `Lesson`

## Constraints
- Keep the existing scope decisions unchanged.
- Do not reintroduce reflection-generation behavior.
- Do not weaken the removal of `/new` / `/reset` generation.
- Keep deterministic backend behavior.

## Minimum acceptance criteria
1. A single keyword hit should not be enough to promote to `Stable decision` or `Durable practice`.
2. Promotion should require explicit evidence, such as one or more of:
   - repeated pattern across messages
   - corroborating cause/fix/prevention context
   - repeated/consistent decision phrasing across turns
3. Add backend tests that prove:
   - insufficient evidence => remains `Lesson`
   - sufficient repeated/corroborated evidence => promotes correctly
4. Re-run and report:
   - `cargo test --manifest-path backend/Cargo.toml --test phase2_contract_semantics -- --nocapture`
   - if touched, any affected TS tests

## Delivery
Report:
- exact evidence gate rule implemented
- changed files
- tests added/updated
- verification results
