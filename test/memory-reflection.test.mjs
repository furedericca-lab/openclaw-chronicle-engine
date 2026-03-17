import { describe, it, beforeEach, afterEach } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import jitiFactory from "jiti";

const testDir = path.dirname(fileURLToPath(import.meta.url));
const pluginSdkStubPath = path.resolve(testDir, "helpers", "openclaw-plugin-sdk-stub.mjs");
const jiti = jitiFactory(import.meta.url, {
  interopDefault: true,
  alias: {
    "openclaw/plugin-sdk": pluginSdkStubPath,
  },
});

const pluginModule = jiti("../index.ts");
const memoryLanceDBProPlugin = pluginModule.default || pluginModule;
const { parsePluginConfig } = pluginModule;
const { getDisplayCategoryTag } = jiti("../src/reflection-metadata.ts");
const {
  classifyReflectionRetry,
  computeReflectionRetryDelayMs,
  isReflectionNonRetryError,
  isTransientReflectionUpstreamError,
  runWithReflectionTransientRetryOnce,
} = jiti("../src/reflection-retry.ts");
const { selectPromptLocalAutoRecallResults } = jiti("../src/prompt-local-auto-recall-selection.ts");
const {
  createDynamicRecallSessionState,
  clearDynamicRecallSessionState,
  orchestrateDynamicRecall,
  normalizeRecallTextKey,
} = jiti("../src/recall-engine.ts");
const { renderTaggedPromptBlock, renderErrorDetectedBlock } = jiti("../src/context/prompt-block-renderer.ts");
const { createSessionExposureState } = jiti("../src/context/session-exposure-state.ts");
const { createAutoRecallPlanner } = jiti("../src/context/auto-recall-orchestrator.ts");
const { createReflectionPromptPlanner } = jiti("../src/context/reflection-prompt-planner.ts");
const { shouldSkipRetrieval } = jiti("../src/adaptive-retrieval.ts");

function makeEntry({ timestamp, metadata, category = "reflection", scope = "global" }) {
  return {
    id: `mem-${Math.random().toString(36).slice(2, 8)}`,
    text: "reflection-entry",
    vector: [],
    category,
    scope,
    importance: 0.7,
    timestamp,
    metadata: JSON.stringify(metadata),
  };
}

function baseConfig() {
  return {
    remoteBackend: {
      enabled: true,
      baseURL: "http://backend.test",
      authToken: "token-test",
    },
  };
}

function createPluginApiHarness({ pluginConfig, resolveRoot }) {
  const eventHandlers = new Map();
  const commandHooks = new Map();
  const logs = [];

  const api = {
    pluginConfig,
    resolvePath(target) {
      if (typeof target !== "string") return target;
      if (path.isAbsolute(target)) return target;
      return path.join(resolveRoot, target);
    },
    logger: {
      info(message) {
        logs.push({ level: "info", message: String(message) });
      },
      warn(message) {
        logs.push({ level: "warn", message: String(message) });
      },
      debug(message) {
        logs.push({ level: "debug", message: String(message) });
      },
    },
    registerTool() {},
    registerCli() {},
    registerService() {},
    on(eventName, handler, meta) {
      const list = eventHandlers.get(eventName) || [];
      list.push({ handler, meta });
      eventHandlers.set(eventName, list);
    },
    registerHook(hookName, handler, meta) {
      const list = commandHooks.get(hookName) || [];
      list.push({ handler, meta });
      commandHooks.set(hookName, list);
    },
  };

  return {
    api,
    eventHandlers,
    commandHooks,
    logs,
  };
}

