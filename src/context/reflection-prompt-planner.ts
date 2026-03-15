import type { MemoryEntry } from "../store.js";
import type { BackendRecallReflectionRow } from "../backend-client/types.js";
import { loadAgentReflectionSlicesFromEntries, DEFAULT_REFLECTION_DERIVED_MAX_AGE_MS } from "../reflection-store.js";
import { rankDynamicReflectionRecallFromEntries } from "../reflection-recall.js";
import { orchestrateDynamicRecall } from "../recall-engine.js";
import type { ReflectionItemKind } from "../reflection-item-store.js";
import {
  joinPrependContextBlocks,
  renderErrorDetectedBlock,
  renderInheritedRulesBlock,
} from "./prompt-block-renderer.js";
import { extractReflectionErrorSignalFromToolCall, type ReflectionToolCallEvent } from "./reflection-error-signals.js";
import type { ReflectionErrorSignal, SessionExposureState } from "./session-exposure-state.js";

export type ReflectionInjectMode = "inheritance-only" | "inheritance+derived";
export type ReflectionRecallMode = "fixed" | "dynamic";

export interface ReflectionPromptPlannerConfig {
  injectMode: ReflectionInjectMode;
  dedupeErrorSignals: boolean;
  errorReminderMaxEntries: number;
  errorScanMaxChars: number;
  recall: {
    mode: ReflectionRecallMode;
    topK: number;
    includeKinds: ReflectionItemKind[];
    maxAgeDays: number;
    maxEntriesPerKey: number;
    minRepeated: number;
    minScore: number;
    minPromptLength: number;
  };
}

export interface ReflectionPromptPlannerDependencies {
  sessionState: SessionExposureState;
  recallReflection?: (params: {
    prompt?: string;
    agentId: string;
    sessionId: string;
    sessionKey?: string;
    userId?: string;
    mode: "invariant-only" | "invariant+derived";
    limit: number;
  }) => Promise<BackendRecallReflectionRow[]>;
  storeList?: (scopeFilter: string[], category?: string, limit?: number, offset?: number) => Promise<MemoryEntry[]>;
  getAccessibleScopes?: (agentId: string) => string[];
  sanitizeForContext: (text: string) => string;
  logger?: {
    info?: (message: string) => void;
    debug?: (message: string) => void;
    warn?: (message: string) => void;
  };
}

