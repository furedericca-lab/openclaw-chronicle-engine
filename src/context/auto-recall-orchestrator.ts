import type { BackendRecallGenericRow } from "../backend-client/types.js";
import { selectFinalAutoRecallResults } from "../auto-recall-final-selection.js";
import type { RecallResultRow } from "../memory-record-types.js";
import {
  filterByMaxAge,
  keepMostRecentPerNormalizedKey,
  normalizeRecallTextKey,
  orchestrateDynamicRecall,
  type DynamicRecallResult,
  type DynamicRecallSessionState,
} from "../recall-engine.js";

export type AutoRecallSelectionMode = "mmr" | "setwise-v2";
export type AutoRecallCategory = "preference" | "fact" | "decision" | "entity" | "other" | "reflection";

export interface AutoRecallPlannerConfig {
  enabled: boolean;
  minPromptLength?: number;
  minRepeated?: number;
  topK: number;
  selectionMode: AutoRecallSelectionMode;
  categories?: AutoRecallCategory[];
  excludeReflection?: boolean;
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
    const fetchLimit = Math.min(20, Math.max(topK * 4, topK, 8));

    return await orchestrateDynamicRecall({
      channelName: "auto-recall",
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
          limit: fetchLimit,
          agentId,
          sessionId,
          sessionKey: params.sessionKey,
          userId: params.userId,
        }));
        const postProcessed = postProcessAutoRecallResults(retrieved, config);
        if (config.selectionMode === "setwise-v2") {
          return selectFinalAutoRecallResults(postProcessed, { topK });
        }
        return postProcessed.slice(0, topK);
      },
      formatLine: (row) =>
        `- [${row.entry.category}:${row.entry.scope}] ${deps.sanitizeForContext(row.entry.text)} ` +
        `(${(row.score * 100).toFixed(0)}%${row.sources?.bm25 ? ", vector+BM25" : ""}${row.sources?.reranked ? "+reranked" : ""})`,
    });
  };

  return { plan };
}

function postProcessAutoRecallResults(
  results: RecallResultRow[],
  config: Pick<AutoRecallPlannerConfig, "categories" | "excludeReflection" | "maxAgeDays" | "maxEntriesPerKey">
): RecallResultRow[] {
  const allowlisted = Array.isArray(config.categories) && config.categories.length > 0
    ? results.filter((row) => config.categories!.includes(row.entry.category as AutoRecallCategory))
    : results;
  const withoutReflection = config.excludeReflection === true
    ? allowlisted.filter((row) => row.entry.category !== "reflection")
    : allowlisted;
  const maxAgeMs = daysToMs(config.maxAgeDays);
  const withinAge = filterByMaxAge({
    items: withoutReflection,
    maxAgeMs,
    getTimestamp: (row) => row.entry.timestamp,
  });
  const cappedRecent = keepMostRecentPerNormalizedKey({
    items: withinAge,
    maxEntriesPerKey: config.maxEntriesPerKey,
    getTimestamp: (row) => row.entry.timestamp,
    getNormalizedKey: (row) => normalizeRecallTextKey(row.entry.text),
  });
  const allowedIds = new Set(cappedRecent.map((row) => row.entry.id));
  return withinAge.filter((row) => allowedIds.has(row.entry.id));
}

function daysToMs(days: number | undefined): number | undefined {
  if (!Number.isFinite(days) || Number(days) <= 0) return undefined;
  return Number(days) * 24 * 60 * 60 * 1000;
}

function normalizePositiveInt(value: unknown, fallback: number): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) return Math.max(1, Math.floor(fallback));
  return Math.max(1, Math.floor(parsed));
}

function mapBackendRowsToRecallResults(rows: BackendRecallGenericRow[]): RecallResultRow[] {
  return rows.map((row) => ({
    entry: {
      id: row.id,
      text: row.text,
      vector: [],
      category: row.category,
      scope: row.scope,
      importance: 1,
      timestamp: Number(row.metadata?.updatedAt ?? row.metadata?.createdAt ?? Date.now()),
    },
    score: Number.isFinite(row.score) ? Number(row.score) : 0,
    sources: {},
  }));
}
