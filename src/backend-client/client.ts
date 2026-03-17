import { randomUUID } from "node:crypto";
import type {
  BackendCallContext,
  BackendCaptureItem,
  BackendDeleteInput,
  BackendDistillJobResponse,
  BackendDistillJobStatusResponse,
  BackendListInput,
  BackendMemoryMutationResult,
  BackendReflectionJobResponse,
  BackendReflectionJobStatusResponse,
  BackendRecallGenericRow,
  BackendRecallReflectionRow,
  BackendStatsResponse,
  BackendStoreToolInput,
  BackendUpdateInput,
  MemoryBackendClient,
} from "./types.js";

export interface MemoryBackendClientConfig {
  baseUrl: string;
  bearerToken: string;
  timeoutMs: number;
  maxRetries: number;
  retryBaseDelayMs: number;
  logger?: {
    debug?: (message: string) => void;
    warn?: (message: string) => void;
  };
}

interface ErrorEnvelope {
  error?: {
    code?: string;
    message?: string;
    retryable?: boolean;
    details?: Record<string, unknown>;
  };
}

export class MemoryBackendClientError extends Error {
  readonly code: string;
  readonly status: number;
  readonly retryable: boolean;
  readonly details: Record<string, unknown>;

  constructor(params: {
    message: string;
    code: string;
    status?: number;
    retryable?: boolean;
    details?: Record<string, unknown>;
  }) {
    super(params.message);
    this.name = "MemoryBackendClientError";
    this.code = params.code;
    this.status = Number.isFinite(params.status) ? Number(params.status) : 0;
    this.retryable = params.retryable === true;
    this.details = params.details || {};
  }
}