export function createReflectionPromptPlanner(
  config: ReflectionPromptPlannerConfig,
  deps: ReflectionPromptPlannerDependencies
): {
  captureAfterToolCall: (event: ReflectionToolCallEvent, sessionKey: string) => void;
  buildBeforePromptPrependContext: (params: {
    prompt?: string;
    agentId?: string;
    sessionId?: string;
    sessionKey?: string;
    userId?: string;
  }) => Promise<string | undefined>;
  getRecentErrorSignals: (sessionKey: string, maxEntries: number) => ReflectionErrorSignal[];
  clearSession: (context: { sessionKey?: string; sessionId?: string }) => void;
  pruneSessionState: () => void;
  invalidateAgentCache: (agentId: string) => void;
} {
  const reflectionByAgentCache = new Map<string, { updatedAt: number; invariants: string[]; derived: string[] }>();
  const REFLECTION_CACHE_TTL_MS = 15_000;

  const loadAgentReflectionSlices = async (agentId: string, scopeFilter: string[]) => {
    if (!deps.storeList) {
      return { updatedAt: Date.now(), invariants: [], derived: [] };
    }
    const cacheKey = `${agentId}::${[...scopeFilter].sort().join(",")}`;
    const cached = reflectionByAgentCache.get(cacheKey);
    if (cached && Date.now() - cached.updatedAt < REFLECTION_CACHE_TTL_MS) return cached;

    const entries = await deps.storeList(scopeFilter, undefined, 120, 0);
    const { invariants, derived } = loadAgentReflectionSlicesFromEntries({
      entries,
      agentId,
      deriveMaxAgeMs: DEFAULT_REFLECTION_DERIVED_MAX_AGE_MS,
    });
    const next = { updatedAt: Date.now(), invariants, derived };
    reflectionByAgentCache.set(cacheKey, next);
    return next;
  };

  const buildReflectionRecallPrependContext = async (params: {
    prompt?: string;
    agentId?: string;
    sessionId?: string;
    sessionKey?: string;
    userId?: string;
  }): Promise<string | undefined> => {
    const agentId = typeof params.agentId === "string" && params.agentId.trim() ? params.agentId.trim() : "main";
    const scopes = deps.getAccessibleScopes ? deps.getAccessibleScopes(agentId) : [];
    const sessionId = params.sessionId || params.sessionKey || "default";

    if (deps.recallReflection) {
      if (config.recall.mode === "fixed") {
        const rows = await deps.recallReflection({
          prompt: params.prompt,
          agentId,
          sessionId,
          sessionKey: params.sessionKey,
          userId: params.userId,
          mode: "invariant-only",
          limit: Math.max(1, Math.min(20, normalizePositiveInt(config.recall.topK, 6))),
        });
        const visible = rows
          .filter((row) => config.recall.includeKinds.includes(row.kind))
          .slice(0, 6)
          .map((row, index) => `${index + 1}. ${deps.sanitizeForContext(row.text)}`);
        if (visible.length === 0) return undefined;
        return renderInheritedRulesBlock(visible);
      }

      const topK = Math.max(1, normalizePositiveInt(config.recall.topK, 1));
      const fetchLimit = Math.min(60, Math.max(topK * 4, topK));
      const result = await orchestrateDynamicRecall({
        channelName: "reflection-recall",
        prompt: params.prompt,
        minPromptLength: config.recall.minPromptLength,
        minRepeated: config.recall.minRepeated,
        topK,
        sessionId,
        state: deps.sessionState.reflectionRecallState,
        outputTag: "inherited-rules",
        headerLines: [
          "Dynamic rules selected by Reflection-Recall. Treat as long-term behavioral constraints unless user overrides.",
        ],
        logger: deps.logger,
        loadCandidates: async () => {
          const rows = await deps.recallReflection!({
            prompt: params.prompt,
            agentId,
            sessionId,
            sessionKey: params.sessionKey,
            userId: params.userId,
            mode: "invariant+derived",
            limit: fetchLimit,
          });
          return rows
            .filter((row) => config.recall.includeKinds.includes(row.kind))
            .filter((row) => Number.isFinite(row.score) && row.score >= config.recall.minScore)
            .slice(0, topK);
        },
        formatLine: (row, index) =>
          `${index + 1}. ${deps.sanitizeForContext(row.text)} (${(row.score * 100).toFixed(0)}%)`,
      });
      return result?.prependContext;
    }

    if (config.recall.mode === "fixed") {
      const slices = await loadAgentReflectionSlices(agentId, scopes);
      if (slices.invariants.length === 0) return undefined;
      return renderInheritedRulesBlock(
        slices.invariants.slice(0, 6).map((line, index) => `${index + 1}. ${line}`)
      );
    }

    const topK = Math.max(1, normalizePositiveInt(config.recall.topK, 1));
    const listLimit = Math.min(800, Math.max(topK * 40, 240));
    const result = await orchestrateDynamicRecall({
      channelName: "reflection-recall",
      prompt: params.prompt,
      minPromptLength: config.recall.minPromptLength,
      minRepeated: config.recall.minRepeated,
      topK,
      sessionId,
      state: deps.sessionState.reflectionRecallState,
      outputTag: "inherited-rules",
      headerLines: [
        "Dynamic rules selected by Reflection-Recall. Treat as long-term behavioral constraints unless user overrides.",
      ],
      logger: deps.logger,
      loadCandidates: async () => {
        const entries = await deps.storeList(scopes, "reflection", listLimit, 0);
        return rankDynamicReflectionRecallFromEntries(entries, {
          agentId,
          includeKinds: config.recall.includeKinds,
          topK,
          maxAgeMs: daysToMs(config.recall.maxAgeDays),
          maxEntriesPerKey: config.recall.maxEntriesPerKey,
          minScore: config.recall.minScore,
        });
      },
      formatLine: (row, index) =>
        `${index + 1}. ${deps.sanitizeForContext(row.text)} (${(row.score * 100).toFixed(0)}%)`,
    });
    return result?.prependContext;
  };

  const captureAfterToolCall = (event: ReflectionToolCallEvent, sessionKey: string) => {
    if (!sessionKey.trim()) return;
    deps.sessionState.pruneReflectionSessionState();
    const signal = extractReflectionErrorSignalFromToolCall(event, config.errorScanMaxChars);
    if (!signal) return;
    deps.sessionState.addReflectionErrorSignal(sessionKey, signal, config.dedupeErrorSignals);
  };

  const buildBeforePromptPrependContext = async (params: {
    prompt?: string;
    agentId?: string;
    sessionId?: string;
    sessionKey?: string;
    userId?: string;
  }): Promise<string | undefined> => {
    deps.sessionState.pruneReflectionSessionState();
    const blocks: string[] = [];

    if (config.injectMode === "inheritance-only" || config.injectMode === "inheritance+derived") {
      try {
        const inherited = await buildReflectionRecallPrependContext(params);
        if (inherited) blocks.push(inherited);
      } catch (err) {
        deps.logger?.warn?.(`memory-reflection: reflection-recall injection failed: ${String(err)}`);
      }
    }

    const sessionKey = typeof params.sessionKey === "string" ? params.sessionKey : "";
    if (sessionKey) {
      const pending = deps.sessionState.getPendingReflectionErrorSignalsForPrompt(
        sessionKey,
        config.errorReminderMaxEntries
      );
      const errorBlock = renderErrorDetectedBlock(pending);
      if (errorBlock) blocks.push(errorBlock);
    }

    return joinPrependContextBlocks(blocks);
  };

  const getRecentErrorSignals = (sessionKey: string, maxEntries: number): ReflectionErrorSignal[] =>
    deps.sessionState.getRecentReflectionErrorSignals(sessionKey, maxEntries);

  const clearSession = (context: { sessionKey?: string; sessionId?: string }) => {
    const sessionKey = typeof context.sessionKey === "string" ? context.sessionKey.trim() : "";
    if (sessionKey) {
      deps.sessionState.clearReflectionErrorSignalsForSession(sessionKey);
    }
    deps.sessionState.clearDynamicRecallForContext({
      sessionKey,
      sessionId: context.sessionId,
    });
  };

  const pruneSessionState = () => {
    deps.sessionState.pruneReflectionSessionState();
  };

  const invalidateAgentCache = (agentId: string) => {
    const prefix = `${agentId.trim()}::`;
    for (const cacheKey of reflectionByAgentCache.keys()) {
      if (cacheKey.startsWith(prefix)) {
        reflectionByAgentCache.delete(cacheKey);
      }
    }
  };

  return {
    captureAfterToolCall,
    buildBeforePromptPrependContext,
    getRecentErrorSignals,
    clearSession,
    pruneSessionState,
    invalidateAgentCache,
  };
}

function daysToMs(days: number): number | undefined {
  if (!Number.isFinite(days) || Number(days) <= 0) return undefined;
  return Number(days) * 24 * 60 * 60 * 1000;
}

function normalizePositiveInt(value: unknown, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return Math.max(1, Math.floor(fallback));
  return Math.max(1, Math.floor(parsed));
}
