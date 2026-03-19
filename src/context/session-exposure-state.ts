import {
  clearDynamicRecallSessionState,
  createDynamicRecallSessionState,
  type DynamicRecallSessionState,
} from "./recall-engine.js";

export type BehavioralGuidanceErrorSignal = {
  at: number;
  toolName: string;
  summary: string;
  source: "tool_error" | "tool_output";
  signature: string;
  signatureHash: string;
};

type BehavioralGuidanceErrorState = {
  entries: BehavioralGuidanceErrorSignal[];
  lastInjectedCount: number;
  signatureSet: Set<string>;
  updatedAt: number;
};

export interface SessionExposureStateOptions {
  maxTrackedSessions?: number;
  behavioralGuidanceErrorSessionTtlMs?: number;
  behavioralGuidanceErrorSessionEntryLimit?: number;
}

export interface SessionExposureState {
  autoRecallState: DynamicRecallSessionState;
  behavioralRecallState: DynamicRecallSessionState;
  clearDynamicRecallForContext: (ctx: { sessionId?: unknown; sessionKey?: unknown }) => void;
  addBehavioralGuidanceErrorSignal: (sessionKey: string, signal: BehavioralGuidanceErrorSignal, dedupeEnabled: boolean) => void;
  getPendingBehavioralGuidanceErrorSignalsForPrompt: (sessionKey: string, maxEntries: number) => BehavioralGuidanceErrorSignal[];
  getRecentBehavioralGuidanceErrorSignals: (sessionKey: string, maxEntries: number) => BehavioralGuidanceErrorSignal[];
  clearBehavioralGuidanceErrorSignalsForSession: (sessionKey: string) => void;
  pruneBehavioralGuidanceSessionState: (now?: number) => void;
}

export const DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS = 200;
export const DEFAULT_BEHAVIORAL_GUIDANCE_ERROR_SESSION_TTL_MS = 30 * 60 * 1000;
const DEFAULT_BEHAVIORAL_GUIDANCE_ERROR_SESSION_ENTRY_LIMIT = 30;

