import { afterEach, describe, it } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
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
const {
  buildBackendCallContext,
  resolveBackendCallContext,
  MissingRuntimePrincipalError,
} = jiti("../src/backend-client/runtime-context.ts");

const cleanupStack = [];
afterEach(() => {
  while (cleanupStack.length > 0) {
    const fn = cleanupStack.pop();
    try {
      fn();
    } catch {
      // best-effort test cleanup
    }
  }
});

function withCleanup(fn) {
  cleanupStack.push(fn);
}

function makeTempRoot() {
  const root = mkdtempSync(path.join(tmpdir(), "remote-backend-shell-test-"));
  withCleanup(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

function makeRemoteConfig(root, overrides = {}) {
  const base = {
    dbPath: path.join(root, "memory-db"),
    autoCapture: true,
    sessionStrategy: "none",
    selfImprovement: { enabled: false },
    enableManagementTools: false,
    remoteBackend: {
      enabled: true,
      baseURL: "http://backend.test",
      authToken: "token-test",
      timeoutMs: 2000,
      maxRetries: 0,
      retryBackoffMs: 25,
    },
  };
  return {
    ...base,
    ...overrides,
    selfImprovement: { ...base.selfImprovement, ...(overrides.selfImprovement || {}) },
    remoteBackend: { ...base.remoteBackend, ...(overrides.remoteBackend || {}) },
    memoryReflection: overrides.memoryReflection
      ? { ...(overrides.memoryReflection || {}) }
      : undefined,
  };
}

function createPluginApiHarness({ pluginConfig, resolveRoot }) {
  const eventHandlers = new Map();
  const commandHooks = new Map();
  const toolFactories = new Map();
  const cliRegistrations = [];
  const logs = [];

  const addHandler = (map, name, handler, meta) => {
    const list = map.get(name) || [];
    list.push({ handler, meta });
    map.set(name, list);
  };

  const api = {
    pluginConfig,
    config: pluginConfig,
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
    registerTool(factory, meta) {
      const name = typeof meta?.name === "string" ? meta.name : factory({}).name;
      toolFactories.set(name, factory);
    },
    registerCli(cli, meta) {
      cliRegistrations.push({ cli, meta });
    },
    registerService() { },
    on(eventName, handler, meta) {
      addHandler(eventHandlers, eventName, handler, meta);
    },
    registerHook(hookName, handler, meta) {
      addHandler(commandHooks, hookName, handler, meta);
    },
  };

  return {
    api,
    eventHandlers,
    commandHooks,
    toolFactories,
    cliRegistrations,
    logs,
    instantiateTool(name, toolCtx = {}) {
      const factory = toolFactories.get(name);
      assert.ok(factory, `missing registered tool: ${name}`);
      return factory(toolCtx);
    },
  };
}

function getLatestHandler(map, name) {
  const list = map.get(name) || [];
  assert.ok(list.length > 0, `missing handler: ${name}`);
  return list[list.length - 1].handler;
}

function normalizeHeaders(headers) {
  if (!headers) return {};
  if (headers instanceof Headers) {
    return Object.fromEntries([...headers.entries()].map(([k, v]) => [k.toLowerCase(), String(v)]));
  }
  if (Array.isArray(headers)) {
    return Object.fromEntries(headers.map(([k, v]) => [String(k).toLowerCase(), String(v)]));
  }
  return Object.fromEntries(
    Object.entries(headers).map(([k, v]) => [String(k).toLowerCase(), String(v)])
  );
}

function jsonResponse(payload, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function installFetchMock(routes) {
  const calls = [];
  const originalFetch = globalThis.fetch;

  globalThis.fetch = async (input, init = {}) => {
    const url = typeof input === "string" ? input : input.url;
    const method = String(init.method || "GET").toUpperCase();
    const pathName = new URL(url).pathname;
    const headers = normalizeHeaders(init.headers);
    const body = typeof init.body === "string" && init.body.trim().length > 0
      ? JSON.parse(init.body)
      : undefined;

    const call = { url, method, path: pathName, headers, body };
    calls.push(call);

    const route = routes.find((candidate) => {
      const expectedMethod = String(candidate.method || "GET").toUpperCase();
      if (expectedMethod !== method) return false;
      if (candidate.path instanceof RegExp) return candidate.path.test(pathName);
      return candidate.path === pathName;
    });

    assert.ok(route, `unhandled fetch route: ${method} ${pathName}`);
    const result = await route.reply(call, calls.length - 1);
    if (result instanceof Response) return result;
    if (result && typeof result === "object" && Object.prototype.hasOwnProperty.call(result, "status")) {
      const status = Number(result.status || 200);
      const payload = Object.prototype.hasOwnProperty.call(result, "body") ? result.body : {};
      return jsonResponse(payload, status);
    }
    return jsonResponse(result || {}, 200);
  };

  withCleanup(() => {
    globalThis.fetch = originalFetch;
  });

  return { calls };
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

describe("remote backend shell integration", () => {
  it("accepts remote mode config without local embedding block", () => {
    const parsed = parsePluginConfig({
      remoteBackend: {
        enabled: true,
        baseURL: "http://backend.test",
        authToken: "token-test",
      },
    });
    assert.equal(parsed.remoteBackend?.enabled, true);
    assert.equal(parsed.embedding, undefined);
  });

  it("rejects local mode config without embedding at parse-time with a clear mode-aware error", () => {
    assert.throws(
      () => parsePluginConfig({ autoCapture: true }),
      /embedding config is required when remoteBackend is disabled \(local mode\)/
    );
  });

  it("builds backend runtime context from real principal identity and blocks synthesized principals", () => {
    const defaults = {
      sessionIdPrefix: "memory-backend",
    };

    const explicit = buildBackendCallContext(
      {
        userId: "user-1",
        agentId: "agent-1",
        sessionId: "session-runtime-1",
        sessionKey: "agent:agent-1:session:logical-1",
      },
      defaults
    );

    assert.equal(explicit.identity.userId, "user-1");
    assert.equal(explicit.identity.agentId, "agent-1");
    assert.equal(explicit.actor.userId, "user-1");
    assert.equal(explicit.actor.agentId, "agent-1");
    assert.equal(explicit.actor.sessionId, "session-runtime-1");
    assert.equal(explicit.actor.sessionKey, "agent:agent-1:session:logical-1");

    const inferred = resolveBackendCallContext(
      {
        sessionKey: "agent:agent-from-key:session:stable-key",
      },
      defaults
    );

    assert.equal(inferred.hasPrincipalIdentity, false);
    assert.deepEqual(inferred.missingPrincipalFields, ["userId", "agentId"]);
    assert.equal(inferred.context.identity.userId, "");
    assert.equal(inferred.context.identity.agentId, "");
    assert.equal(inferred.context.actor.sessionKey, "agent:agent-from-key:session:stable-key");
    assert.match(inferred.context.actor.sessionId, /^memory-backend-/);

    assert.throws(
      () => buildBackendCallContext({ sessionKey: "agent:agent-from-key:session:stable-key" }, defaults),
      (error) => {
        assert.ok(error instanceof MissingRuntimePrincipalError);
        assert.deepEqual(error.missingPrincipalFields, ["userId", "agentId"]);
        return true;
      }
    );
  });

  it("registers remote memory tools and forwards recall/store/forget/update without local scope authority payloads", async () => {
    const root = makeTempRoot();
    assert.equal(existsSync(path.join(root, "memory-db")), false);
    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/recall/generic",
        reply: () => ({
          rows: [
            {
              id: "mem-r1",
              text: "Always write post-check commands after risky infra changes.",
              category: "decision",
              scope: "agent:agent-7",
              score: 0.95,
              metadata: { createdAt: Date.now(), updatedAt: Date.now() },
            },
          ],
        }),
      },
      {
        method: "POST",
        path: "/v1/memories/store",
        reply: () => ({
          results: [
            {
              id: "mem-s1",
              action: "ADD",
              text: "Use local proxy 127.0.0.1:17890",
              category: "fact",
              importance: 0.9,
              scope: "agent:agent-7",
            },
          ],
        }),
      },
      {
        method: "POST",
        path: "/v1/memories/delete",
        reply: () => ({ deleted: 1 }),
      },
      {
        method: "POST",
        path: "/v1/memories/update",
        reply: () => ({
          result: {
            id: "123e4567-e89b-12d3-a456-426614174000",
            action: "UPDATE",
            text: "Use local proxy 127.0.0.1:17890 and keep LAN in NO_PROXY",
            category: "decision",
            importance: 1,
            scope: "agent:agent-7",
          },
        }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoCapture: false,
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);
    assert.equal(
      existsSync(path.join(root, "memory-db")),
      false,
      "remote mode should not initialize local LanceDB storage path on register"
    );

    assert.equal(harness.cliRegistrations.length, 0, "remote mode should disable local memory-pro CLI");

    for (const toolName of ["memory_recall", "memory_store", "memory_forget", "memory_update"]) {
      assert.ok(harness.toolFactories.has(toolName), `expected tool registration: ${toolName}`);
    }

    const toolCtx = {
      userId: "user-7",
      agentId: "agent-7",
      sessionId: "session-runtime-7",
      sessionKey: "agent:agent-7:session:stable-7",
      context: { userId: "user-7" },
    };

    const recall = harness.instantiateTool("memory_recall", toolCtx);
    const recallResult = await recall.execute("call-1", {
      query: "post-check command after risky change",
      limit: 4,
      category: "decision",
    });
    assert.equal(recallResult.details.count, 1);

    const store = harness.instantiateTool("memory_store", toolCtx);
    const storeResult = await store.execute("call-2", {
      text: "Use local proxy 127.0.0.1:17890",
      category: "fact",
      importance: 1,
    });
    assert.equal(storeResult.details.action, "add");

    const forget = harness.instantiateTool("memory_forget", toolCtx);
    const forgetResult = await forget.execute("call-3", {
      memoryId: "mem-s1",
    });
    assert.equal(forgetResult.details.action, "deleted");
    assert.equal(forgetResult.details.deleted, 1);

    const update = harness.instantiateTool("memory_update", toolCtx);
    const updateResult = await update.execute("call-4", {
      memoryId: "123e4567-e89b-12d3-a456-426614174000",
      text: "Use local proxy 127.0.0.1:17890 and keep LAN in NO_PROXY",
      category: "decision",
      importance: 1,
    });
    assert.equal(updateResult.details.action, "updated");

    const [recallCall, storeCall, forgetCall, updateCall] = fetchMock.calls;
    assert.equal(fetchMock.calls.length, 4);

    assert.equal(recallCall.path, "/v1/recall/generic");
    assert.deepEqual(Object.keys(recallCall.body).sort(), ["actor", "limit", "query"]);
    assert.equal(recallCall.body.actor.userId, "user-7");
    assert.equal(recallCall.body.actor.agentId, "agent-7");
    assert.equal(recallCall.body.actor.sessionId, "session-runtime-7");
    assert.equal(recallCall.body.actor.sessionKey, "agent:agent-7:session:stable-7");

    assert.equal(storeCall.path, "/v1/memories/store");
    assert.deepEqual(Object.keys(storeCall.body).sort(), ["actor", "memory", "mode"]);
    assert.equal(storeCall.body.mode, "tool-store");

    assert.equal(forgetCall.path, "/v1/memories/delete");
    assert.deepEqual(Object.keys(forgetCall.body).sort(), ["actor", "memoryId"]);

    assert.equal(updateCall.path, "/v1/memories/update");
    assert.deepEqual(Object.keys(updateCall.body).sort(), ["actor", "memoryId", "patch"]);

    for (const call of fetchMock.calls) {
      assert.equal(call.headers["x-auth-user-id"], "user-7");
      assert.equal(call.headers["x-auth-agent-id"], "agent-7");
      assert.ok(typeof call.headers["idempotency-key"] === "string" || call.path === "/v1/recall/generic");
      assert.equal("scope" in (call.body || {}), false);
      assert.equal("scopeFilter" in (call.body || {}), false);
    }
  });

  it("keeps remote recall fail-open when runtime principal identity is unavailable", async () => {
    const root = makeTempRoot();
    const fetchMock = installFetchMock([]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoCapture: false,
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const recall = harness.instantiateTool("memory_recall", {
      sessionKey: "agent:agent-missing:session:stable-missing",
    });
    const result = await recall.execute("call-missing-principal-recall", {
      query: "should skip recall without runtime principal",
    });

    assert.equal(fetchMock.calls.length, 0);
    assert.equal(result.details.error, "missing_runtime_principal");
    assert.equal(result.details.skipped, true);
    assert.deepEqual(result.details.missingPrincipalFields, ["userId", "agentId"]);
    assert.match(result.content[0].text, /Remote recall skipped/);
    assert.ok(
      harness.logs.some((entry) => entry.level === "warn" && entry.message.includes("memory_recall skipped")),
      "missing principal should remain visible in logs"
    );
  });

  it("fails remote write paths closed when runtime principal identity is unavailable", async () => {
    const root = makeTempRoot();
    const fetchMock = installFetchMock([]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoCapture: false,
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const store = harness.instantiateTool("memory_store", {
      sessionKey: "agent:agent-missing:session:stable-missing",
    });
    const result = await store.execute("call-missing-principal-store", {
      text: "should be blocked",
      category: "fact",
    });

    assert.equal(fetchMock.calls.length, 0);
    assert.equal(result.details.error, "missing_runtime_principal");
    assert.deepEqual(result.details.missingPrincipalFields, ["userId", "agentId"]);
    assert.match(result.content[0].text, /blocked because runtime principal identity is unavailable/);
  });

  it("forwards auto-capture through backend mode=auto-capture with actor context", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/memories/store",
        reply: () => ({
          results: [
            {
              id: "mem-cap-1",
              action: "ADD",
              text: "Keep NO_PROXY aligned with LAN ranges",
              category: "fact",
              importance: 0.8,
              scope: "agent:agent-auto",
            },
          ],
        }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoCapture: true,
        captureAssistant: false,
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const handler = getLatestHandler(harness.eventHandlers, "agent_end");
    await handler(
      {
        success: true,
        messages: [
          { role: "user", content: [{ type: "text", text: "Keep NO_PROXY aligned with LAN ranges." }] },
          { role: "assistant", content: [{ type: "text", text: "Acknowledged." }] },
          { role: "user", content: "Do not route /mnt/azure through proxies." },
        ],
      },
      {
        userId: "user-auto",
        agentId: "agent-auto",
        sessionId: "session-auto",
        sessionKey: "agent:agent-auto:session:stable-auto",
      }
    );

    assert.equal(fetchMock.calls.length, 1);
    const call = fetchMock.calls[0];
    assert.equal(call.path, "/v1/memories/store");
    assert.deepEqual(Object.keys(call.body).sort(), ["actor", "items", "mode"]);
    assert.equal(call.body.mode, "auto-capture");
    assert.equal(call.body.actor.userId, "user-auto");
    assert.equal(call.body.actor.agentId, "agent-auto");
    assert.equal(call.body.actor.sessionId, "session-auto");
    assert.equal(call.body.actor.sessionKey, "agent:agent-auto:session:stable-auto");
    assert.equal(call.body.items.length, 2, "assistant message should be filtered when captureAssistant=false");
    assert.equal(call.body.items[0].role, "user");
    assert.equal(call.body.items[1].role, "user");
  });

  it("keeps auto-recall fail-open when backend generic recall fails", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/recall/generic",
        reply: () => ({
          status: 503,
          body: {
            error: {
              code: "BACKEND_UNAVAILABLE",
              message: "generic recall temporarily unavailable",
              retryable: true,
            },
          },
        }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoRecall: true,
        autoCapture: false,
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const beforeAgentStart = getLatestHandler(harness.eventHandlers, "before_agent_start");
    const output = await beforeAgentStart(
      { prompt: "What are my infrastructure operating invariants?" },
      {
        userId: "user-auto-recall",
        agentId: "agent-auto-recall",
        sessionId: "session-auto-recall",
        sessionKey: "agent:agent-auto-recall:session:stable-auto-recall",
      }
    );

    assert.equal(output, undefined, "auto-recall should fail open on backend recall failures");
    assert.equal(fetchMock.calls.length, 1);
    const recallCall = fetchMock.calls[0];
    assert.equal(recallCall.path, "/v1/recall/generic");
    assert.deepEqual(Object.keys(recallCall.body).sort(), ["actor", "limit", "query"]);
    assert.ok(
      harness.logs.some((entry) => entry.level === "warn" && entry.message.includes("auto-recall failed")),
      "failure should remain observable in logs"
    );
  });

  it("retries retryable remote store failures and reuses idempotency-key across retry attempts", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/memories/store",
        reply: (_call, attempt) => {
          if (attempt === 0) {
            return {
              status: 429,
              body: {
                error: {
                  code: "RATE_LIMITED",
                  message: "retry later",
                  retryable: true,
                },
              },
            };
          }
          return {
            results: [
              {
                id: "mem-retry-1",
                action: "ADD",
                text: "Persist with retry",
                category: "fact",
                importance: 0.7,
                scope: "agent:agent-retry",
              },
            ],
          };
        },
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoCapture: false,
        remoteBackend: {
          maxRetries: 1,
        },
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const store = harness.instantiateTool("memory_store", {
      userId: "user-retry",
      agentId: "agent-retry",
      sessionId: "session-retry",
      sessionKey: "agent:agent-retry:session:stable-retry",
    });
    const storeResult = await store.execute("call-store-retry", {
      text: "Persist with retry",
      category: "fact",
    });

    assert.equal(storeResult.details.action, "add");
    assert.equal(fetchMock.calls.length, 2);
    const first = fetchMock.calls[0];
    const second = fetchMock.calls[1];
    assert.ok(first.headers["idempotency-key"]);
    assert.equal(second.headers["idempotency-key"], first.headers["idempotency-key"]);
    assert.equal(first.headers["x-request-id"], second.headers["x-request-id"]);
  });

  it("does not retry non-retryable remote store failures", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/memories/store",
        reply: () => ({
          status: 400,
          body: {
            error: {
              code: "VALIDATION_ERROR",
              message: "invalid input",
              retryable: false,
            },
          },
        }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoCapture: false,
        remoteBackend: {
          maxRetries: 2,
        },
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const store = harness.instantiateTool("memory_store", {
      userId: "user-no-retry",
      agentId: "agent-no-retry",
      sessionId: "session-no-retry",
      sessionKey: "agent:agent-no-retry:session:stable-no-retry",
    });
    const storeResult = await store.execute("call-store-no-retry", {
      text: "Should fail once",
      category: "fact",
    });

    assert.equal(storeResult.details.error, "remote_backend_error");
    assert.equal(storeResult.details.code, "VALIDATION_ERROR");
    assert.equal(storeResult.details.status, 400);
    assert.equal(fetchMock.calls.length, 1);
  });

  it("surfaces backend write/update/delete failures in remote mode", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/memories/store",
        reply: () => ({
          status: 503,
          body: {
            error: {
              code: "STORE_BACKEND_DOWN",
              message: "store path unavailable",
              retryable: true,
            },
          },
        }),
      },
      {
        method: "POST",
        path: "/v1/memories/delete",
        reply: () => ({
          status: 502,
          body: {
            error: {
              code: "DELETE_BACKEND_DOWN",
              message: "delete path unavailable",
              retryable: true,
            },
          },
        }),
      },
      {
        method: "POST",
        path: "/v1/memories/update",
        reply: () => ({
          status: 500,
          body: {
            error: {
              code: "UPDATE_BACKEND_DOWN",
              message: "update path unavailable",
              retryable: false,
            },
          },
        }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "none",
        autoCapture: false,
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const toolCtx = {
      userId: "user-fail",
      agentId: "agent-fail",
      sessionId: "session-fail",
      sessionKey: "agent:agent-fail:session:stable-fail",
    };

    const store = harness.instantiateTool("memory_store", toolCtx);
    const storeResult = await store.execute("call-store-fail", {
      text: "Persist this",
      category: "fact",
    });
    assert.match(storeResult.content[0].text, /Memory storage failed: store path unavailable/);
    assert.equal(storeResult.details.error, "remote_backend_error");
    assert.equal(storeResult.details.code, "STORE_BACKEND_DOWN");
    assert.equal(storeResult.details.status, 503);

    const forget = harness.instantiateTool("memory_forget", toolCtx);
    const forgetResult = await forget.execute("call-delete-fail", {
      memoryId: "11111111-1111-1111-1111-111111111111",
    });
    assert.match(forgetResult.content[0].text, /Memory deletion failed: delete path unavailable/);
    assert.equal(forgetResult.details.error, "remote_backend_error");
    assert.equal(forgetResult.details.code, "DELETE_BACKEND_DOWN");
    assert.equal(forgetResult.details.status, 502);

    const update = harness.instantiateTool("memory_update", toolCtx);
    const updateResult = await update.execute("call-update-fail", {
      memoryId: "22222222-2222-2222-2222-222222222222",
      text: "Updated memory text",
    });
    assert.match(updateResult.content[0].text, /Memory update failed: update path unavailable/);
    assert.equal(updateResult.details.error, "remote_backend_error");
    assert.equal(updateResult.details.code, "UPDATE_BACKEND_DOWN");
    assert.equal(updateResult.details.status, 500);

    assert.equal(fetchMock.calls.length, 3);
  });

  it("uses backend reflection recall in before_prompt_build and preserves runtime context fields", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/recall/reflection",
        reply: () => ({
          rows: [
            {
              id: "refl-1",
              text: "Always perform post-change service and DNS checks.",
              kind: "invariant",
              strictKey: "post-checks",
              scope: "agent:agent-reflect",
              score: 0.92,
              metadata: { timestamp: Date.now() },
            },
          ],
        }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "memoryReflection",
        memoryReflection: {
          recall: {
            mode: "fixed",
            topK: 4,
            includeKinds: ["invariant", "derived"],
          },
          injectMode: "inheritance-only",
          messageCount: 12,
        },
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const beforePromptHandler = getLatestHandler(harness.eventHandlers, "before_prompt_build");
    const output = await beforePromptHandler(
      {
        prompt: "Before we continue, remind me of the operating invariants.",
      },
      {
        userId: "user-reflect",
        agentId: "agent-reflect",
        sessionId: "session-reflect",
        sessionKey: "agent:agent-reflect:session:stable-reflect",
      }
    );

    assert.ok(output && typeof output.prependContext === "string");
    assert.match(output.prependContext, /<inherited-rules>/);
    assert.match(output.prependContext, /Always perform post-change service and DNS checks\./);

    assert.equal(fetchMock.calls.length, 1);
    const recallCall = fetchMock.calls[0];
    assert.equal(recallCall.path, "/v1/recall/reflection");
    assert.deepEqual(Object.keys(recallCall.body).sort(), ["actor", "limit", "mode", "query"]);
    assert.equal(recallCall.body.mode, "invariant-only");
    assert.equal(recallCall.body.actor.userId, "user-reflect");
    assert.equal(recallCall.body.actor.agentId, "agent-reflect");
    assert.equal(recallCall.body.actor.sessionId, "session-reflect");
    assert.equal(recallCall.body.actor.sessionKey, "agent:agent-reflect:session:stable-reflect");
  });

  it("keeps reflection recall fail-open when runtime principal identity is unavailable", async () => {
    const root = makeTempRoot();
    const fetchMock = installFetchMock([]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "memoryReflection",
        memoryReflection: {
          injectMode: "inheritance-only",
          recall: {
            mode: "fixed",
            topK: 3,
            includeKinds: ["invariant"],
          },
        },
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const beforePromptHandler = getLatestHandler(harness.eventHandlers, "before_prompt_build");
    const output = await beforePromptHandler(
      { prompt: "Remind me of the key invariants." },
      {
        agentId: "agent-reflect-missing-user",
        sessionId: "session-reflect-missing-user",
        sessionKey: "agent:agent-reflect-missing-user:session:stable",
      }
    );

    assert.equal(output, undefined, "reflection recall should skip when runtime principal is unavailable");
    assert.equal(fetchMock.calls.length, 0);
    assert.ok(
      harness.logs.some(
        (entry) =>
          entry.level === "warn" &&
          entry.message.includes("reflection-recall skipped remote call (missing runtime principal")
      ),
      "skip reason should remain visible in logs"
    );
  });

  it("keeps reflection recall fail-open when backend reflection endpoint fails", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/recall/reflection",
        reply: () => ({
          status: 500,
          body: {
            error: {
              code: "REFLECTION_RECALL_DOWN",
              message: "reflection recall unavailable",
              retryable: true,
            },
          },
        }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "memoryReflection",
        memoryReflection: {
          injectMode: "inheritance-only",
          recall: {
            mode: "fixed",
            topK: 3,
            includeKinds: ["invariant"],
          },
        },
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const beforePromptHandler = getLatestHandler(harness.eventHandlers, "before_prompt_build");
    const output = await beforePromptHandler(
      { prompt: "Remind me of the key invariants." },
      {
        userId: "user-reflect-fail",
        agentId: "agent-reflect-fail",
        sessionId: "session-reflect-fail",
        sessionKey: "agent:agent-reflect-fail:session:stable-reflect-fail",
      }
    );

    assert.equal(output, undefined, "reflection recall failure should not block prompt flow");
    assert.equal(fetchMock.calls.length, 1);
    assert.ok(
      harness.logs.some(
        (entry) =>
          entry.level === "warn" &&
          entry.message.includes("reflection-recall injection failed")
      ),
      "failure should be observable in logs"
    );
  });

  it("enqueues command:new reflection jobs asynchronously and returns without waiting for backend completion", async () => {
    const root = makeTempRoot();

    const pending = deferred();
    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/reflection/jobs",
        reply: async () => await pending.promise,
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "memoryReflection",
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const commandNewHook = getLatestHandler(harness.commandHooks, "command:new");
    const event = {
      action: "new",
      agentId: "agent-cmd",
      sessionKey: "agent:agent-cmd:session:stable-cmd",
      sessionId: "event-session-id",
      userId: "user-cmd",
      messages: [
        { role: "user", content: [{ type: "text", text: "Keep reflection enqueue non-blocking." }] },
      ],
      context: {
        sessionEntry: { sessionId: "context-session-id" },
        commandSource: "unit-test",
      },
    };

    const hookPromise = commandNewHook(event);
    const race = await Promise.race([
      hookPromise.then(() => "resolved"),
      sleep(200).then(() => "timed_out"),
    ]);
    assert.equal(race, "resolved", "command:new hook should return before enqueue job completion");

    assert.equal(fetchMock.calls.length, 1);
    const call = fetchMock.calls[0];
    assert.equal(call.path, "/v1/reflection/jobs");
    assert.deepEqual(Object.keys(call.body).sort(), ["actor", "messages", "trigger"]);
    assert.equal(call.body.trigger, "new");
    assert.equal(call.body.actor.userId, "user-cmd");
    assert.equal(call.body.actor.agentId, "agent-cmd");
    assert.equal(call.body.actor.sessionId, "context-session-id");
    assert.equal(call.body.actor.sessionKey, "agent:agent-cmd:session:stable-cmd");
    assert.equal(call.body.messages.length, 1);

    pending.resolve(jsonResponse({ jobId: "job-new-1", status: "queued" }, 200));
    await sleep(0);
  });

  it("keeps reflection enqueue non-blocking and logs enqueue failures", async () => {
    const root = makeTempRoot();

    const pending = deferred();
    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/reflection/jobs",
        reply: async () => await pending.promise,
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "memoryReflection",
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const commandNewHook = getLatestHandler(harness.commandHooks, "command:new");
    const hookPromise = commandNewHook({
      action: "new",
      agentId: "agent-cmd-fail",
      sessionKey: "agent:agent-cmd-fail:session:stable-cmd-fail",
      userId: "user-cmd-fail",
      messages: [{ role: "user", content: [{ type: "text", text: "force async failure path" }] }],
      context: {
        sessionEntry: { sessionId: "context-session-fail" },
      },
    });
    const race = await Promise.race([
      hookPromise.then(() => "resolved"),
      sleep(200).then(() => "timed_out"),
    ]);
    assert.equal(race, "resolved", "enqueue failures should not block command hook completion");

    pending.reject(new Error("simulated enqueue transport failure"));
    await sleep(20);

    assert.equal(fetchMock.calls.length, 1);
    assert.ok(
      harness.logs.some(
        (entry) =>
          entry.level === "warn" &&
          entry.message.includes("command:new enqueue failed")
      ),
      "enqueue failures should remain visible to operators"
    );
  });

  it("enqueues command:reset reflection jobs with reset trigger and explicit runtime actor identity", async () => {
    const root = makeTempRoot();

    const fetchMock = installFetchMock([
      {
        method: "POST",
        path: "/v1/reflection/jobs",
        reply: () => ({ jobId: "job-reset-1", status: "queued" }),
      },
    ]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "memoryReflection",
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const commandResetHook = getLatestHandler(harness.commandHooks, "command:reset");
    await commandResetHook({
      action: "reset",
      agentId: "agent-reset",
      sessionKey: "agent:agent-reset:session:stable-reset",
      userId: "user-reset",
      messages: [
        { role: "user", content: [{ type: "text", text: "Reset the session and keep lessons." }] },
      ],
      context: {
        sessionEntry: { sessionId: "reset-session-id" },
      },
    });

    assert.equal(fetchMock.calls.length, 1);
    const call = fetchMock.calls[0];
    assert.equal(call.path, "/v1/reflection/jobs");
    assert.equal(call.body.trigger, "reset");
    assert.equal(call.body.actor.userId, "user-reset");
    assert.equal(call.body.actor.agentId, "agent-reset");
    assert.equal(call.body.actor.sessionId, "reset-session-id");
    assert.equal(call.body.actor.sessionKey, "agent:agent-reset:session:stable-reset");
    assert.equal("scope" in call.body, false);
    assert.equal("scopeFilter" in call.body, false);
  });

  it("fails reflection job enqueue closed when runtime principal identity is unavailable", async () => {
    const root = makeTempRoot();
    const fetchMock = installFetchMock([]);

    const harness = createPluginApiHarness({
      pluginConfig: makeRemoteConfig(root, {
        sessionStrategy: "memoryReflection",
      }),
      resolveRoot: root,
    });
    memoryLanceDBProPlugin.register(harness.api);

    const commandResetHook = getLatestHandler(harness.commandHooks, "command:reset");
    await commandResetHook({
      action: "reset",
      sessionKey: "agent:agent-reset:session:stable-reset",
      messages: [
        { role: "user", content: [{ type: "text", text: "Reset without runtime user principal." }] },
      ],
      context: {
        sessionEntry: { sessionId: "reset-session-id" },
      },
    });

    assert.equal(fetchMock.calls.length, 0);
    assert.ok(
      harness.logs.some(
        (entry) =>
          entry.level === "warn" &&
          entry.message.includes("enqueue blocked (missing runtime principal")
      ),
      "missing principal should block enqueue and remain observable in logs"
    );
  });
});