export function createMemoryBackendClient(config: MemoryBackendClientConfig): MemoryBackendClient {
  const baseUrl = normalizeBaseUrl(config.baseUrl);
  const bearerToken = config.bearerToken.trim();
  const timeoutMs = Math.max(1000, Math.floor(config.timeoutMs || 10_000));
  const maxRetries = Math.max(0, Math.floor(config.maxRetries || 0));
  const retryBaseDelayMs = Math.max(50, Math.floor(config.retryBaseDelayMs || 250));

  if (!baseUrl) {
    throw new Error("remoteBackend.baseURL is required");
  }
  if (!bearerToken) {
    throw new Error("remoteBackend.authToken is required");
  }

  const requestJson = async <T>(params: {
    method: "GET" | "POST";
    path: string;
    ctx: BackendCallContext;
    body?: Record<string, unknown>;
    idempotencyKey?: string;
  }): Promise<T> => {
    let attempt = 0;
    let lastError: unknown;
    const requestId = params.ctx.requestId?.trim() || randomUUID();
    const idempotencyKey = params.idempotencyKey?.trim();

    while (attempt <= maxRetries) {
      if (attempt > 0) {
        await sleep(retryBaseDelayMs * attempt);
      }
      try {
        const controller = new AbortController();
        const timer = setTimeout(() => controller.abort(), timeoutMs);
        const headers = buildHeaders({
          requestId,
          actorUserId: params.ctx.identity.userId,
          actorAgentId: params.ctx.identity.agentId,
          bearerToken,
          idempotencyKey,
        });

        const response = await fetch(`${baseUrl}${params.path}`, {
          method: params.method,
          headers,
          body: params.body ? JSON.stringify(params.body) : undefined,
          signal: controller.signal,
        }).finally(() => {
          clearTimeout(timer);
        });

        if (!response.ok) {
          const envelope = (await safeParseJson(response)) as ErrorEnvelope | null;
          const backendError = new MemoryBackendClientError({
            message:
              envelope?.error?.message ||
              `memory backend request failed with status ${response.status}`,
            code: envelope?.error?.code || "BACKEND_UNAVAILABLE",
            status: response.status,
            retryable: envelope?.error?.retryable === true || shouldRetryStatus(response.status),
            details: envelope?.error?.details || {},
          });
          if (attempt < maxRetries && backendError.retryable) {
            config.logger?.warn?.(
              `memory-backend: retrying ${params.method} ${params.path} after ${backendError.code} (attempt ${attempt + 1}/${maxRetries})`
            );
            attempt += 1;
            lastError = backendError;
            continue;
          }
          throw backendError;
        }

        const parsed = (await safeParseJson(response)) as T | null;
        if (parsed === null) {
          throw new MemoryBackendClientError({
            message: `memory backend returned empty JSON for ${params.method} ${params.path}`,
            code: "BACKEND_UNAVAILABLE",
            status: response.status,
            retryable: false,
          });
        }
        return parsed;
      } catch (error) {
        const backendError = toBackendError(error, params.path);
        if (attempt < maxRetries && backendError.retryable) {
          config.logger?.warn?.(
            `memory-backend: retrying ${params.method} ${params.path} after transport failure (attempt ${attempt + 1}/${maxRetries})`
          );
          attempt += 1;
          lastError = backendError;
          continue;
        }
        throw backendError;
      }
    }
    throw toBackendError(lastError, params.path);
  };

  const withActor = (ctx: BackendCallContext) => ({ actor: ctx.actor });

  return {
    async recallGeneric(ctx, input) {
      const body = {
        ...withActor(ctx),
        query: input.query,
        limit: clampInt(input.limit, 1, 200),
      };
      const response = await requestJson<{ rows: BackendRecallGenericRow[] }>({
        method: "POST",
        path: "/v1/recall/generic",
        ctx,
        body,
      });
      return Array.isArray(response.rows) ? response.rows : [];
    },

    async recallReflection(ctx, input) {
      const body = {
        ...withActor(ctx),
        query: input.query,
        mode: input.mode,
        limit: clampInt(input.limit, 1, 200),
      };
      const response = await requestJson<{ rows: BackendRecallReflectionRow[] }>({
        method: "POST",
        path: "/v1/recall/reflection",
        ctx,
        body,
      });
      return Array.isArray(response.rows) ? response.rows : [];
    },

    async storeToolMemory(ctx, input: BackendStoreToolInput) {
      const response = await requestJson<{ results: BackendMemoryMutationResult[] }>({
        method: "POST",
        path: "/v1/memories/store",
        ctx,
        idempotencyKey: randomUUID(),
        body: {
          ...withActor(ctx),
          mode: "tool-store",
          memory: {
            text: input.text,
            category: input.category,
            importance: Number.isFinite(input.importance) ? input.importance : undefined,
          },
        },
      });
      return Array.isArray(response.results) ? response.results : [];
    },

    async storeAutoCapture(ctx, input: { items: BackendCaptureItem[] }) {
      const response = await requestJson<{ results: BackendMemoryMutationResult[] }>({
        method: "POST",
        path: "/v1/memories/store",
        ctx,
        idempotencyKey: randomUUID(),
        body: {
          ...withActor(ctx),
          mode: "auto-capture",
          items: input.items,
        },
      });
      return Array.isArray(response.results) ? response.results : [];
    },

    async updateMemory(ctx, input: BackendUpdateInput) {
      const response = await requestJson<{ result: BackendMemoryMutationResult }>({
        method: "POST",
        path: "/v1/memories/update",
        ctx,
        idempotencyKey: randomUUID(),
        body: {
          ...withActor(ctx),
          memoryId: input.memoryId,
          patch: input.patch,
        },
      });
      return response.result;
    },

    async deleteMemory(ctx, input: BackendDeleteInput) {
      const response = await requestJson<{ deleted: number }>({
        method: "POST",
        path: "/v1/memories/delete",
        ctx,
        idempotencyKey: randomUUID(),
        body: {
          ...withActor(ctx),
          memoryId: input.memoryId,
          query: input.query,
        },
      });
      return { deleted: Number(response.deleted || 0) };
    },

    async listMemories(ctx, input: BackendListInput) {
      const response = await requestJson<{ rows: any[]; nextOffset?: number | null }>({
        method: "POST",
        path: "/v1/memories/list",
        ctx,
        body: {
          ...withActor(ctx),
          limit: clampInt(input.limit, 1, 200),
          offset: clampInt(input.offset, 0, 1_000_000),
          category: input.category,
        },
      });
      return {
        rows: Array.isArray(response.rows) ? response.rows : [],
        nextOffset:
          response.nextOffset === null || response.nextOffset === undefined
            ? null
            : Number(response.nextOffset),
      };
    },

    async stats(ctx) {
      const response = await requestJson<BackendStatsResponse>({
        method: "POST",
        path: "/v1/memories/stats",
        ctx,
        body: withActor(ctx),
      });
      return response;
    },

    async enqueueReflectionJob(ctx, input) {
      const response = await requestJson<BackendReflectionJobResponse>({
        method: "POST",
        path: "/v1/reflection/jobs",
        ctx,
        idempotencyKey: input.idempotencyKey || randomUUID(),
        body: {
          ...withActor(ctx),
          trigger: input.trigger,
          messages: input.messages,
        },
      });
      return response;
    },

    async getReflectionJobStatus(ctx, input) {
      const response = await requestJson<BackendReflectionJobStatusResponse>({
        method: "GET",
        path: `/v1/reflection/jobs/${encodeURIComponent(input.jobId)}`,
        ctx,
      });
      return response;
    },

    async enqueueDistillJob(ctx, input) {
      const response = await requestJson<BackendDistillJobResponse>({
        method: "POST",
        path: "/v1/distill/jobs",
        ctx,
        idempotencyKey: input.idempotencyKey || randomUUID(),
        body: {
          ...withActor(ctx),
          mode: input.mode,
          source: input.source,
          options: input.options,
        },
      });
      return response;
    },

    async getDistillJobStatus(ctx, input) {
      const response = await requestJson<BackendDistillJobStatusResponse>({
        method: "GET",
        path: `/v1/distill/jobs/${encodeURIComponent(input.jobId)}`,
        ctx,
      });
      return response;
    },
  };
}