export function createSessionExposureState(options?: SessionExposureStateOptions): SessionExposureState {
  const maxTrackedSessions = normalizePositiveInt(
    options?.maxTrackedSessions,
    DEFAULT_SESSION_EXPOSURE_MAX_TRACKED_SESSIONS
  );
  const behavioralGuidanceErrorSessionTtlMs = normalizePositiveInt(
    options?.behavioralGuidanceErrorSessionTtlMs,
    DEFAULT_BEHAVIORAL_GUIDANCE_ERROR_SESSION_TTL_MS
  );
  const behavioralGuidanceErrorSessionEntryLimit = normalizePositiveInt(
    options?.behavioralGuidanceErrorSessionEntryLimit,
    DEFAULT_BEHAVIORAL_GUIDANCE_ERROR_SESSION_ENTRY_LIMIT
  );

  const behavioralGuidanceErrorStateBySession = new Map<string, BehavioralGuidanceErrorState>();
  const autoRecallState = createDynamicRecallSessionState({ maxSessions: maxTrackedSessions });
  const behavioralRecallState = createDynamicRecallSessionState({ maxSessions: maxTrackedSessions });

  const pruneOldestByUpdatedAt = <T extends { updatedAt: number }>(map: Map<string, T>, maxSize: number) => {
    if (map.size <= maxSize) return;
    const sorted = [...map.entries()].sort((a, b) => a[1].updatedAt - b[1].updatedAt);
    const removeCount = map.size - maxSize;
    for (let i = 0; i < removeCount; i += 1) {
      const key = sorted[i]?.[0];
      if (key) map.delete(key);
    }
  };

  const pruneBehavioralGuidanceSessionState = (now = Date.now()) => {
    for (const [key, state] of behavioralGuidanceErrorStateBySession.entries()) {
      if (now - state.updatedAt > behavioralGuidanceErrorSessionTtlMs) {
        behavioralGuidanceErrorStateBySession.delete(key);
      }
    }
    pruneOldestByUpdatedAt(behavioralGuidanceErrorStateBySession, maxTrackedSessions);
  };

  const getBehavioralGuidanceErrorState = (sessionKey: string): BehavioralGuidanceErrorState => {
    const key = sessionKey.trim();
    const current = behavioralGuidanceErrorStateBySession.get(key);
    if (current) {
      current.updatedAt = Date.now();
      return current;
    }
    const created: BehavioralGuidanceErrorState = {
      entries: [],
      lastInjectedCount: 0,
      signatureSet: new Set<string>(),
      updatedAt: Date.now(),
    };
    behavioralGuidanceErrorStateBySession.set(key, created);
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
      clearDynamicRecallSessionState(behavioralRecallState, sessionId);
    }
  };

  const addBehavioralGuidanceErrorSignal = (
    sessionKey: string,
    signal: BehavioralGuidanceErrorSignal,
    dedupeEnabled: boolean
  ) => {
    const normalizedSessionKey = sessionKey.trim();
    if (!normalizedSessionKey) return;
    pruneBehavioralGuidanceSessionState();
    const state = getBehavioralGuidanceErrorState(normalizedSessionKey);
    if (dedupeEnabled && state.signatureSet.has(signal.signatureHash)) return;
    state.entries.push(signal);
    state.signatureSet.add(signal.signatureHash);
    state.updatedAt = Date.now();
    if (state.entries.length > behavioralGuidanceErrorSessionEntryLimit) {
      const removed = state.entries.length - behavioralGuidanceErrorSessionEntryLimit;
      state.entries.splice(0, removed);
      state.lastInjectedCount = Math.max(0, state.lastInjectedCount - removed);
      state.signatureSet = new Set(state.entries.map((entry) => entry.signatureHash));
    }
  };

  const getPendingBehavioralGuidanceErrorSignalsForPrompt = (
    sessionKey: string,
    maxEntries: number
  ): BehavioralGuidanceErrorSignal[] => {
    pruneBehavioralGuidanceSessionState();
    const state = behavioralGuidanceErrorStateBySession.get(sessionKey.trim());
    if (!state) return [];
    state.updatedAt = Date.now();
    state.lastInjectedCount = Math.min(state.lastInjectedCount, state.entries.length);
    const pending = state.entries.slice(state.lastInjectedCount);
    if (pending.length === 0) return [];
    const bounded = pending.slice(-normalizePositiveInt(maxEntries, pending.length));
    state.lastInjectedCount = state.entries.length;
    return bounded;
  };

  const getRecentBehavioralGuidanceErrorSignals = (
    sessionKey: string,
    maxEntries: number
  ): BehavioralGuidanceErrorSignal[] => {
    pruneBehavioralGuidanceSessionState();
    const state = behavioralGuidanceErrorStateBySession.get(sessionKey.trim());
    if (!state) return [];
    state.updatedAt = Date.now();
    const limit = normalizePositiveInt(maxEntries, state.entries.length || 1);
    return state.entries.slice(-limit);
  };

  const clearBehavioralGuidanceErrorSignalsForSession = (sessionKey: string) => {
    const normalized = sessionKey.trim();
    if (!normalized) return;
    behavioralGuidanceErrorStateBySession.delete(normalized);
  };

  return {
    autoRecallState,
    behavioralRecallState,
    clearDynamicRecallForContext,
    addBehavioralGuidanceErrorSignal,
    getPendingBehavioralGuidanceErrorSignalsForPrompt,
    getRecentBehavioralGuidanceErrorSignals,
    clearBehavioralGuidanceErrorSignalsForSession,
    pruneBehavioralGuidanceSessionState,
  };
}

function normalizePositiveInt(value: unknown, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return Math.max(1, Math.floor(fallback));
  return Math.max(1, Math.floor(parsed));
}
