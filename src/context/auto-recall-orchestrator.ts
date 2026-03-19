import type { BackendBehavioralRecallRow, BackendRecallGenericRow } from "../backend-client/types.js";
import {
  extractBehavioralGuidanceErrorSignalFromToolCall,
  type BehavioralGuidanceToolCallEvent,
} from "./behavioral-guidance-error-signals.js";
import {
  orchestrateDynamicRecall,
  type DynamicRecallResult,
  type DynamicRecallSessionState,
} from "./recall-engine.js";
import {
  joinPrependContextBlocks,
  renderBehavioralGuidanceBlock,
  renderErrorDetectedBlock,
} from "./prompt-block-renderer.js";
import type { BehavioralGuidanceErrorSignal, SessionExposureState } from "./session-exposure-state.js";

export type AutoRecallSelectionMode = "mmr";
export type AutoRecallCategory = "preference" | "fact" | "decision" | "entity" | "other" | "reflection";
export type BehavioralGuidanceInjectMode = "durable-only" | "durable+adaptive";
export type BehavioralRecallMode = "fixed" | "dynamic";
export type BehavioralRecallKind = "durable" | "adaptive";

export interface AutoRecallPlannerConfig {
  enabled: boolean;
  minPromptLength?: number;
  minRepeated?: number;
  topK: number;
  selectionMode: AutoRecallSelectionMode;
  categories?: AutoRecallCategory[];
  excludeBehavioral?: boolean;
  maxAgeDays?: number;
  maxEntriesPerKey?: number;
}

export interface AutoRecallPlannerDependencies {
  state: DynamicRecallSessionState;
  recallGeneric: (params: {
    query: string;
    limit: number;
    agentId: string;
    sessionId: string;
    sessionKey?: string;
    userId?: string;
    categories?: AutoRecallCategory[];
    excludeBehavioral?: boolean;
    maxAgeDays?: number;
    maxEntriesPerKey?: number;
  }) => Promise<BackendRecallGenericRow[]>;
  sanitizeForContext: (text: string) => string;
  logger?: {
    info?: (message: string) => void;
    debug?: (message: string) => void;
  };
}

export interface AutoRecallPlanParams {
  prompt: string | undefined;
  agentId?: string;
  sessionId?: string;
  sessionKey?: string;
  userId?: string;
}

export interface AutoRecallBehavioralPlannerConfig {
  enabled: boolean;
  injectMode: BehavioralGuidanceInjectMode;
  dedupeErrorSignals: boolean;
  errorReminderMaxEntries: number;
  errorScanMaxChars: number;
  recall: {
    mode: BehavioralRecallMode;
    topK: number;
    includeKinds: BehavioralRecallKind[];
    maxAgeDays: number;
    maxEntriesPerKey: number;
    minRepeated: number;
    minScore: number;
    minPromptLength: number;
  };
}

export interface AutoRecallBehavioralPlannerDependencies {
  sessionState: SessionExposureState;
  recallBehavioral: (params: {
    prompt?: string;
    agentId: string;
    sessionId: string;
    sessionKey?: string;
    userId?: string;
    mode: "durable-only" | "durable+adaptive";
    limit: number;
    includeKinds?: BehavioralRecallKind[];
    minScore?: number;
  }) => Promise<BackendBehavioralRecallRow[]>;
  sanitizeForContext: (text: string) => string;
  logger?: {
    info?: (message: string) => void;
    debug?: (message: string) => void;
    warn?: (message: string) => void;
  };
}

interface AutoRecallResultRow {
  id: string;
  text: string;
  category: AutoRecallCategory;
  scope: string;
  score: number;
}

export function createAutoRecallPlanner(
  config: AutoRecallPlannerConfig,
  deps: AutoRecallPlannerDependencies
): { plan: (params: AutoRecallPlanParams) => Promise<DynamicRecallResult | undefined> } {
  if (typeof deps.recallGeneric !== "function") {
    throw new Error("auto-recall planner requires remote recallGeneric dependency");
  }

  const plan = async (params: AutoRecallPlanParams): Promise<DynamicRecallResult | undefined> => {
    if (config.enabled !== true) return undefined;

    const agentId = typeof params.agentId === "string" && params.agentId.trim() ? params.agentId.trim() : "main";
    const sessionId = typeof params.sessionId === "string" && params.sessionId.trim() ? params.sessionId.trim() : "default";
    const topK = Math.max(1, normalizePositiveInt(config.topK, 3));

    return await orchestrateDynamicRecall({
      channelName: "auto-recall-context",
      prompt: params.prompt,
      minPromptLength: config.minPromptLength,
      minRepeated: config.minRepeated,
      topK,
      sessionId,
      state: deps.state,
      outputTag: "relevant-memories",
      headerLines: [],
      wrapUntrustedData: true,
      logger: deps.logger,
      loadCandidates: async () => {
        const retrieved = mapBackendRowsToRecallResults(await deps.recallGeneric({
          query: String(params.prompt || ""),
          limit: topK,
          agentId,
          sessionId,
          sessionKey: params.sessionKey,
          userId: params.userId,
          categories: config.categories,
          excludeBehavioral: config.excludeBehavioral,
          maxAgeDays: config.maxAgeDays,
          maxEntriesPerKey: config.maxEntriesPerKey,
        }));
        return retrieved.slice(0, topK);
      },
      formatLine: (row) =>
        `- [${row.category}:${row.scope}] ${deps.sanitizeForContext(row.text)} ` +
        `(${(row.score * 100).toFixed(0)}%)`,
    });
  };

  return { plan };
}

