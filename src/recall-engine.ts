import { shouldSkipRetrieval } from "./adaptive-retrieval.js";
import { renderTaggedPromptBlock } from "./context/prompt-block-renderer.js";

export interface DynamicRecallSessionState {
  historyBySession: Map<string, Map<string, number>>;
  turnCounterBySession: Map<string, number>;
  updatedAtBySession: Map<string, number>;
  maxSessions: number;
}

export interface DynamicRecallCandidate {
  id: string;
  text: string;
  score: number;
}

export interface DynamicRecallResult {
  prependContext: string;
  injectedCount: number;
}

interface DynamicRecallLogger {
  info?: (message: string) => void;
  debug?: (message: string) => void;
}

interface OrchestrateDynamicRecallParams<T extends DynamicRecallCandidate> {
  channelName: string;
  prompt: string | undefined;
  minPromptLength?: number;
  minRepeated?: number;
  topK: number;
  sessionId: string;
  state: DynamicRecallSessionState;
  outputTag: string;
  headerLines: string[];
  wrapUntrustedData?: boolean;
  logger?: DynamicRecallLogger;
  loadCandidates: () => Promise<T[]>;
  formatLine: (candidate: T, index: number) => string;
}

interface CreateDynamicRecallSessionStateOptions {
  maxSessions?: number;
}

const DEFAULT_DYNAMIC_RECALL_MAX_SESSIONS = 200;

export function createDynamicRecallSessionState(
  options?: CreateDynamicRecallSessionStateOptions
): DynamicRecallSessionState {
  return {
    historyBySession: new Map<string, Map<string, number>>(),
    turnCounterBySession: new Map<string, number>(),
    updatedAtBySession: new Map<string, number>(),
    maxSessions: normalizeMaxSessions(options?.maxSessions),
  };
}

export function clearDynamicRecallSessionState(state: DynamicRecallSessionState, sessionId: string): void {
  const key = String(sessionId || "").trim();
  if (!key) return;
  state.historyBySession.delete(key);
  state.turnCounterBySession.delete(key);
  state.updatedAtBySession.delete(key);
}

export async function orchestrateDynamicRecall<T extends DynamicRecallCandidate>(
  params: OrchestrateDynamicRecallParams<T>
): Promise<DynamicRecallResult | undefined> {
  if (!params.prompt || shouldSkipRetrieval(params.prompt, params.minPromptLength)) return undefined;

  const topK = Number.isFinite(params.topK) ? Math.max(1, Math.floor(params.topK)) : 1;
  const sessionId = params.sessionId || "default";
  touchDynamicRecallSessionState(params.state, sessionId);
  const currentTurn = (params.state.turnCounterBySession.get(sessionId) || 0) + 1;
  params.state.turnCounterBySession.set(sessionId, currentTurn);

  const loaded = await params.loadCandidates();
  if (loaded.length === 0) return undefined;

  const sliced = loaded.slice(0, topK);
  const minRepeated = Number.isFinite(params.minRepeated) ? Math.max(0, Math.floor(Number(params.minRepeated))) : 0;
  const sessionHistory = params.state.historyBySession.get(sessionId) || new Map<string, number>();

  const injected = minRepeated > 0
    ? sliced.filter((candidate) => {
      const lastTurn = sessionHistory.get(candidate.id) ?? -999_999;
      const turnsSinceLastInjection = currentTurn - lastTurn;
      return turnsSinceLastInjection >= minRepeated;
    })
    : sliced;

  if (injected.length === 0) {
    params.logger?.debug?.(
      `openclaw-chronicle-engine: ${params.channelName} skipped due to repeated-injection guard (session=${sessionId}, turn=${currentTurn})`
    );
    return undefined;
  }

  for (const candidate of injected) {
    sessionHistory.set(candidate.id, currentTurn);
  }
  params.state.historyBySession.set(sessionId, sessionHistory);

  const memoryLines = injected
    .map((candidate, idx) => params.formatLine(candidate, idx))
    .filter((line) => typeof line === "string" && line.trim().length > 0);

  if (memoryLines.length === 0) return undefined;

  params.logger?.info?.(
    `openclaw-chronicle-engine: ${params.channelName} injecting ${memoryLines.length} row(s) for session=${sessionId}`
  );

  return {
    prependContext: renderTaggedPromptBlock({
      tag: params.outputTag,
      headerLines: params.headerLines,
      contentLines: memoryLines,
      wrapUntrustedData: params.wrapUntrustedData === true,
    }),
    injectedCount: memoryLines.length,
  };
}

export function normalizeRecallTextKey(text: string): string {
  return String(text)
    .trim()
    .replace(/\s+/g, " ")
    .toLowerCase();
}

function normalizeMaxSessions(value: unknown): number {
  if (typeof value === "number" && Number.isFinite(value) && value > 0) return Math.floor(value);
  return DEFAULT_DYNAMIC_RECALL_MAX_SESSIONS;
}

function touchDynamicRecallSessionState(state: DynamicRecallSessionState, sessionId: string): void {
  const key = String(sessionId || "").trim();
  if (!key) return;
  state.updatedAtBySession.set(key, Date.now());
  pruneDynamicRecallSessionState(state);
}

function pruneDynamicRecallSessionState(state: DynamicRecallSessionState): void {
  const maxSessions = normalizeMaxSessions(state.maxSessions);
  state.maxSessions = maxSessions;

  const sessionIds = new Set<string>([
    ...state.historyBySession.keys(),
    ...state.turnCounterBySession.keys(),
    ...state.updatedAtBySession.keys(),
  ]);
  if (sessionIds.size <= maxSessions) return;

  const staleCandidates = [...sessionIds]
    .map((sessionId) => ({ sessionId, updatedAt: state.updatedAtBySession.get(sessionId) || 0 }))
    .sort((a, b) => a.updatedAt - b.updatedAt);

  const removeCount = sessionIds.size - maxSessions;
  for (let i = 0; i < removeCount; i += 1) {
    const victim = staleCandidates[i];
    if (!victim) break;
    clearDynamicRecallSessionState(state, victim.sessionId);
  }
}
