import type { RetrievalResult } from "../retriever.js";
import { selectFinalAutoRecallResults } from "../auto-recall-final-selection.js";
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
  retrieve: (params: {
    query: string;
    limit: number;
    scopeFilter: string[];
    source: "auto-recall";
  }) => Promise<RetrievalResult[]>;
  getAccessibleScopes: (agentId: string) => string[];
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
}

export function createAutoRecallPlanner(
  config: AutoRecallPlannerConfig,
  deps: AutoRecallPlannerDependencies
): { plan: (params: AutoRecallPlanParams) => Promise<DynamicRecallResult | undefined> } {
  const plan = async (params: AutoRecallPlanParams): Promise<DynamicRecallResult | undefined> => {
    if (config.enabled !== true) return undefined;

    const agentId = typeof params.agentId === "string" && params.agentId.trim() ? params.agentId.trim() : "main";
    const sessionId = typeof params.sessionId === "string" && params.sessionId.trim() ? params.sessionId.trim() : "default";
    const topK = Math.max(1, normalizePositiveInt(config.topK, 3));
    const fetchLimit = Math.min(20, Math.max(topK * 4, topK, 8));
    const accessibleScopes = deps.getAccessibleScopes(agentId);

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
        const retrieved = await deps.retrieve({
          query: String(params.prompt || ""),
          limit: fetchLimit,
          scopeFilter: accessibleScopes,
          source: "auto-recall",
        });
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
  results: RetrievalResult[],
  config: Pick<AutoRecallPlannerConfig, "categories" | "excludeReflection" | "maxAgeDays" | "maxEntriesPerKey">
): RetrievalResult[] {
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