describe("memory reflection", () => {
  describe("adaptive retrieval control prompt skip gate", () => {
    it("skips session-start boilerplate containing /new or /reset", () => {
      const prompt = "A new session was started via /new or /reset. Keep this in mind.";
      assert.equal(shouldSkipRetrieval(prompt), true);
    });

    it("skips startup boilerplate containing session startup sequence text", () => {
      const prompt = "Execute your Session Startup sequence now before continuing.";
      assert.equal(shouldSkipRetrieval(prompt), true);
    });

    it("skips /note handoff/control prompts", () => {
      const prompt = "Control wrapper line\n/note self-improvement (before reset): preserve incident timeline.";
      assert.equal(shouldSkipRetrieval(prompt), true);
    });

    it("does not skip a normal user task prompt", () => {
      const prompt = "Please draft a rollback checklist for mosdns and rclone incidents.";
      assert.equal(shouldSkipRetrieval(prompt), false);
    });
  });

  describe("display category tags", () => {
    it("uses scope tag for reflection entries", () => {
      assert.equal(
        getDisplayCategoryTag({
          category: "reflection",
          scope: "project-a",
          metadata: JSON.stringify({ type: "memory-reflection-item", itemKind: "invariant" }),
        }),
        "reflection:project-a"
      );

      assert.equal(
        getDisplayCategoryTag({
          category: "reflection",
          scope: "project-b",
          metadata: JSON.stringify({
            type: "memory-reflection-item",
            reflectionVersion: 4,
            itemKind: "derived",
          }),
        }),
        "reflection:project-b"
      );
    });

    it("uses scope tag for reflection rows with optional metadata fields", () => {
      assert.equal(
        getDisplayCategoryTag({
          category: "reflection",
          scope: "global",
          metadata: JSON.stringify({
            type: "memory-reflection-item",
            reflectionVersion: 4,
            itemKind: "invariant",
            baseWeight: 1.1,
          }),
        }),
        "reflection:global"
      );

      assert.equal(
        getDisplayCategoryTag({
          category: "reflection",
          scope: "global",
          metadata: JSON.stringify({
            type: "memory-reflection-event",
            reflectionVersion: 4,
            eventId: "refl-test",
          }),
        }),
        "reflection:global"
      );
    });

    it("preserves non-reflection display categories", () => {
      assert.equal(
        getDisplayCategoryTag({
          category: "fact",
          scope: "global",
          metadata: "{}",
        }),
        "fact:global"
      );
    });
  });

  describe("transient retry classifier", () => {
    it("classifies unexpected EOF as transient upstream error", () => {
      const isTransient = isTransientReflectionUpstreamError(new Error("unexpected EOF while reading upstream response"));
      assert.equal(isTransient, true);
    });

    it("classifies auth/billing/model/context/session/refusal errors as non-retry", () => {
      assert.equal(isReflectionNonRetryError(new Error("401 unauthorized: invalid api key")), true);
      assert.equal(isReflectionNonRetryError(new Error("insufficient credits for this request")), true);
      assert.equal(isReflectionNonRetryError(new Error("model not found: gpt-x")), true);
      assert.equal(isReflectionNonRetryError(new Error("context length exceeded")), true);
      assert.equal(isReflectionNonRetryError(new Error("session expired, please re-authenticate")), true);
      assert.equal(isReflectionNonRetryError(new Error("refusal due to safety policy")), true);
    });

    it("allows retry only in reflection scope with zero useful output and retryCount=0", () => {
      const allowed = classifyReflectionRetry({
        inReflectionScope: true,
        retryCount: 0,
        usefulOutputChars: 0,
        error: new Error("upstream temporarily unavailable (503)"),
      });
      assert.equal(allowed.retryable, true);
      assert.equal(allowed.reason, "transient_upstream_failure");

      const notScope = classifyReflectionRetry({
        inReflectionScope: false,
        retryCount: 0,
        usefulOutputChars: 0,
        error: new Error("unexpected EOF"),
      });
      assert.equal(notScope.retryable, false);
      assert.equal(notScope.reason, "not_reflection_scope");

      const hadOutput = classifyReflectionRetry({
        inReflectionScope: true,
        retryCount: 0,
        usefulOutputChars: 12,
        error: new Error("unexpected EOF"),
      });
      assert.equal(hadOutput.retryable, false);
      assert.equal(hadOutput.reason, "useful_output_present");

      const retryUsed = classifyReflectionRetry({
        inReflectionScope: true,
        retryCount: 1,
        usefulOutputChars: 0,
        error: new Error("unexpected EOF"),
      });
      assert.equal(retryUsed.retryable, false);
      assert.equal(retryUsed.reason, "retry_already_used");
    });

    it("computes jitter delay in the required 1-3s range", () => {
      assert.equal(computeReflectionRetryDelayMs(() => 0), 1000);
      assert.equal(computeReflectionRetryDelayMs(() => 0.5), 2000);
      assert.equal(computeReflectionRetryDelayMs(() => 1), 3000);
    });
  });

  describe("runWithReflectionTransientRetryOnce", () => {
    it("retries once and succeeds for transient failures", async () => {
      let attempts = 0;
      const sleeps = [];
      const logs = [];
      const retryState = { count: 0 };

      const result = await runWithReflectionTransientRetryOnce({
        scope: "reflection",
        runner: "embedded",
        retryState,
        execute: async () => {
          attempts += 1;
          if (attempts === 1) {
            throw new Error("unexpected EOF from provider");
          }
          return "ok";
        },
        random: () => 0,
        sleep: async (ms) => {
          sleeps.push(ms);
        },
        onLog: (level, message) => logs.push({ level, message }),
      });

      assert.equal(result, "ok");
      assert.equal(attempts, 2);
      assert.equal(retryState.count, 1);
      assert.deepEqual(sleeps, [1000]);
      assert.equal(logs.length, 2);
      assert.match(logs[0].message, /transient upstream failure detected/i);
      assert.match(logs[0].message, /retrying once in 1000ms/i);
      assert.match(logs[1].message, /retry succeeded/i);
    });

    it("does not retry non-transient failures", async () => {
      let attempts = 0;
      const retryState = { count: 0 };

      await assert.rejects(
        runWithReflectionTransientRetryOnce({
          scope: "reflection",
          runner: "cli",
          retryState,
          execute: async () => {
            attempts += 1;
            throw new Error("invalid api key");
          },
          sleep: async () => { },
        }),
        /invalid api key/i
      );

      assert.equal(attempts, 1);
      assert.equal(retryState.count, 0);
    });

    it("does not loop: exhausted after one retry", async () => {
      let attempts = 0;
      const logs = [];
      const retryState = { count: 0 };

      await assert.rejects(
        runWithReflectionTransientRetryOnce({
          scope: "distiller",
          runner: "cli",
          retryState,
          execute: async () => {
            attempts += 1;
            throw new Error("service unavailable 503");
          },
          random: () => 0.1,
          sleep: async () => { },
          onLog: (level, message) => logs.push({ level, message }),
        }),
        /service unavailable/i
      );

      assert.equal(attempts, 2);
      assert.equal(retryState.count, 1);
      assert.equal(logs.length, 2);
      assert.match(logs[1].message, /retry exhausted/i);
    });
  });

  describe("dynamic recall session state hygiene", () => {
    it("clears per-session state so repeated-injection guard resets after session_end cleanup", async () => {
      const state = createDynamicRecallSessionState({ maxSessions: 16 });
      const run = () => orchestrateDynamicRecall({
        channelName: "unit-dynamic-recall",
        prompt: "Need targeted recall",
        minPromptLength: 1,
        minRepeated: 2,
        topK: 1,
        sessionId: "session-a",
        state,
        outputTag: "relevant-memories",
        headerLines: [],
        loadCandidates: async () => [{ id: "rule-a", text: "Always verify post-checks.", score: 0.9 }],
        formatLine: (candidate) => candidate.text,
      });

      const first = await run();
      assert.ok(first);

      const second = await run();
      assert.equal(second, undefined);

      clearDynamicRecallSessionState(state, "session-a");

      const third = await run();
      assert.ok(third);
    });

    it("bounds tracked sessions by maxSessions to avoid unbounded growth", async () => {
      const state = createDynamicRecallSessionState({ maxSessions: 2 });
      const run = (sessionId) => orchestrateDynamicRecall({
        channelName: "unit-dynamic-recall",
        prompt: "Need targeted recall",
        minPromptLength: 1,
        minRepeated: 0,
        topK: 1,
        sessionId,
        state,
        outputTag: "relevant-memories",
        headerLines: [],
        loadCandidates: async () => [{ id: "rule-a", text: "Keep DNS checks in post-flight.", score: 0.9 }],
        formatLine: (candidate) => candidate.text,
      });

      await run("session-a");
      await run("session-b");
      await run("session-c");

      assert.equal(state.turnCounterBySession.size, 2);
      assert.equal(state.historyBySession.size, 2);
      assert.equal(state.updatedAtBySession.size, 2);
      assert.equal(state.turnCounterBySession.has("session-a"), false);
      assert.equal(state.historyBySession.has("session-a"), false);
      assert.equal(state.updatedAtBySession.has("session-a"), false);
    });
  });

  describe("sessionStrategy cutover contract", () => {
    it("rejects removed sessionMemory fields", () => {
      assert.throws(
        () =>
          parsePluginConfig({
            ...baseConfig(),
            sessionMemory: { enabled: true },
          }),
        /sessionMemory is no longer supported in 1\.0\.0-beta\.0/
      );
    });

    it("defaults to systemSessionMemory when neither field is set", () => {
      const parsed = parsePluginConfig(baseConfig());
      assert.equal(parsed.sessionStrategy, "systemSessionMemory");
    });

    it("defaults auto-recall category allowlist to include other while keeping reflection excluded", () => {
      const parsed = parsePluginConfig(baseConfig());
      assert.deepEqual(parsed.autoRecallCategories, ["preference", "fact", "decision", "entity", "other"]);
      assert.equal(parsed.autoRecallExcludeReflection, true);
    });

    it("defaults Reflection-Recall mode to fixed", () => {
      const parsed = parsePluginConfig({
        ...baseConfig(),
        sessionStrategy: "memoryReflection",
      });
      assert.equal(parsed.memoryReflection.recall.mode, "fixed");
      assert.equal(parsed.memoryReflection.recall.topK, 6);
    });

    it("parses dynamic Reflection-Recall config fields", () => {
      const parsed = parsePluginConfig({
        ...baseConfig(),
        memoryReflection: {
          recall: {
            mode: "dynamic",
            topK: 9,
            includeKinds: ["invariant", "derived"],
            maxAgeDays: 14,
            maxEntriesPerKey: 7,
            minRepeated: 3,
            minScore: 0.22,
            minPromptLength: 12,
          },
        },
      });
      assert.equal(parsed.memoryReflection.recall.mode, "dynamic");
      assert.equal(parsed.memoryReflection.recall.topK, 9);
      assert.deepEqual(parsed.memoryReflection.recall.includeKinds, ["invariant", "derived"]);
      assert.equal(parsed.memoryReflection.recall.maxAgeDays, 14);
      assert.equal(parsed.memoryReflection.recall.maxEntriesPerKey, 7);
      assert.equal(parsed.memoryReflection.recall.minRepeated, 3);
      assert.equal(parsed.memoryReflection.recall.minScore, 0.22);
      assert.equal(parsed.memoryReflection.recall.minPromptLength, 12);
    });
  });

  describe("context split orchestration modules", () => {
    it("renders tagged and error-detected prompt blocks", () => {
      const tagged = renderTaggedPromptBlock({
        tag: "relevant-memories",
        headerLines: [],
        contentLines: ["- [fact:global] keep it concise"],
        wrapUntrustedData: true,
      });
      assert.match(tagged, /<relevant-memories>/);
      assert.match(tagged, /UNTRUSTED DATA/);
      assert.match(tagged, /END UNTRUSTED DATA/);

      const errorBlock = renderErrorDetectedBlock([
        { toolName: "bash", summary: "permission denied" },
      ]);
      assert.match(errorBlock, /<error-detected>/);
      assert.match(errorBlock, /\[bash\] permission denied/);
    });

    it("tracks pending reflection error signals with dedupe and one-shot prompt exposure", () => {
      const state = createSessionExposureState();
      const sessionKey = "agent:main:session:test";
      const signal = {
        at: 1,
        toolName: "bash",
        summary: "permission denied",
        source: "tool_error",
        signature: "permission denied",
        signatureHash: "deadbeef",
      };

      state.addReflectionErrorSignal(sessionKey, signal, true);
      state.addReflectionErrorSignal(sessionKey, signal, true);
      const first = state.getPendingReflectionErrorSignalsForPrompt(sessionKey, 5);
      const second = state.getPendingReflectionErrorSignalsForPrompt(sessionKey, 5);
      assert.equal(first.length, 1);
      assert.equal(second.length, 0);
    });

    it("plans generic auto-recall via dedicated planner module", async () => {
      const state = createSessionExposureState();
      const recallCalls = [];
      const now = Date.now();
      const planner = createAutoRecallPlanner(
        {
          enabled: true,
          minPromptLength: 1,
          topK: 2,
          selectionMode: "mmr",
          categories: ["fact", "reflection"],
          excludeReflection: true,
          maxEntriesPerKey: 5,
          maxAgeDays: 30,
        },
        {
          state: state.autoRecallState,
          recallGeneric: async (params) => {
            recallCalls.push(params);
            return [
              {
                id: "fact-1",
                text: "Always run post-checks after service changes.",
                category: "fact",
                scope: "global",
                score: 0.91,
                metadata: {
                  createdAt: now - 1000,
                  updatedAt: now,
                },
              },
              {
                id: "refl-1",
                text: "Reflection memory should be filtered for generic recall.",
                category: "reflection",
                scope: "global",
                score: 0.89,
                metadata: {
                  createdAt: now - 1000,
                  updatedAt: now,
                },
              },
            ];
          },
          sanitizeForContext: (text) => text,
        }
      );

      const output = await planner.plan({
        prompt: "recall prior rollout guidance",
        agentId: "main",
        sessionId: "session-1",
      });

      assert.equal(recallCalls.length, 1);
      assert.equal(recallCalls[0].agentId, "main");
      assert.equal(recallCalls[0].sessionId, "session-1");
      assert.ok(output);
      assert.match(output.prependContext, /<relevant-memories>/);
      assert.match(output.prependContext, /Always run post-checks/);
      assert.doesNotMatch(output.prependContext, /Reflection memory should be filtered/);
    });

    it("keeps setwise-v2 as prompt-local post-selection over backend rows", async () => {
      const state = createSessionExposureState();
      const recallCalls = [];
      const now = Date.now();
      const planner = createAutoRecallPlanner(
        {
          enabled: true,
          minPromptLength: 1,
          topK: 3,
          selectionMode: "setwise-v2",
          maxEntriesPerKey: 10,
        },
        {
          state: state.autoRecallState,
          recallGeneric: async (params) => {
            recallCalls.push(params);
            return [
              {
                id: "dup-1",
                text: "Verify DNS and mount health after service restart.",
                category: "fact",
                scope: "global",
                score: 0.99,
                metadata: { createdAt: now - 1000, updatedAt: now },
              },
              {
                id: "dup-2",
                text: "Verify dns and mount health after service restart!",
                category: "fact",
                scope: "global",
                score: 0.985,
                metadata: { createdAt: now - 1000, updatedAt: now },
              },
              {
                id: "decision-1",
                text: "Record rollback command before changing service units.",
                category: "decision",
                scope: "global",
                score: 0.95,
                metadata: { createdAt: now - 2000, updatedAt: now - 1000 },
              },
              {
                id: "pref-1",
                text: "Prefer concise post-check summaries in final responses.",
                category: "preference",
                scope: "agent:main",
                score: 0.945,
                metadata: { createdAt: now - 2000, updatedAt: now - 1000 },
              },
            ];
          },
          sanitizeForContext: (text) => text,
        }
      );

      const output = await planner.plan({
        prompt: "Need the restart recovery checklist",
        agentId: "main",
        sessionId: "session-setwise",
      });

      assert.equal(recallCalls.length, 1);
      assert.deepEqual(Object.keys(recallCalls[0]).sort(), [
        "agentId",
        "limit",
        "query",
        "sessionId",
        "sessionKey",
        "userId",
      ]);
      assert.equal(recallCalls[0].limit, 12, "planner should request backend candidates, then trim locally");
      assert.ok(output);
      const lines = output.prependContext
        .split("\n")
        .filter((line) => line.trim().startsWith("- "));
      assert.equal(lines.length, 3);
      assert.equal(lines.filter((line) => /verify dns and mount health/i.test(line)).length, 1);
      assert.ok(lines.some((line) => /rollback command/i.test(line)));
      assert.ok(lines.some((line) => /concise post-check summaries/i.test(line)));
    });

    it("rejects missing remote recall dependency for generic auto-recall planner", () => {
      const state = createSessionExposureState();
      assert.throws(
        () =>
          createAutoRecallPlanner(
            {
              enabled: true,
              topK: 2,
              selectionMode: "mmr",
            },
            {
              state: state.autoRecallState,
              sanitizeForContext: (text) => text,
            }
          ),
        /requires remote recallGeneric dependency/
      );
    });

    it("plans reflection inherited-rules and error reminders via dedicated planner module", async () => {
      const sessionState = createSessionExposureState();
      const planner = createReflectionPromptPlanner(
        {
          injectMode: "inheritance+derived",
          dedupeErrorSignals: true,
          errorReminderMaxEntries: 3,
          errorScanMaxChars: 8000,
          recall: {
            mode: "fixed",
            topK: 4,
            includeKinds: ["invariant"],
            maxAgeDays: 45,
            maxEntriesPerKey: 10,
            minRepeated: 2,
            minScore: 0.18,
            minPromptLength: 8,
          },
        },
        {
          sessionState,
          recallReflection: async () => [
            {
              id: "invariant-1",
              text: "Always verify scope and post-check results before concluding.",
              kind: "invariant",
              scope: "global",
              score: 0.86,
              metadata: {
                timestamp: Date.now(),
              },
            },
          ],
          sanitizeForContext: (text) => text,
        }
      );

      const sessionKey = "agent:main:session:planner-test";
      planner.captureAfterToolCall({ toolName: "bash", error: "permission denied while writing file" }, sessionKey);

      const first = await planner.buildBeforePromptPrependContext({
        prompt: "continue with rollout",
        agentId: "main",
        sessionId: "planner-test",
        sessionKey,
      });
      assert.ok(first);
      assert.match(first, /<inherited-rules>/);
      assert.match(first, /Always verify scope and post-check results before concluding\./);
      assert.match(first, /<error-detected>/);

      const second = await planner.buildBeforePromptPrependContext({
        prompt: "continue with rollout",
        agentId: "main",
        sessionId: "planner-test",
        sessionKey,
      });
      assert.ok(second);
      assert.match(second, /<inherited-rules>/);
      assert.doesNotMatch(second, /<error-detected>/);
    });

    it("rejects missing remote recall dependency for reflection planner", () => {
      const sessionState = createSessionExposureState();
      assert.throws(
        () =>
          createReflectionPromptPlanner(
            {
              injectMode: "inheritance-only",
              dedupeErrorSignals: true,
              errorReminderMaxEntries: 3,
              errorScanMaxChars: 8000,
              recall: {
                mode: "fixed",
                topK: 3,
                includeKinds: ["invariant"],
                maxAgeDays: 45,
                maxEntriesPerKey: 10,
                minRepeated: 1,
                minScore: 0.18,
                minPromptLength: 8,
              },
            },
            {
              sessionState,
              sanitizeForContext: (text) => text,
            }
          ),
        /requires remote recallReflection dependency/
      );
    });

    it("uses one shared session-clear contract for reflection errors and dynamic recall state", () => {
      const clearedReflectionErrors = [];
      const clearedDynamicContext = [];
      const planner = createReflectionPromptPlanner(
        {
          injectMode: "inheritance-only",
          dedupeErrorSignals: true,
          errorReminderMaxEntries: 3,
          errorScanMaxChars: 8000,
          recall: {
            mode: "fixed",
            topK: 3,
            includeKinds: ["invariant"],
            maxAgeDays: 45,
            maxEntriesPerKey: 10,
            minRepeated: 1,
            minScore: 0.18,
            minPromptLength: 8,
          },
        },
        {
          sessionState: {
            autoRecallState: {},
            reflectionRecallState: {},
            clearDynamicRecallForContext: (ctx) => {
              clearedDynamicContext.push(ctx);
            },
            addReflectionErrorSignal() { },
            getPendingReflectionErrorSignalsForPrompt() {
              return [];
            },
            getRecentReflectionErrorSignals() {
              return [];
            },
            clearReflectionErrorSignalsForSession(sessionKey) {
              clearedReflectionErrors.push(sessionKey);
            },
            pruneReflectionSessionState() { },
          },
          recallReflection: async () => [],
          sanitizeForContext: (text) => text,
        }
      );

      planner.clearSession({
        sessionKey: "agent:main:session:planner-reset",
        sessionId: "planner-reset-runtime",
      });

      assert.deepEqual(clearedReflectionErrors, ["agent:main:session:planner-reset"]);
      assert.deepEqual(clearedDynamicContext, [
        {
          sessionKey: "agent:main:session:planner-reset",
          sessionId: "planner-reset-runtime",
        },
      ]);
    });
  });
});
