import type { MemoryCategory } from "./backend-client/types.js";

export interface MemoryEntry {
  id: string;
  text: string;
  vector: number[];
  category: MemoryCategory;
  scope: string;
  importance: number;
  timestamp: number;
  metadata?: string;
}

export interface RecallResultSources {
  vector?: { score: number; rank: number };
  bm25?: { score: number; rank: number };
  fused?: { score: number };
  reranked?: { score: number };
}

export interface RecallResultRow {
  entry: MemoryEntry;
  score: number;
  sources: RecallResultSources;
}
