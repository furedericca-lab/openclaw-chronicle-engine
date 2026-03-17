import type { BackendRecallReflectionRow } from "../backend-client/types.js";
import { orchestrateDynamicRecall } from "../recall-engine.js";
import {
  joinPrependContextBlocks,
  renderErrorDetectedBlock,
  renderInheritedRulesBlock,
} from "./prompt-block-renderer.js";
import { extractReflectionErrorSignalFromToolCall, type ReflectionToolCallEvent } from "./reflection-error-signals.js";
import type { ReflectionErrorSignal, SessionExposureState } from "./session-exposure-state.js";

export type ReflectionInjectMode = "inheritance-only" | "inheritance+derived";
export type ReflectionRecallMode = "fixed" | "dynamic";
export type ReflectionItemKind = "invariant" | "derived";

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
  recallReflection: (params: {
    prompt?: string;
    agentId: string;
    sessionId: string;
    sessionKey?: string;
    userId?: string;
    mode: "invariant-only" | "invariant+derived";
    limit: number;
  }) => Promise<BackendRecallReflectionRow[]>;
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
  invalidateAgentCache: (_agentId: string) => void;
} {
  if (typeof deps.recallReflection !== "function") {
    throw new Error("reflection prompt planner requires remote recallReflection dependency");
  }

  const buildReflectionRecallPrependContext = async (params: {
    prompt?: string;
    agentId?: string;
    sessionId?: string;
    sessionKey?: string;
    userId?: string;
  }): Promise<string | undefined> => {
    const agentId = typeof params.agentId === "string" && params.agentId.trim() ? params.agentId.trim() : "main";
    const sessionId = params.sessionId || params.sessionKey || "default";

    if (config.recall.mode === "fixed") {
      const rows = await deps.recallReflection({
        prompt: params.prompt,
        agentId,
        sessionId,
        sessionKey: params.sessionKey,
        userId: params.userId,
        mode: "invariant-only",
        limit: Math.max(1, Math.min(20, normalizePositiveInt(config.recall.topK, 6))),
        includeKinds: config.recall.includeKinds,
      });
      const visible = rows
        .slice(0, 6)
        .map((row, index) => `${index + 1}. ${deps.sanitizeForContext(row.text)}`);
      if (visible.length === 0) return undefined;
      return renderInheritedRulesBlock(visible);
    }

    const topK = Math.max(1, normalizePositiveInt(config.recall.topK, 1));
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
        const rows = await deps.recallReflection({
          prompt: params.prompt,
          agentId,
          sessionId,
          sessionKey: params.sessionKey,
          userId: params.userId,
          mode: "invariant+derived",
          limit: topK,
          includeKinds: config.recall.includeKinds,
          minScore: config.recall.minScore,
        });
        return rows.slice(0, topK);
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

  return {
    captureAfterToolCall,
    buildBeforePromptPrependContext,
    getRecentErrorSignals,
    clearSession,
    pruneSessionState,
    invalidateAgentCache: () => {},
  };
}

function normalizePositiveInt(value: unknown, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return Math.max(1, Math.floor(fallback));
  return Math.max(1, Math.floor(parsed));
}