export function createAutoRecallBehavioralPlanner(
  config: AutoRecallBehavioralPlannerConfig,
  deps: AutoRecallBehavioralPlannerDependencies
): {
  captureAfterToolCall: (event: BehavioralGuidanceToolCallEvent, sessionKey: string) => void;
  buildBeforePromptPrependContext: (params: {
    prompt?: string;
    agentId?: string;
    sessionId?: string;
    sessionKey?: string;
    userId?: string;
  }) => Promise<string | undefined>;
  getRecentErrorSignals: (sessionKey: string, maxEntries: number) => BehavioralGuidanceErrorSignal[];
  clearSession: (context: { sessionKey?: string; sessionId?: string }) => void;
  pruneSessionState: () => void;
  invalidateAgentCache: (_agentId: string) => void;
} {
  if (typeof deps.recallBehavioral !== "function") {
    throw new Error("auto-recall behavioral planner requires remote recallBehavioral dependency");
  }

  const buildBehavioralRecallPrependContext = async (params: {
    prompt?: string;
    agentId?: string;
    sessionId?: string;
    sessionKey?: string;
    userId?: string;
  }): Promise<string | undefined> => {
    const agentId = typeof params.agentId === "string" && params.agentId.trim() ? params.agentId.trim() : "main";
    const sessionId = params.sessionId || params.sessionKey || "default";

    if (config.recall.mode === "fixed") {
      const rows = await deps.recallBehavioral({
        prompt: params.prompt,
        agentId,
        sessionId,
        sessionKey: params.sessionKey,
        userId: params.userId,
        mode: "durable-only",
        limit: Math.max(1, Math.min(20, normalizePositiveInt(config.recall.topK, 6))),
        includeKinds: config.recall.includeKinds,
      });
      const visible = rows
        .slice(0, 6)
        .map((row, index) => `${index + 1}. ${deps.sanitizeForContext(row.text)}`);
      if (visible.length === 0) return undefined;
      return renderBehavioralGuidanceBlock(visible);
    }

    const topK = Math.max(1, normalizePositiveInt(config.recall.topK, 1));
    const result = await orchestrateDynamicRecall({
      channelName: "auto-recall-behavioral",
      prompt: params.prompt,
      minPromptLength: config.recall.minPromptLength,
      minRepeated: config.recall.minRepeated,
      topK,
      sessionId,
      state: deps.sessionState.behavioralRecallState,
      outputTag: "behavioral-guidance",
      headerLines: [
        "Dynamic behavioral guidance selected by Auto-Recall. Treat as durable guidance unless user or higher-priority system instructions override.",
      ],
      logger: deps.logger,
      loadCandidates: async () => {
        const rows = await deps.recallBehavioral({
          prompt: params.prompt,
          agentId,
          sessionId,
          sessionKey: params.sessionKey,
          userId: params.userId,
          mode: "durable+adaptive",
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

  const captureAfterToolCall = (event: BehavioralGuidanceToolCallEvent, sessionKey: string) => {
    if (!sessionKey.trim()) return;
    deps.sessionState.pruneBehavioralGuidanceSessionState();
    const signal = extractBehavioralGuidanceErrorSignalFromToolCall(event, config.errorScanMaxChars);
    if (!signal) return;
    deps.sessionState.addBehavioralGuidanceErrorSignal(sessionKey, signal, config.dedupeErrorSignals);
  };

  const buildBeforePromptPrependContext = async (params: {
    prompt?: string;
    agentId?: string;
    sessionId?: string;
    sessionKey?: string;
    userId?: string;
  }): Promise<string | undefined> => {
    deps.sessionState.pruneBehavioralGuidanceSessionState();
    const blocks: string[] = [];

    if (config.injectMode === "durable-only" || config.injectMode === "durable+adaptive") {
      try {
        const inherited = await buildBehavioralRecallPrependContext(params);
        if (inherited) blocks.push(inherited);
      } catch (err) {
        deps.logger?.warn?.(`auto-recall.behavioral-guidance: guidance injection failed: ${String(err)}`);
      }
    }

    const sessionKey = typeof params.sessionKey === "string" ? params.sessionKey : "";
    if (sessionKey) {
      const pending = deps.sessionState.getPendingBehavioralGuidanceErrorSignalsForPrompt(
        sessionKey,
        config.errorReminderMaxEntries
      );
      const errorBlock = renderErrorDetectedBlock(pending);
      if (errorBlock) blocks.push(errorBlock);
    }

    return joinPrependContextBlocks(blocks);
  };

  const getRecentErrorSignals = (sessionKey: string, maxEntries: number): BehavioralGuidanceErrorSignal[] =>
    deps.sessionState.getRecentBehavioralGuidanceErrorSignals(sessionKey, maxEntries);

  const clearSession = (context: { sessionKey?: string; sessionId?: string }) => {
    const sessionKey = typeof context.sessionKey === "string" ? context.sessionKey.trim() : "";
    if (sessionKey) {
      deps.sessionState.clearBehavioralGuidanceErrorSignalsForSession(sessionKey);
    }
    deps.sessionState.clearDynamicRecallForContext({
      sessionKey,
      sessionId: context.sessionId,
    });
  };

  const pruneSessionState = () => {
    deps.sessionState.pruneBehavioralGuidanceSessionState();
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

function mapBackendRowsToRecallResults(rows: BackendRecallGenericRow[]): AutoRecallResultRow[] {
  return rows.map((row) => ({
    id: row.id,
    text: row.text,
    category: row.category,
    scope: row.scope,
    score: Number.isFinite(row.score) ? Number(row.score) : 0,
  }));
}