function buildHeaders(params: {
  requestId: string;
  actorUserId: string;
  actorAgentId: string;
  bearerToken: string;
  idempotencyKey?: string;
}): Record<string, string> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
    authorization: `Bearer ${params.bearerToken}`,
    "x-request-id": params.requestId,
    "x-auth-user-id": params.actorUserId,
    "x-auth-agent-id": params.actorAgentId,
  };
  if (params.idempotencyKey) {
    headers["idempotency-key"] = params.idempotencyKey;
  }
  return headers;
}

function normalizeBaseUrl(value: string): string {
  const trimmed = (value || "").trim();
  if (!trimmed) return "";
  return trimmed.endsWith("/") ? trimmed.slice(0, -1) : trimmed;
}

function clampInt(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.max(min, Math.min(max, Math.floor(value)));
}

function shouldRetryStatus(status: number): boolean {
  return status === 408 || status === 429 || status >= 500;
}

async function safeParseJson(response: Response): Promise<unknown | null> {
  const text = await response.text();
  if (!text.trim()) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function toBackendError(error: unknown, path: string): MemoryBackendClientError {
  if (error instanceof MemoryBackendClientError) {
    return error;
  }
  if (error instanceof Error && error.name === "AbortError") {
    return new MemoryBackendClientError({
      message: `memory backend request timed out for ${path}`,
      code: "BACKEND_UNAVAILABLE",
      retryable: true,
    });
  }
  return new MemoryBackendClientError({
    message: `memory backend transport error for ${path}: ${error instanceof Error ? error.message : String(error)}`,
    code: "BACKEND_UNAVAILABLE",
    retryable: true,
  });
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
