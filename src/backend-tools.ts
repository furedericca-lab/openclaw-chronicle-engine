import { Type } from "@sinclair/typebox";
import { stringEnum } from "openclaw/plugin-sdk";
import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import {
  MissingRuntimePrincipalError,
  resolveBackendCallContext,
  type RuntimeContextDefaults,
} from "./backend-client/runtime-context.js";
import { MemoryBackendClientError } from "./backend-client/client.js";
import type {
  BackendCaptureItem,
  BackendRetrievalTrace,
  DistillMode,
  DistillPersistMode,
  MemoryBackendClient,
  MemoryCategory,
  ReflectionRecallMode,
} from "./backend-client/types.js";

const MEMORY_CATEGORIES = [
  "preference",
  "fact",
  "decision",
  "entity",
  "reflection",
  "other",
] as const;

const MESSAGE_ROLES = ["user", "assistant", "system"] as const;
const DISTILL_MODES = ["session-lessons", "governance-candidates"] as const;
const DISTILL_SOURCE_KINDS = ["inline-messages", "session-transcript"] as const;
const DISTILL_PERSIST_MODES = ["artifacts-only", "persist-memory-rows"] as const;
const DEBUG_RECALL_CHANNELS = ["generic", "reflection"] as const;
const REFLECTION_DEBUG_MODES = ["invariant-only", "invariant+derived"] as const;

export interface BackendToolRegistrationOptions {
  enableManagementTools?: boolean;
}

export interface BackendToolRegistrationContext {
  backendClient: MemoryBackendClient;
  runtimeDefaults: RuntimeContextDefaults;
}

export function registerRemoteMemoryTools(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext,
  options: BackendToolRegistrationOptions = {}
) {
  registerMemoryRecallTool(api, context);
  registerMemoryStoreTool(api, context);
  registerMemoryForgetTool(api, context);
  registerMemoryUpdateTool(api, context);

  if (options.enableManagementTools) {
    registerMemoryReflectionStatusTool(api, context);
    registerMemoryDistillEnqueueTool(api, context);
    registerMemoryDistillStatusTool(api, context);
    registerMemoryRecallDebugTool(api, context);
    registerMemoryStatsTool(api, context);
    registerMemoryListTool(api, context);
  }
}

function registerMemoryRecallTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_recall",
      label: "Memory Recall",
      description: "Search caller-visible memories via the remote backend (scope authority is backend-owned).",
      parameters: Type.Object({
        query: Type.String({ description: "Search query for finding relevant memories" }),
        limit: Type.Optional(Type.Number({ description: "Max results to return (default: 5, max: 20)" })),
        category: Type.Optional(stringEnum(MEMORY_CATEGORIES)),
        debug: Type.Optional(Type.Boolean({ description: "Include lightweight debug details." })),
      }),
      async execute(_toolCallId, params) {
        const { query, limit = 5, category, debug = false } = params as {
          query: string;
          limit?: number;
          category?: string;
          debug?: boolean;
        };
        try {
          const resolved = resolveToolCallContext(toolCtx, context.runtimeDefaults);
          if (!resolved.hasPrincipalIdentity) {
            const warning = formatMissingPrincipalWarning(resolved.missingPrincipalFields);
            api.logger.warn?.(`openclaw-chronicle-engine: memory_recall skipped (${warning})`);
            return {
              content: [{ type: "text", text: `No relevant memories found. Remote recall skipped: ${warning}.` }],
              details: {
                count: 0,
                query,
                skipped: true,
                error: "missing_runtime_principal",
                missingPrincipalFields: resolved.missingPrincipalFields,
                category: category || undefined,
              },
            };
          }
          const rows = await context.backendClient.recallGeneric(resolved.context, {
            query,
            limit: clampInt(limit, 1, 20),
          });
          const filtered = category
            ? rows.filter((row) => row.category === category)
            : rows;

          if (filtered.length === 0) {
            return {
              content: [{ type: "text", text: "No relevant memories found." }],
              details: {
                count: 0,
                query,
                category: category || undefined,
              },
            };
          }

          const text = filtered
            .map(
              (row, i) =>
                `${i + 1}. [${row.id}] [${row.category}] ${row.text} (${(row.score * 100).toFixed(0)}%)`
            )
            .join("\n");

          return {
            content: [
              {
                type: "text",
                text: `Found ${filtered.length} memories:\n\n${text}`,
              },
            ],
            details: {
              count: filtered.length,
              query,
              memories: filtered,
              category: category || undefined,
              debug: debug ? { note: "Backend-scoped recall path active." } : undefined,
            },
          };
        } catch (error) {
          return backendToolError("Memory recall failed", error);
        }
      },
    }),
    { name: "memory_recall" }
  );
}

function registerMemoryStoreTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_store",
      label: "Memory Store",
      description: "Store an explicit memory via remote backend tool-store mode (backend chooses scope).",
      parameters: Type.Object({
        text: Type.String({ description: "Information to remember" }),
        importance: Type.Optional(Type.Number({ description: "Importance score 0-1 (optional)" })),
        category: Type.Optional(stringEnum(MEMORY_CATEGORIES)),
      }),
      async execute(_toolCallId, params) {
        const { text, importance, category } = params as {
          text: string;
          importance?: number;
          category?: MemoryCategory;
        };
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const results = await context.backendClient.storeToolMemory(ctx, {
            text,
            importance: Number.isFinite(importance) ? clamp01(Number(importance), 0.7) : undefined,
            category,
          });
          if (results.length === 0) {
            return {
              content: [{ type: "text", text: "No memory mutations were returned by backend." }],
              details: {
                action: "noop",
              },
            };
          }
          const first = results[0];
          return {
            content: [
              {
                type: "text",
                text: `Stored via backend (${first.action.toLowerCase()}): "${first.text.slice(0, 100)}${first.text.length > 100 ? "..." : ""}"`,
              },
            ],
            details: {
              action: first.action.toLowerCase(),
              results,
            },
          };
        } catch (error) {
          return backendToolError("Memory storage failed", error);
        }
      },
    }),
    { name: "memory_store" }
  );
}

function registerMemoryForgetTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_forget",
      label: "Memory Forget",
      description: "Delete caller-visible memories by id or query via remote backend.",
      parameters: Type.Object({
        query: Type.Optional(Type.String({ description: "Search query to find memory to delete" })),
        memoryId: Type.Optional(Type.String({ description: "Specific memory ID to delete" })),
      }),
      async execute(_toolCallId, params) {
        const { query, memoryId } = params as {
          query?: string;
          memoryId?: string;
        };
        if ((query ? 1 : 0) + (memoryId ? 1 : 0) !== 1) {
          return {
            content: [{ type: "text", text: "Provide exactly one of query or memoryId." }],
            details: { error: "missing_param" },
          };
        }
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const result = await context.backendClient.deleteMemory(ctx, {
            query: query || undefined,
            memoryId: memoryId || undefined,
          });
          if (result.deleted > 0) {
            return {
              content: [{ type: "text", text: `Deleted ${result.deleted} memory item(s).` }],
              details: {
                action: "deleted",
                deleted: result.deleted,
              },
            };
          }
          return {
            content: [{ type: "text", text: "No matching memory found or access denied." }],
            details: {
              error: "not_found",
              deleted: 0,
            },
          };
        } catch (error) {
          return backendToolError("Memory deletion failed", error);
        }
      },
    }),
    { name: "memory_forget" }
  );
}

function registerMemoryUpdateTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_update",
      label: "Memory Update",
      description: "Update an existing caller-visible memory via remote backend.",
      parameters: Type.Object({
        memoryId: Type.String({
          description: "Memory ID (full UUID or query-like alias for best-effort resolution)",
        }),
        text: Type.Optional(Type.String({ description: "New text content" })),
        importance: Type.Optional(Type.Number({ description: "New importance score 0-1" })),
        category: Type.Optional(stringEnum(MEMORY_CATEGORIES)),
      }),
      async execute(_toolCallId, params) {
        const { memoryId, text, importance, category } = params as {
          memoryId: string;
          text?: string;
          importance?: number;
          category?: MemoryCategory;
        };
        if (!text && importance === undefined && !category) {
          return {
            content: [{ type: "text", text: "Nothing to update. Provide text, importance, or category." }],
            details: { error: "no_updates" },
          };
        }
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const resolvedId = await resolveMemoryId(ctx, context.backendClient, memoryId);
          if (!resolvedId) {
            return {
              content: [{ type: "text", text: `No memory found matching "${memoryId}".` }],
              details: { error: "not_found", query: memoryId },
            };
          }
          if (Array.isArray(resolvedId)) {
            const list = resolvedId.map((row) => `- [${row.id.slice(0, 8)}] ${row.text.slice(0, 60)}`).join("\n");
            return {
              content: [{ type: "text", text: `Multiple matches. Specify memoryId:\n${list}` }],
              details: { action: "candidates", candidates: resolvedId },
            };
          }

          const updated = await context.backendClient.updateMemory(ctx, {
            memoryId: resolvedId,
            patch: {
              text: text || undefined,
              category,
              importance: Number.isFinite(importance) ? clamp01(Number(importance), 0.7) : undefined,
            },
          });

          return {
            content: [{ type: "text", text: `Updated memory ${updated.id.slice(0, 8)}...` }],
            details: {
              action: "updated",
              id: updated.id,
              category: updated.category,
              importance: updated.importance,
              scope: updated.scope,
            },
          };
        } catch (error) {
          return backendToolError("Memory update failed", error);
        }
      },
    }),
    { name: "memory_update" }
  );
}

function registerMemoryStatsTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_stats",
      label: "Memory Statistics",
      description: "Get caller-scoped memory statistics from the remote backend.",
      parameters: Type.Object({}),
      async execute() {
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const stats = await context.backendClient.stats(ctx);
          const lines = [
            "Memory Statistics (remote backend):",
            `• Total memories: ${stats.memoryCount}`,
            `• Reflection memories: ${stats.reflectionCount}`,
            "",
            "Memories by category:",
            ...Object.entries(stats.categories || {}).map(([name, count]) => `  • ${name}: ${count}`),
          ];
          return {
            content: [{ type: "text", text: lines.join("\n") }],
            details: {
              stats,
            },
          };
        } catch (error) {
          return backendToolError("Failed to get memory stats", error);
        }
      },
    }),
    { name: "memory_stats" }
  );
}

function registerMemoryListTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_list",
      label: "Memory List",
      description: "List recent caller-visible memories via remote backend (backend-owned scope semantics).",
      parameters: Type.Object({
        limit: Type.Optional(Type.Number({ description: "Max memories to list (default: 10, max: 50)" })),
        category: Type.Optional(stringEnum(MEMORY_CATEGORIES)),
        offset: Type.Optional(Type.Number({ description: "Rows to skip (default: 0)" })),
      }),
      async execute(_toolCallId, params) {
        const { limit = 10, category, offset = 0 } = params as {
          limit?: number;
          category?: MemoryCategory;
          offset?: number;
        };
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const response = await context.backendClient.listMemories(ctx, {
            limit: clampInt(limit, 1, 50),
            offset: clampInt(offset, 0, 1_000_000),
            category,
          });
          if (response.rows.length === 0) {
            return {
              content: [{ type: "text", text: "No memories found." }],
              details: {
                count: 0,
                nextOffset: response.nextOffset,
              },
            };
          }

          const text = response.rows
            .map((row, i) => `${Number(offset) + i + 1}. [${row.id}] [${row.category}] ${row.text.slice(0, 100)}`)
            .join("\n");

          return {
            content: [{ type: "text", text: `Recent memories (showing ${response.rows.length}):\n\n${text}` }],
            details: {
              count: response.rows.length,
              rows: response.rows,
              nextOffset: response.nextOffset,
            },
          };
        } catch (error) {
          return backendToolError("Failed to list memories", error);
        }
      },
    }),
    { name: "memory_list" }
  );
}

function registerMemoryDistillEnqueueTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_distill_enqueue",
      label: "Memory Distill Enqueue",
      description: "Enqueue a backend-native distill job for inline messages or a caller-scoped session transcript.",
      parameters: Type.Object({
        mode: stringEnum(DISTILL_MODES),
        sourceKind: stringEnum(DISTILL_SOURCE_KINDS),
        persistMode: stringEnum(DISTILL_PERSIST_MODES),
        messages: Type.Optional(
          Type.Array(
            Type.Object({
              role: stringEnum(MESSAGE_ROLES),
              text: Type.String({ description: "Message text to distill." }),
            })
          )
        ),
        sessionKey: Type.Optional(Type.String({ description: "Session transcript key when sourceKind=session-transcript." })),
        sessionId: Type.Optional(Type.String({ description: "Optional runtime session id when sourceKind=session-transcript." })),
        maxMessages: Type.Optional(Type.Number({ description: "Optional cap on transcript messages included in distill." })),
        chunkChars: Type.Optional(Type.Number({ description: "Optional text chunk size in characters." })),
        chunkOverlapMessages: Type.Optional(Type.Number({ description: "Optional overlap in messages between chunks." })),
        maxArtifacts: Type.Optional(Type.Number({ description: "Optional cap on emitted artifacts." })),
      }),
      async execute(_toolCallId, params) {
        const {
          mode,
          sourceKind,
          persistMode,
          messages,
          sessionKey,
          sessionId,
          maxMessages,
          chunkChars,
          chunkOverlapMessages,
          maxArtifacts,
        } = params as {
          mode: DistillMode;
          sourceKind: "inline-messages" | "session-transcript";
          persistMode: DistillPersistMode;
          messages?: BackendCaptureItem[];
          sessionKey?: string;
          sessionId?: string;
          maxMessages?: number;
          chunkChars?: number;
          chunkOverlapMessages?: number;
          maxArtifacts?: number;
        };

        const source = buildDistillSource({
          sourceKind,
          messages,
          sessionKey,
          sessionId,
        });
        if ("error" in source) {
          return {
            content: [{ type: "text", text: source.message }],
            details: { error: source.error },
          };
        }

        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const response = await context.backendClient.enqueueDistillJob(ctx, {
            mode,
            source,
            options: {
              persistMode,
              maxMessages: normalizeOptionalInt(maxMessages, 1, 10_000),
              chunkChars: normalizeOptionalInt(chunkChars, 128, 1_000_000),
              chunkOverlapMessages: normalizeOptionalInt(chunkOverlapMessages, 0, 10_000),
              maxArtifacts: normalizeOptionalInt(maxArtifacts, 1, 10_000),
            },
          });
          const sourceSummary = describeDistillSource(source);
          return {
            content: [
              {
                type: "text",
                text: `Distill job enqueued: ${response.jobId} (${response.status})\nSource: ${sourceSummary}`,
              },
            ],
            details: {
              jobId: response.jobId,
              status: response.status,
              mode,
              persistMode,
              sourceKind: source.kind,
              sourceSummary,
            },
          };
        } catch (error) {
          return backendToolError("Distill enqueue failed", error);
        }
      },
    }),
    { name: "memory_distill_enqueue" }
  );
}

function registerMemoryReflectionStatusTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_reflection_status",
      label: "Memory Reflection Status",
      description: "Inspect a caller-scoped backend reflection job by id.",
      parameters: Type.Object({
        jobId: Type.String({ description: "Reflection job id returned by the reflection enqueue path." }),
      }),
      async execute(_toolCallId, params) {
        const { jobId } = params as { jobId: string };
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const status = await context.backendClient.getReflectionJobStatus(ctx, { jobId });
          const lines = [
            `Reflection job ${status.jobId}: ${status.status}`,
          ];
          if (status.persisted !== undefined) {
            lines.push(`Persisted: ${status.persisted ? "yes" : "no"}`);
          }
          if (status.memoryCount !== undefined) {
            lines.push(`Memory rows: ${status.memoryCount}`);
          }
          if (status.error) {
            lines.push(`Error: ${status.error.code} (${status.error.retryable ? "retryable" : "non-retryable"})`);
          }
          return {
            content: [{ type: "text", text: lines.join("\n") }],
            details: status,
          };
        } catch (error) {
          return backendToolError("Reflection status lookup failed", error);
        }
      },
    }),
    { name: "memory_reflection_status" }
  );
}

function registerMemoryDistillStatusTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_distill_status",
      label: "Memory Distill Status",
      description: "Inspect a backend-native distill job by id.",
      parameters: Type.Object({
        jobId: Type.String({ description: "Distill job id returned by memory_distill_enqueue." }),
      }),
      async execute(_toolCallId, params) {
        const { jobId } = params as { jobId: string };
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const status = await context.backendClient.getDistillJobStatus(ctx, { jobId });
          const lines = [
            `Distill job ${status.jobId}: ${status.status}`,
            `Mode: ${status.mode}`,
            `Source: ${status.sourceKind}`,
            `Created: ${status.createdAt}`,
            `Updated: ${status.updatedAt}`,
          ];
          if (status.result) {
            lines.push(
              `Result: artifacts=${status.result.artifactCount}, persistedMemoryRows=${status.result.persistedMemoryCount}`
            );
            if (status.result.warnings.length > 0) {
              lines.push(`Warnings: ${status.result.warnings.join(" | ")}`);
            }
          }
          if (status.error) {
            lines.push(`Error: ${status.error.code} (${status.error.retryable ? "retryable" : "non-retryable"})`);
          }
          return {
            content: [{ type: "text", text: lines.join("\n") }],
            details: status,
          };
        } catch (error) {
          return backendToolError("Distill status lookup failed", error);
        }
      },
    }),
    { name: "memory_distill_status" }
  );
}

