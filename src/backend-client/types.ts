export type MemoryCategory =
  | "preference"
  | "fact"
  | "decision"
  | "entity"
  | "reflection"
  | "other";

export type ReflectionRecallMode = "invariant-only" | "invariant+derived";
export type ReflectionTrigger = "new" | "reset";
export type MessageRole = "user" | "assistant" | "system";

export interface BackendActor {
  userId: string;
  agentId: string;
  sessionId: string;
  sessionKey: string;
}

export interface BackendIdentity {
  userId: string;
  agentId: string;
}

export interface BackendCallContext {
  actor: BackendActor;
  identity: BackendIdentity;
  requestId?: string;
}

export interface BackendStoreToolInput {
  text: string;
  category?: MemoryCategory;
  importance?: number;
}

export interface BackendCaptureItem {
  role: MessageRole;
  text: string;
}

export interface BackendUpdateInput {
  memoryId: string;
  patch: {
    text?: string;
    category?: MemoryCategory;
    importance?: number;
  };
}

export interface BackendDeleteInput {
  memoryId?: string;
  query?: string;
}

export interface BackendListInput {
  limit: number;
  offset: number;
  category?: MemoryCategory;
}

export interface BackendMemoryMutationResult {
  id: string;
  action: "ADD" | "UPDATE" | "DELETE" | "NOOP";
  text: string;
  category: MemoryCategory;
  importance: number;
  scope: string;
}

export interface BackendRecallGenericRow {
  id: string;
  text: string;
  category: MemoryCategory;
  scope: string;
  score: number;
  metadata: {
    createdAt: number;
    updatedAt: number;
  };
}

export interface BackendRecallReflectionRow {
  id: string;
  text: string;
  kind: "invariant" | "derived";
  strictKey?: string;
  scope: string;
  score: number;
  metadata: {
    timestamp: number;
  };
}

export interface BackendListRow {
  id: string;
  text: string;
  category: MemoryCategory;
  scope: string;
  metadata: {
    createdAt: number;
    updatedAt: number;
  };
}

export interface BackendStatsResponse {
  memoryCount: number;
  reflectionCount: number;
  categories: Record<string, number>;
}

export interface BackendReflectionJobResponse {
  jobId: string;
  status: "queued" | "running" | "completed" | "failed";
}

export interface BackendReflectionJobStatusResponse extends BackendReflectionJobResponse {
  persisted?: boolean;
  memoryCount?: number;
  error?: {
    code: string;
    message: string;
    retryable: boolean;
    details: Record<string, unknown>;
  };
}

export interface MemoryBackendClient {
  recallGeneric: (
    ctx: BackendCallContext,
    input: { query: string; limit: number }
  ) => Promise<BackendRecallGenericRow[]>;
  recallReflection: (
    ctx: BackendCallContext,
    input: { query: string; mode: ReflectionRecallMode; limit: number }
  ) => Promise<BackendRecallReflectionRow[]>;
  storeToolMemory: (
    ctx: BackendCallContext,
    input: BackendStoreToolInput
  ) => Promise<BackendMemoryMutationResult[]>;
  storeAutoCapture: (
    ctx: BackendCallContext,
    input: { items: BackendCaptureItem[] }
  ) => Promise<BackendMemoryMutationResult[]>;
  updateMemory: (
    ctx: BackendCallContext,
    input: BackendUpdateInput
  ) => Promise<BackendMemoryMutationResult>;
  deleteMemory: (
    ctx: BackendCallContext,
    input: BackendDeleteInput
  ) => Promise<{ deleted: number }>;
  listMemories: (
    ctx: BackendCallContext,
    input: BackendListInput
  ) => Promise<{ rows: BackendListRow[]; nextOffset: number | null }>;
  stats: (ctx: BackendCallContext) => Promise<BackendStatsResponse>;
  enqueueReflectionJob: (
    ctx: BackendCallContext,
    input: { trigger: ReflectionTrigger; messages: BackendCaptureItem[]; idempotencyKey?: string }
  ) => Promise<BackendReflectionJobResponse>;
  getReflectionJobStatus: (
    ctx: BackendCallContext,
    input: { jobId: string }
  ) => Promise<BackendReflectionJobStatusResponse>;
}
