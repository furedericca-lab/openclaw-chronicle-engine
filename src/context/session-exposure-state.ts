import {
  clearDynamicRecallSessionState,
  createDynamicRecallSessionState,
  type DynamicRecallSessionState,
} from "./recall-engine.js";

export type ReflectionErrorSignal = {
  at: number;
  toolName: string;
  summary: string;
  source: "tool_error" | "tool_output";
  signature: string;
  signatureHash: string;
};

type ReflectionErrorState = {
  entries: ReflectionErrorSignal[];
  lastInjectedCount: number;
  signatureSet: Set<string>;
  updatedAt: number;
};

export interface SessionExposureStateOptions {
  maxTrackedSessions?: number;
  reflectionErrorSessionTtlMs?: number;
  reflectionErrorSessionEntryLimit?: number;
}

export interface SessionExposureState {
  autoRecallState: DynamicRecallSessionState;
  reflectionRecallState: DynamicRecallSessionState;
  clearDynamicRecallForContext: (ctx: { sessionId?: unknown; sessionKey?: unknown }) => void;
  addReflectionErrorSignal: (sessionKey: string, signal: ReflectionErrorSignal, dedupeEnabled: boolean) => void;
  getPendingReflectionErrorSignalsForPrompt: (sessionKey: string, maxEntries: number) => ReflectionErrorSignal[];
  getRecentReflectionErrorSignals: (sessionKey: string, maxEntries: number) => ReflectionErrorSignal[];
  clearReflectionErrorSignalsForSession: (sessionKey: string) => void;
  pruneReflectionSessionState: (now?: number) => void;
}

export const DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS = 200;
export const DEFAULT_REFLECTION_ERROR_SESSION_TTL_MS = 30 * 60 * 1000;
const DEFAULT_REFLECTION_ERROR_SESSION_ENTRY_LIMIT = 30;

export function createSessionExposureState(options?: SessionExposureStateOptions): SessionExposureState {
  const maxTrackedSessions = normalizePositiveInt(
    options?.maxTrackedSessions,
    DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS
  );
  const reflectionErrorSessionTtlMs = normalizePositiveInt(
    options?.reflectionErrorSessionTtlMs,
    DEFAULT_REFLECTION_ERROR_SESSION_TTL_MS
  );
  const reflectionErrorSessionEntryLimit = normalizePositiveInt(
    options?.reflectionErrorSessionEntryLimit,
    DEFAULT_REFLECTION_ERROR_SESSION_ENTRY_LIMIT
  );

  const reflectionErrorStateBySession = new Map<string, ReflectionErrorState>();
  const autoRecallState = createDynamicRecallSessionState({ maxSessions: maxTrackedSessions });
  const reflectionRecallState = createDynamicRecallSessionState({ maxSessions: maxTrackedSessions });

  const pruneOldestByUpdatedAt = <T extends { updatedAt: number }>(map: Map<string, T>, maxSize: number) => {
    if (map.size <= maxSize) return;
    const sorted = [...map.entries()].sort((a, b) => a[1].updatedAt - b[1].updatedAt);
    const removeCount = map.size - maxSize;
    for (let i = 0; i < removeCount; i += 1) {
      const key = sorted[i]?.[0];
      if (key) map.delete(key);
    }
  };

  const pruneReflectionSessionState = (now = Date.now()) => {
    for (const [key, state] of reflectionErrorStateBySession.entries()) {
      if (now - state.updatedAt > reflectionErrorSessionTtlMs) {
        reflectionErrorStateBySession.delete(key);
      }
    }
    pruneOldestByUpdatedAt(reflectionErrorStateBySession, maxTrackedSessions);
  };

  const getReflectionErrorState = (sessionKey: string): ReflectionErrorState => {
    const key = sessionKey.trim();
    const current = reflectionErrorStateBySession.get(key);
    if (current) {
      current.updatedAt = Date.now();
      return current;
    }
    const created: ReflectionErrorState = {
      entries: [],
      lastInjectedCount: 0,
      signatureSet: new Set<string>(),
      updatedAt: Date.now(),
    };
    reflectionErrorStateBySession.set(key, created);
    return created;
  };

  const clearDynamicRecallForContext = (ctx: { sessionId?: unknown; sessionKey?: unknown }) => {
    const sessionIds = new Set<string>();
    if (typeof ctx.sessionId === "string" && ctx.sessionId.trim()) {
      sessionIds.add(ctx.sessionId.trim());
    }
    if (typeof ctx.sessionKey === "string" && ctx.sessionKey.trim()) {
      sessionIds.add(ctx.sessionKey.trim());
    }
    for (const sessionId of sessionIds) {
      clearDynamicRecallSessionState(autoRecallState, sessionId);
      clearDynamicRecallSessionState(reflectionRecallState, sessionId);
    }
  };

  const addReflectionErrorSignal = (sessionKey: string, signal: ReflectionErrorSignal, dedupeEnabled: boolean) => {
    const normalizedSessionKey = sessionKey.trim();
    if (!normalizedSessionKey) return;
    pruneReflectionSessionState();
    const state = getReflectionErrorState(normalizedSessionKey);
    if (dedupeEnabled && state.signatureSet.has(signal.signatureHash)) return;
    state.entries.push(signal);
    state.signatureSet.add(signal.signatureHash);
    state.updatedAt = Date.now();
    if (state.entries.length > reflectionErrorSessionEntryLimit) {
      const removed = state.entries.length - reflectionErrorSessionEntryLimit;
      state.entries.splice(0, removed);
      state.lastInjectedCount = Math.max(0, state.lastInjectedCount - removed);
      state.signatureSet = new Set(state.entries.map((entry) => entry.signatureHash));
    }
  };

  const getPendingReflectionErrorSignalsForPrompt = (sessionKey: string, maxEntries: number): ReflectionErrorSignal[] => {
    pruneReflectionSessionState();
    const state = reflectionErrorStateBySession.get(sessionKey.trim());
    if (!state) return [];
    state.updatedAt = Date.now();
    state.lastInjectedCount = Math.min(state.lastInjectedCount, state.entries.length);
    const pending = state.entries.slice(state.lastInjectedCount);
    if (pending.length === 0) return [];
    const bounded = pending.slice(-normalizePositiveInt(maxEntries, pending.length));
    state.lastInjectedCount = state.entries.length;
    return bounded;
  };

  const getRecentReflectionErrorSignals = (sessionKey: string, maxEntries: number): ReflectionErrorSignal[] => {
    pruneReflectionSessionState();
    const state = reflectionErrorStateBySession.get(sessionKey.trim());
    if (!state) return [];
    state.updatedAt = Date.now();
    const limit = normalizePositiveInt(maxEntries, state.entries.length || 1);
    return state.entries.slice(-limit);
  };

  const clearReflectionErrorSignalsForSession = (sessionKey: string) => {
    const normalized = sessionKey.trim();
    if (!normalized) return;
    reflectionErrorStateBySession.delete(normalized);
  };

  return {
    autoRecallState,
    reflectionRecallState,
    clearDynamicRecallForContext,
    addReflectionErrorSignal,
    getPendingReflectionErrorSignalsForPrompt,
    getRecentReflectionErrorSignals,
    clearReflectionErrorSignalsForSession,
    pruneReflectionSessionState,
  };
}

function normalizePositiveInt(value: unknown, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return Math.max(1, Math.floor(fallback));
  return Math.max(1, Math.floor(parsed));
}