function registerMemoryRecallDebugTool(
  api: OpenClawPluginApi,
  context: BackendToolRegistrationContext
) {
  api.registerTool(
    (toolCtx) => ({
      name: "memory_recall_debug",
      label: "Memory Recall Debug",
      description: "Inspect backend retrieval trace data on explicit debug routes.",
      parameters: Type.Object({
        channel: stringEnum(DEBUG_RECALL_CHANNELS),
        query: Type.String({ description: "Debug recall query." }),
        limit: Type.Optional(Type.Number({ description: "Max rows to inspect (default: 5, max: 20)." })),
        reflectionMode: Type.Optional(
          stringEnum(REFLECTION_DEBUG_MODES)
        ),
      }),
      async execute(_toolCallId, params) {
        const { channel, query, limit = 5, reflectionMode } = params as {
          channel: "generic" | "reflection";
          query: string;
          limit?: number;
          reflectionMode?: ReflectionRecallMode;
        };
        if (channel === "generic" && reflectionMode) {
          return {
            content: [{ type: "text", text: "reflectionMode is only valid when channel=reflection." }],
            details: { error: "invalid_param" },
          };
        }
        try {
          const ctx = buildToolCallContext(toolCtx, context.runtimeDefaults);
          const normalizedLimit = clampInt(limit, 1, 20);
          const response = channel === "reflection"
            ? await context.backendClient.recallReflectionDebug(ctx, {
              query,
              mode: reflectionMode || "invariant+derived",
              limit: normalizedLimit,
            })
            : await context.backendClient.recallGenericDebug(ctx, {
              query,
              limit: normalizedLimit,
            });

          return {
            content: [
              {
                type: "text",
                text: formatDebugRecallSummary(channel, response.rows, response.trace),
              },
            ],
            details: {
              channel,
              count: response.rows.length,
              rows: response.rows,
              trace: response.trace,
            },
          };
        } catch (error) {
          return backendToolError("Recall debug lookup failed", error);
        }
      },
    }),
    { name: "memory_recall_debug" }
  );
}

function buildToolCallContext(toolCtx: unknown, defaults: RuntimeContextDefaults) {
  const resolved = resolveToolCallContext(toolCtx, defaults);
  if (!resolved.hasPrincipalIdentity) {
    throw new MissingRuntimePrincipalError(resolved.missingPrincipalFields);
  }
  return resolved.context;
}

function resolveToolCallContext(toolCtx: unknown, defaults: RuntimeContextDefaults) {
  const source = (toolCtx && typeof toolCtx === "object") ? toolCtx as Record<string, unknown> : {};
  const nested = source.context && typeof source.context === "object"
    ? source.context as Record<string, unknown>
    : {};
  return resolveBackendCallContext({ ...nested, ...source }, defaults);
}

function backendToolError(prefix: string, error: unknown) {
  if (error instanceof MissingRuntimePrincipalError) {
    const warning = formatMissingPrincipalWarning(error.missingPrincipalFields);
    return {
      content: [{ type: "text", text: `${prefix}: blocked because ${warning}.` }],
      details: {
        error: "missing_runtime_principal",
        missingPrincipalFields: error.missingPrincipalFields,
        message: warning,
      },
    };
  }
  if (error instanceof MemoryBackendClientError) {
    return {
      content: [{ type: "text", text: `${prefix}: ${error.message}` }],
      details: {
        error: "remote_backend_error",
        code: error.code,
        retryable: error.retryable,
        status: error.status,
        message: error.message,
      },
    };
  }
  return {
    content: [{ type: "text", text: `${prefix}: ${error instanceof Error ? error.message : String(error)}` }],
    details: {
      error: "remote_backend_error",
      message: String(error),
    },
  };
}

function formatMissingPrincipalWarning(fields: Array<"userId" | "agentId">): string {
  const joined = fields.join(", ");
  return `runtime principal identity is unavailable (missing ${joined})`;
}

