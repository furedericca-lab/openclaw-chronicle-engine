import { afterEach, describe, it } from "node:test";
import assert from "node:assert/strict";
import jitiFactory from "jiti";

const jiti = jitiFactory(import.meta.url, {
  interopDefault: true,
});

const { createMemoryBackendClient, MemoryBackendClientError } = jiti("../src/backend-client/client.ts");

const cleanupStack = [];
afterEach(() => {
  while (cleanupStack.length > 0) {
    const fn = cleanupStack.pop();
    try {
      fn();
    } catch {
      // best-effort cleanup
    }
  }
});

function withCleanup(fn) {
  cleanupStack.push(fn);
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

function installFetchMock(reply) {
  const calls = [];
  const originalFetch = globalThis.fetch;

  globalThis.fetch = async (input, init = {}) => {
    const url = typeof input === "string" ? input : input.url;
    const method = String(init.method || "GET").toUpperCase();
    const headers = normalizeHeaders(init.headers);
    const body = typeof init.body === "string" && init.body.trim()
      ? JSON.parse(init.body)
      : undefined;
    const call = { url, method, headers, body };
    calls.push(call);
    const result = await reply(call, calls.length - 1);
    if (result instanceof Response) return result;
    if (result && typeof result === "object" && Object.prototype.hasOwnProperty.call(result, "status")) {
      return jsonResponse(result.body || {}, Number(result.status || 200));
    }
    return jsonResponse(result || {}, 200);
  };

  withCleanup(() => {
    globalThis.fetch = originalFetch;
  });

  return { calls };
}

function createClient(overrides = {}) {
  return createMemoryBackendClient({
    baseUrl: "http://backend.test",
    bearerToken: "token-test",
    timeoutMs: 2000,
    maxRetries: 0,
    retryBaseDelayMs: 25,
    ...overrides,
  });
}

function baseContext(requestId = "req-default") {
  return {
    requestId,
    identity: {
      userId: "user-test",
      agentId: "agent-test",
    },
    actor: {
      userId: "user-test",
      agentId: "agent-test",
      sessionId: "session-runtime-test",
      sessionKey: "agent:agent-test:session:stable-test",
    },
  };
}

describe("memory backend client retry/idempotency", () => {
  it("retries retryable 429 write failures and keeps idempotency key stable across attempts", async () => {
    const fetchMock = installFetchMock((_call, attempt) => {
      if (attempt === 0) {
        return {
          status: 429,
          body: {
            error: {
              code: "RATE_LIMITED",
              message: "too many requests",
              retryable: true,
            },
          },
        };
      }
      return {
        results: [
          {
            id: "mem-1",
            action: "ADD",
            text: "retry me",
            category: "fact",
            importance: 0.7,
            scope: "agent:agent-test",
          },
        ],
      };
    });

    const client = createClient({ maxRetries: 1 });
    const results = await client.storeToolMemory(baseContext("req-store-429"), {
      text: "retry me",
      category: "fact",
    });

    assert.equal(results.length, 1);
    assert.equal(fetchMock.calls.length, 2);
    assert.equal(fetchMock.calls[0].method, "POST");
    assert.equal(fetchMock.calls[1].method, "POST");

    const firstKey = fetchMock.calls[0].headers["idempotency-key"];
    assert.ok(firstKey);
    assert.equal(fetchMock.calls[1].headers["idempotency-key"], firstKey);
    assert.equal(fetchMock.calls[0].headers["x-request-id"], "req-store-429");
    assert.equal(fetchMock.calls[1].headers["x-request-id"], "req-store-429");
  });

  it("retries transport failures and preserves write idempotency key per request", async () => {
    const fetchMock = installFetchMock((_call, attempt) => {
      if (attempt === 0) {
        throw new Error("socket closed");
      }
      return { deleted: 1 };
    });

    const client = createClient({ maxRetries: 1 });
    const deleted = await client.deleteMemory(baseContext("req-delete-transport"), {
      memoryId: "mem-delete-1",
    });

    assert.equal(deleted.deleted, 1);
    assert.equal(fetchMock.calls.length, 2);
    const firstKey = fetchMock.calls[0].headers["idempotency-key"];
    assert.ok(firstKey);
    assert.equal(fetchMock.calls[1].headers["idempotency-key"], firstKey);
    assert.equal(fetchMock.calls[0].headers["x-request-id"], "req-delete-transport");
    assert.equal(fetchMock.calls[1].headers["x-request-id"], "req-delete-transport");
  });

  it("does not retry non-retryable backend failures", async () => {
    const fetchMock = installFetchMock(() => ({
      status: 400,
      body: {
        error: {
          code: "VALIDATION_ERROR",
          message: "invalid patch",
          retryable: false,
        },
      },
    }));

    const client = createClient({ maxRetries: 3 });

    await assert.rejects(
      () =>
        client.updateMemory(baseContext("req-update-400"), {
          memoryId: "mem-update-1",
          patch: { text: "patched" },
        }),
      (error) => {
        assert.ok(error instanceof MemoryBackendClientError);
        assert.equal(error.code, "VALIDATION_ERROR");
        assert.equal(error.status, 400);
        assert.equal(error.retryable, false);
        return true;
      }
    );

    assert.equal(fetchMock.calls.length, 1);
  });

  it("retries 503 for read requests without idempotency key headers", async () => {
    const fetchMock = installFetchMock((_call, attempt) => {
      if (attempt === 0) {
        return {
          status: 503,
          body: {
            error: {
              code: "BACKEND_UNAVAILABLE",
              message: "temporary outage",
              retryable: true,
            },
          },
        };
      }
      return {
        rows: [
          {
            id: "mem-r1",
            text: "Recovered row",
            category: "decision",
            scope: "agent:agent-test",
            score: 0.9,
            metadata: { createdAt: Date.now(), updatedAt: Date.now() },
          },
        ],
      };
    });

    const client = createClient({ maxRetries: 1 });
    const rows = await client.recallGeneric(baseContext("req-recall-503"), {
      query: "recover me",
      limit: 3,
    });

    assert.equal(rows.length, 1);
    assert.equal(fetchMock.calls.length, 2);
    assert.equal(fetchMock.calls[0].headers["idempotency-key"], undefined);
    assert.equal(fetchMock.calls[1].headers["idempotency-key"], undefined);
  });

  it("uses a fresh idempotency key for each top-level write request", async () => {
    const fetchMock = installFetchMock(() => ({
      results: [
        {
          id: "mem-2",
          action: "ADD",
          text: "stored",
          category: "fact",
          importance: 0.7,
          scope: "agent:agent-test",
        },
      ],
    }));

    const client = createClient();
    await client.storeToolMemory(baseContext("req-store-a"), { text: "stored", category: "fact" });
    await client.storeToolMemory(baseContext("req-store-b"), { text: "stored", category: "fact" });

    assert.equal(fetchMock.calls.length, 2);
    const keyA = fetchMock.calls[0].headers["idempotency-key"];
    const keyB = fetchMock.calls[1].headers["idempotency-key"];
    assert.ok(keyA);
    assert.ok(keyB);
    assert.notEqual(keyA, keyB);
  });

  it("retries distill enqueue failures and keeps the idempotency key stable", async () => {
    const fetchMock = installFetchMock((_call, attempt) => {
      if (attempt === 0) {
        return {
          status: 503,
          body: {
            error: {
              code: "BACKEND_UNAVAILABLE",
              message: "distill queue unavailable",
              retryable: true,
            },
          },
        };
      }
      return jsonResponse({
        jobId: "distill-job-1",
        status: "queued",
      });
    });

    const client = createClient({ maxRetries: 1 });
    const response = await client.enqueueDistillJob(baseContext("req-distill-503"), {
      mode: "session-lessons",
      source: {
        kind: "inline-messages",
        messages: [{ role: "user", text: "Capture durable rollout lessons." }],
      },
      options: {
        persistMode: "artifacts-only",
        maxArtifacts: 4,
      },
    });

    assert.equal(response.jobId, "distill-job-1");
    assert.equal(fetchMock.calls.length, 2);
    assert.equal(fetchMock.calls[0].method, "POST");
    assert.equal(new URL(fetchMock.calls[0].url).pathname, "/v1/distill/jobs");
    const firstKey = fetchMock.calls[0].headers["idempotency-key"];
    assert.ok(firstKey);
    assert.equal(fetchMock.calls[1].headers["idempotency-key"], firstKey);
    assert.equal(fetchMock.calls[0].headers["x-request-id"], "req-distill-503");
    assert.equal(fetchMock.calls[1].headers["x-request-id"], "req-distill-503");
  });

  it("calls debug recall routes without idempotency headers and returns trace payloads", async () => {
    const fetchMock = installFetchMock(() => ({
      rows: [
        {
          id: "mem-debug-1",
          text: "Keep rollback evidence compact and explicit.",
          category: "decision",
          scope: "agent:agent-test",
          score: 0.91,
          metadata: { createdAt: Date.now(), updatedAt: Date.now() },
        },
      ],
      trace: {
        kind: "generic",
        query: {
          preview: "rollback evidence",
          rawLen: 17,
          lexicalPreview: "rollback evidence",
          lexicalLen: 17,
        },
        stages: [
          { name: "seed.merge", status: "ok" },
          { name: "rank.finalize", status: "ok" },
        ],
        finalRowIds: ["mem-debug-1"],
      },
    }));

    const client = createClient();
    const response = await client.recallGenericDebug(baseContext("req-debug-recall"), {
      query: "rollback evidence",
      limit: 3,
    });

    assert.equal(response.rows.length, 1);
    assert.equal(response.trace.kind, "generic");
    assert.equal(fetchMock.calls.length, 1);
    assert.equal(new URL(fetchMock.calls[0].url).pathname, "/v1/debug/recall/generic");
    assert.equal(fetchMock.calls[0].headers["idempotency-key"], undefined);
    assert.deepEqual(Object.keys(fetchMock.calls[0].body).sort(), ["actor", "limit", "query"]);
  });
});
