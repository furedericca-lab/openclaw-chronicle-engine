import { randomUUID } from "node:crypto";
import type { BackendCallContext } from "./types.js";

export interface RuntimeContextDefaults {
  sessionIdPrefix?: string;
}

export interface RuntimeContextOverrides {
  userId?: string;
  agentId?: string;
  sessionId?: string;
  sessionKey?: string;
  requestId?: string;
}

export interface BackendCallContextResolution {
  context: BackendCallContext;
  hasPrincipalIdentity: boolean;
  missingPrincipalFields: Array<"userId" | "agentId">;
}

export class MissingRuntimePrincipalError extends Error {
  readonly missingPrincipalFields: Array<"userId" | "agentId">;

  constructor(missingPrincipalFields: Array<"userId" | "agentId">) {
    super(`missing runtime principal identity: ${missingPrincipalFields.join(", ")}`);
    this.name = "MissingRuntimePrincipalError";
    this.missingPrincipalFields = missingPrincipalFields;
  }
}

const USER_ID_PATHS = [
  "auth.userId",
  "principal.userId",
  "identity.userId",
  "user.id",
  "authUserId",
  "principalUserId",
  "userId",
];

const AGENT_ID_PATHS = [
  "auth.agentId",
  "principal.agentId",
  "identity.agentId",
  "agent.id",
  "authAgentId",
  "principalAgentId",
  "agentId",
];

const SESSION_ID_PATHS = [
  "session.id",
  "context.sessionId",
  "sessionId",
];

const SESSION_KEY_PATHS = [
  "session.key",
  "context.sessionKey",
  "sessionKey",
];

const REQUEST_ID_PATHS = [
  "request.id",
  "context.requestId",
  "requestId",
  "traceId",
];

export function buildBackendCallContext(
  input: unknown,
  defaults: RuntimeContextDefaults,
  overrides?: RuntimeContextOverrides
): BackendCallContext {
  const resolved = resolveBackendCallContext(input, defaults, overrides);
  if (!resolved.hasPrincipalIdentity) {
    throw new MissingRuntimePrincipalError(resolved.missingPrincipalFields);
  }
  return resolved.context;
}

export function resolveBackendCallContext(
  input: unknown,
  defaults: RuntimeContextDefaults,
  overrides?: RuntimeContextOverrides
): BackendCallContextResolution {
  const merged = normalizeInput(input);
  const rawSessionKey = firstNonEmptyString(
    overrides?.sessionKey,
    readByPaths(merged, SESSION_KEY_PATHS)
  );

  const userId = firstNonEmptyString(
    overrides?.userId,
    readByPaths(merged, USER_ID_PATHS)
  );
  const agentId = firstNonEmptyString(
    overrides?.agentId,
    readByPaths(merged, AGENT_ID_PATHS)
  );
  const sessionId = firstNonEmptyString(
    overrides?.sessionId,
    readByPaths(merged, SESSION_ID_PATHS),
    `${defaults.sessionIdPrefix || "session"}-${Date.now()}-${randomUUID().slice(0, 8)}`
  );
  const generatedSessionKey = agentId
    ? `agent:${agentId}:session:${sessionId}`
    : `session:${sessionId}`;
  const sessionKey = firstNonEmptyString(rawSessionKey, generatedSessionKey);
  const requestId = firstNonEmptyString(
    overrides?.requestId,
    readByPaths(merged, REQUEST_ID_PATHS),
    randomUUID()
  );
  const missingPrincipalFields: Array<"userId" | "agentId"> = [];
  if (!userId) missingPrincipalFields.push("userId");
  if (!agentId) missingPrincipalFields.push("agentId");
  const hasPrincipalIdentity = missingPrincipalFields.length === 0;

  return {
    context: {
      requestId,
      identity: { userId, agentId },
      actor: {
        userId,
        agentId,
        sessionId,
        sessionKey,
      },
    },
    hasPrincipalIdentity,
    missingPrincipalFields,
  };
}

function normalizeInput(input: unknown): Record<string, unknown> {
  if (!input || typeof input !== "object") return {};
  return input as Record<string, unknown>;
}

function readByPaths(input: Record<string, unknown>, paths: string[]): string | undefined {
  for (const path of paths) {
    const value = readPath(input, path);
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return undefined;
}

function readPath(input: Record<string, unknown>, path: string): unknown {
  const parts = path.split(".");
  let cursor: unknown = input;
  for (const part of parts) {
    if (!cursor || typeof cursor !== "object") return undefined;
    cursor = (cursor as Record<string, unknown>)[part];
  }
  return cursor;
}

function firstNonEmptyString(...values: Array<string | undefined>): string {
  for (const value of values) {
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return "";
}
