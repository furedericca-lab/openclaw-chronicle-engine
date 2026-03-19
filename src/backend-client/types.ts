export type MemoryCategory =
  | "preference"
  | "fact"
  | "decision"
  | "entity"
  | "reflection"
  | "other";

export type WritableMemoryCategory = Exclude<MemoryCategory, "reflection">;

export type BehavioralRecallMode = "invariant-only" | "invariant+derived";
export type MessageRole = "user" | "assistant" | "system";
export type DistillMode = "session-lessons" | "governance-candidates";
export type DistillPersistMode = "artifacts-only" | "persist-memory-rows";

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
  category?: WritableMemoryCategory;
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
    category?: WritableMemoryCategory;
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

export interface BackendBehavioralRecallRow {
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

export type BackendRetrievalTraceKind = "generic" | "reflection" | "behavioral";
export type BackendRetrievalTraceStageStatus = "ok" | "fallback" | "skipped" | "failed";

export interface BackendRetrievalTraceQuery {
  preview: string;
  rawLen: number;
  lexicalPreview: string;
  lexicalLen: number;
}

export interface BackendRetrievalTraceStage {
  name: string;
  status: BackendRetrievalTraceStageStatus;
  inputCount?: number;
  outputCount?: number;
  fallbackTo?: string;
  reason?: string;
  metrics?: Record<string, unknown>;
}

export interface BackendRetrievalTrace {
  kind: BackendRetrievalTraceKind;
  query: BackendRetrievalTraceQuery;
  mode?: string;
  stages: BackendRetrievalTraceStage[];
  finalRowIds: string[];
}

export interface BackendRecallGenericDebugResponse {
  rows: BackendRecallGenericRow[];
  trace: BackendRetrievalTrace;
}

export interface BackendBehavioralRecallDebugResponse {
  rows: BackendBehavioralRecallRow[];
  trace: BackendRetrievalTrace;
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
  behavioralCount: number;
  categories: Record<string, number>;
}

export interface BackendSessionTranscriptAppendResponse {
  appended: number;
}

export interface BackendDistillInlineSource {
  kind: "inline-messages";
  messages: BackendCaptureItem[];
}

export interface BackendDistillSessionTranscriptSource {
  kind: "session-transcript";
  sessionKey: string;
  sessionId?: string;
}

export type BackendDistillSource =
  | BackendDistillInlineSource
  | BackendDistillSessionTranscriptSource;

export interface BackendDistillJobResponse {
  jobId: string;
  status: "queued" | "running" | "completed" | "failed";
}

export interface BackendDistillJobStatusResponse extends BackendDistillJobResponse {
  mode: DistillMode;
  sourceKind: BackendDistillSource["kind"];
  createdAt: number;
  updatedAt: number;
  result?: {
    artifactCount: number;
    persistedMemoryCount: number;
    warnings: string[];
  };
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
    input: {
      query: string;
      limit: number;
      categories?: MemoryCategory[];
      excludeBehavioral?: boolean;
      maxAgeDays?: number;
      maxEntriesPerKey?: number;
    }
  ) => Promise<BackendRecallGenericRow[]>;
  recallGenericDebug: (
    ctx: BackendCallContext,
    input: {
      query: string;
      limit: number;
      categories?: MemoryCategory[];
      excludeBehavioral?: boolean;
      maxAgeDays?: number;
      maxEntriesPerKey?: number;
    }
  ) => Promise<BackendRecallGenericDebugResponse>;
  recallBehavioral: (
    ctx: BackendCallContext,
    input: {
      query: string;
      mode: BehavioralRecallMode;
      limit: number;
      includeKinds?: Array<"invariant" | "derived">;
      minScore?: number;
    }
  ) => Promise<BackendBehavioralRecallRow[]>;
  recallBehavioralDebug: (
    ctx: BackendCallContext,
    input: {
      query: string;
      mode: BehavioralRecallMode;
      limit: number;
      includeKinds?: Array<"invariant" | "derived">;
      minScore?: number;
    }
  ) => Promise<BackendBehavioralRecallDebugResponse>;
  storeToolMemory: (
    ctx: BackendCallContext,
    input: BackendStoreToolInput
  ) => Promise<BackendMemoryMutationResult[]>;
  storeAutoCapture: (
    ctx: BackendCallContext,
    input: { items: BackendCaptureItem[] }
  ) => Promise<BackendMemoryMutationResult[]>;
  appendSessionTranscript: (
    ctx: BackendCallContext,
    input: { items: BackendCaptureItem[]; idempotencyKey?: string }
  ) => Promise<BackendSessionTranscriptAppendResponse>;
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
  enqueueDistillJob: (
    ctx: BackendCallContext,
    input: {
      mode: DistillMode;
      source: BackendDistillSource;
      options: {
        persistMode: DistillPersistMode;
        maxMessages?: number;
        chunkChars?: number;
        chunkOverlapMessages?: number;
        maxArtifacts?: number;
      };
      idempotencyKey?: string;
    }
  ) => Promise<BackendDistillJobResponse>;
  getDistillJobStatus: (
    ctx: BackendCallContext,
    input: { jobId: string }
  ) => Promise<BackendDistillJobStatusResponse>;
}
