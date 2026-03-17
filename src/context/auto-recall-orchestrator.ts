import type { BackendRecallGenericRow } from "../backend-client/types.js";
import type { RecallResultRow } from "../memory-record-types.js";
import {
  orchestrateDynamicRecall,
  type DynamicRecallResult,
  type DynamicRecallSessionState,
} from "../recall-engine.js";

export type AutoRecallSelectionMode = "mmr";
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
    categories?: AutoRecallCategory[];
    excludeReflection?: boolean;
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
          limit: topK,
          agentId,
          sessionId,
          sessionKey: params.sessionKey,
          userId: params.userId,
          categories: config.categories,
          excludeReflection: config.excludeReflection,
          maxAgeDays: config.maxAgeDays,
          maxEntriesPerKey: config.maxEntriesPerKey,
        }));
        return retrieved.slice(0, topK);
      },
      formatLine: (row) =>
        `- [${row.entry.category}:${row.entry.scope}] ${deps.sanitizeForContext(row.entry.text)} ` +
        `(${(row.score * 100).toFixed(0)}%)`,
    });
  };

  return { plan };
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
