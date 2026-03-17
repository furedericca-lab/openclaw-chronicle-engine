# Changelog

## 1.0.0-beta.0

- Breaking: reset the plugin/package line to `1.0.0-beta.0` as the post-migration baseline.
- Breaking: remove migration-only config aliases `sessionMemory.*` and deprecated `memoryReflection.agentId/maxInputChars/timeoutMs/thinkLevel`.
- Refactor: move test-only query/reflection reference helpers under `test/helpers/` so top-level `src/` reflects supported runtime ownership.
- Fix: remove placeholder checklist markers from extracted self-improvement skill scaffolds.
- Docs: update README / README_CN and schema text to match the new cutover contract.