async function resolveMemoryId(
  ctx: ReturnType<typeof buildToolCallContext>,
  backend: MemoryBackendClient,
  memoryId: string
): Promise<string | Array<{ id: string; text: string }> | undefined> {
  const trimmed = memoryId.trim();
  if (!trimmed) return undefined;
  const uuidLike = /^[0-9a-f]{8}(-[0-9a-f]{4}){0,4}/i.test(trimmed);
  if (uuidLike) return trimmed;

  const rows = await backend.recallGeneric(ctx, { query: trimmed, limit: 3 });
  if (rows.length === 0) return undefined;
  if (rows.length === 1 || rows[0].score >= 0.85) return rows[0].id;
  return rows.map((row) => ({ id: row.id, text: row.text }));
}

function clampInt(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.max(min, Math.min(max, Math.floor(value)));
}

function clamp01(value: number, fallback: number): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.max(0, Math.min(1, value));
}

function normalizeOptionalInt(value: unknown, min: number, max: number): number | undefined {
  if (!Number.isFinite(value)) return undefined;
  return clampInt(Number(value), min, max);
}

function buildDistillSource(params: {
  sourceKind: "inline-messages" | "session-transcript";
  messages?: BackendCaptureItem[];
  sessionKey?: string;
  sessionId?: string;
}):
  | { kind: "inline-messages"; messages: BackendCaptureItem[] }
  | { kind: "session-transcript"; sessionKey: string; sessionId?: string }
  | { error: string; message: string } {
  if (params.sourceKind === "inline-messages") {
    const messages = Array.isArray(params.messages)
      ? params.messages
        .map((item) => ({
          role: item?.role === "assistant" || item?.role === "system" ? item.role : "user",
          text: typeof item?.text === "string" ? item.text.trim() : "",
        }))
        .filter((item) => item.text.length > 0)
      : [];
    if (messages.length === 0) {
      return {
        error: "missing_messages",
        message: "messages is required when sourceKind=inline-messages.",
      };
    }
    return {
      kind: "inline-messages",
      messages,
    };
  }

  const normalizedSessionKey = typeof params.sessionKey === "string" ? params.sessionKey.trim() : "";
  if (!normalizedSessionKey) {
    return {
      error: "missing_session_key",
      message: "sessionKey is required when sourceKind=session-transcript.",
    };
  }
  const normalizedSessionId = typeof params.sessionId === "string" ? params.sessionId.trim() : "";
  return {
    kind: "session-transcript",
    sessionKey: normalizedSessionKey,
    sessionId: normalizedSessionId || undefined,
  };
}

function describeDistillSource(source:
  | { kind: "inline-messages"; messages: BackendCaptureItem[] }
  | { kind: "session-transcript"; sessionKey: string; sessionId?: string }
): string {
  if (source.kind === "inline-messages") {
    return `inline-messages (${source.messages.length} message(s))`;
  }
  return `session-transcript (${source.sessionKey}${source.sessionId ? `, sessionId=${source.sessionId}` : ""})`;
}

function formatDebugRecallSummary(
  channel: "generic" | "reflection",
  rows: Array<{ id: string; text: string; score: number }>,
  trace: BackendRetrievalTrace
): string {
  const lines = [
    `Debug recall trace (${channel}): ${rows.length} row(s)`,
    `Trace kind: ${trace.kind}`,
  ];
  if (trace.mode) {
    lines.push(`Trace mode: ${trace.mode}`);
  }

  const stageSummary = trace.stages
    .slice(0, 8)
    .map((stage) => {
      const reason = stage.reason ? ` (${clipForToolOutput(stage.reason, 120)})` : "";
      return `- ${stage.name}: ${stage.status}${reason}`;
    });
  if (stageSummary.length > 0) {
    lines.push("Stages:");
    lines.push(...stageSummary);
  }

  const rowSummary = rows
    .slice(0, 5)
    .map((row, index) => `${index + 1}. [${row.id}] ${(Number(row.score) * 100).toFixed(0)}% ${clipForToolOutput(row.text, 120)}`);
  if (rowSummary.length > 0) {
    lines.push("Rows:");
    lines.push(...rowSummary);
  }

  return lines.join("\n");
}

function clipForToolOutput(text: string, maxLen: number): string {
  const normalized = String(text || "").replace(/\s+/g, " ").trim();
  if (normalized.length <= maxLen) return normalized;
  return `${normalized.slice(0, Math.max(0, maxLen - 3))}...`;
}
