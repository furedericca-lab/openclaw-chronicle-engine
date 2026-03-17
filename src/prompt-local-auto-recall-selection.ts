import type { RecallResultRow } from "./memory-record-types.js";
import { normalizeRecallTextKey } from "./recall-engine.js";
import {
  DEFAULT_PROMPT_LOCAL_SELECTION_FRESHNESS_HALF_LIFE_MS,
  type PromptLocalOverlapThreshold,
  type PromptLocalSetwiseCandidate,
  selectPromptLocalTopKSetwise,
} from "./prompt-local-topk-setwise-selection.js";

export interface AutoRecallFinalSelectionOptions {
  topK?: number;
  now?: number;
  shortlistLimit?: number;
}

const GENERIC_OVERLAP_THRESHOLDS: PromptLocalOverlapThreshold[] = [
  { minOverlap: 0.86, multiplier: 0.2 },
  { minOverlap: 0.72, multiplier: 0.45 },
  { minOverlap: 0.58, multiplier: 0.75 },
];

export function selectPromptLocalAutoRecallResults(
  results: RecallResultRow[],
  options: AutoRecallFinalSelectionOptions = {}
): RecallResultRow[] {
  if (!Array.isArray(results) || results.length === 0) return [];

  const finalLimit = Math.min(results.length, normalizeLimit(options.topK, results.length));
  if (finalLimit <= 0) return [];
  const shortlistLimit = Math.min(
    results.length,
    normalizeLimit(options.shortlistLimit, Math.max(finalLimit, finalLimit * 4))
  );

  const candidates: PromptLocalSetwiseCandidate<RecallResultRow>[] = results.map((row) => {
    const normalizedKey = normalizeRecallTextKey(row.entry.text);
    return {
      id: row.entry.id,
      text: row.entry.text,
      baseScore: Number.isFinite(row.score) ? row.score : 0,
      ts: row.entry.timestamp,
      softKey: normalizedKey || undefined,
      normalizedKey: normalizedKey || undefined,
      category: row.entry.category,
      scope: row.entry.scope,
      raw: row,
    };
  });

  return selectPromptLocalTopKSetwise(candidates, {
    finalLimit,
    shortlistLimit,
    now: options.now,
    freshnessHalfLifeMs: DEFAULT_PROMPT_LOCAL_SELECTION_FRESHNESS_HALF_LIFE_MS,
    weights: {
      relevance: 1,
      freshness: 0.08,
      categoryCoverage: 0.05,
      scopeCoverage: 0.03,
    },
    penalties: {
      sameKeyMultiplier: 0.08,
      overlapThresholds: GENERIC_OVERLAP_THRESHOLDS,
    },
  }).map((row) => row.raw);
}

function normalizeLimit(value: unknown, fallback: number): number {
  const resolved = Number.isFinite(value) ? Number(value) : fallback;
  return Math.max(1, Math.floor(resolved));
}
