import { Type } from "@sinclair/typebox";
import { stringEnum } from "openclaw/plugin-sdk";
import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import {
  MissingRuntimePrincipalError,
  resolveBackendCallContext,
  type RuntimeContextDefaults,
} from "./backend-client/runtime-context.js";
import { MemoryBackendClientError } from "./backend-client/client.js";
import type { MemoryBackendClient, MemoryCategory } from "./backend-client/types.js";
import {
  registerSelfImprovementExtractSkillTool,
  registerSelfImprovementLogTool,
  registerSelfImprovementReviewTool,
  type SelfImprovementToolContext,
} from "./self-improvement-tools.js";

const MEMORY_CATEGORIES = [
  "preference",
  "fact",
  "decision",
  "entity",
  "reflection",
  "other",
] as const;

export interface BackendToolRegistrationOptions {
  enableManagementTools?: boolean;
  enableSelfImprovementTools?: boolean;
  defaultWorkspaceDir?: string;
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
    registerMemoryStatsTool(api, context);
    registerMemoryListTool(api, context);
  }
  if (options.enableSelfImprovementTools !== false) {
    const passthroughCtx: SelfImprovementToolContext = { workspaceDir: options.defaultWorkspaceDir };
    registerSelfImprovementLogTool(api, passthroughCtx);
    if (options.enableManagementTools) {
      registerSelfImprovementExtractSkillTool(api, passthroughCtx);
      registerSelfImprovementReviewTool(api, passthroughCtx);
    }
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
            api.logger.warn?.(`memory-lancedb-pro: memory_recall skipped (${warning})`);
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
