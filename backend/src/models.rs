use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_IMPORTANCE: f64 = 0.5;
pub const MAX_LIMIT: u64 = 200;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Actor {
    pub user_id: String,
    pub agent_id: String,
    pub session_id: String,
    pub session_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Principal {
    pub user_id: String,
    pub agent_id: String,
}

impl Actor {
    pub fn validate(&self) -> AppResult<()> {
        validate_non_empty("actor.userId", &self.user_id)?;
        validate_non_empty("actor.agentId", &self.agent_id)?;
        validate_non_empty("actor.sessionId", &self.session_id)?;
        validate_non_empty("actor.sessionKey", &self.session_key)?;
        Ok(())
    }

    pub fn principal(&self) -> Principal {
        Principal {
            user_id: self.user_id.clone(),
            agent_id: self.agent_id.clone(),
        }
    }

    pub fn derived_scope(&self) -> String {
        format!("agent:{}", self.agent_id)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Preference,
    Fact,
    Decision,
    Entity,
    Reflection,
    Other,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Preference => "preference",
            Self::Fact => "fact",
            Self::Decision => "decision",
            Self::Entity => "entity",
            Self::Reflection => "reflection",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoryAction {
    Add,
    Update,
    Delete,
    Noop,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "kebab-case", deny_unknown_fields)]
pub enum StoreRequest {
    ToolStore {
        actor: Actor,
        memory: ToolStoreMemory,
    },
    AutoCapture {
        actor: Actor,
        items: Vec<CaptureItem>,
    },
}

impl StoreRequest {
    pub fn actor(&self) -> &Actor {
        match self {
            Self::ToolStore { actor, .. } | Self::AutoCapture { actor, .. } => actor,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolStoreMemory {
    pub text: String,
    pub category: Option<Category>,
    pub importance: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureItem {
    pub role: MessageRole,
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMutationResult {
    pub id: String,
    pub action: MemoryAction,
    pub text: String,
    pub category: Category,
    pub importance: f64,
    pub scope: String,
}

#[derive(Debug, Serialize)]
pub struct StoreResponse {
    pub results: Vec<MemoryMutationResult>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateRequest {
    pub actor: Actor,
    pub memory_id: String,
    pub patch: UpdatePatch,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePatch {
    pub text: Option<String>,
    pub category: Option<Category>,
    pub importance: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct UpdateResponse {
    pub result: MemoryMutationResult,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteRequest {
    pub actor: Actor,
    pub memory_id: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListRequest {
    pub actor: Actor,
    pub limit: u64,
    pub offset: u64,
    pub category: Option<Category>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse {
    pub rows: Vec<ListRow>,
    pub next_offset: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ListRow {
    pub id: String,
    pub text: String,
    pub category: Category,
    pub scope: String,
    pub metadata: RowMetadata,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowMetadata {
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatsRequest {
    pub actor: Actor,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub memory_count: u64,
    pub reflection_count: u64,
    pub categories: BTreeMap<String, u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecallGenericRequest {
    pub actor: Actor,
    pub query: String,
    pub limit: u64,
    #[serde(default)]
    pub categories: Option<Vec<Category>>,
    #[serde(default)]
    pub exclude_reflection: Option<bool>,
    #[serde(default)]
    pub max_age_days: Option<u64>,
    #[serde(default)]
    pub max_entries_per_key: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecallReflectionRequest {
    pub actor: Actor,
    pub query: String,
    pub mode: Option<ReflectionRecallMode>,
    pub limit: u64,
    #[serde(default)]
    pub include_kinds: Option<Vec<ReflectionKind>>,
    #[serde(default)]
    pub min_score: Option<f64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ReflectionRecallMode {
    #[serde(rename = "invariant-only")]
    InvariantOnly,
    #[serde(rename = "invariant+derived")]
    InvariantDerived,
}

#[derive(Debug, Serialize)]
pub struct RecallGenericResponse {
    pub rows: Vec<RecallGenericRow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallGenericDebugResponse {
    pub rows: Vec<RecallGenericRow>,
    pub trace: RetrievalTrace,
}

#[derive(Debug, Serialize)]
pub struct RecallReflectionResponse {
    pub rows: Vec<RecallReflectionRow>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallReflectionDebugResponse {
    pub rows: Vec<RecallReflectionRow>,
    pub trace: RetrievalTrace,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RetrievalTraceKind {
    Generic,
    Reflection,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RetrievalTraceStageStatus {
    Ok,
    Fallback,
    Skipped,
    Failed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalTrace {
    pub kind: RetrievalTraceKind,
    pub query: RetrievalTraceQuery,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    pub stages: Vec<RetrievalTraceStage>,
    pub final_row_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalTraceQuery {
    pub preview: String,
    pub raw_len: usize,
    pub lexical_preview: String,
    pub lexical_len: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalTraceStage {
    pub name: String,
    pub status: RetrievalTraceStageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub metrics: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallGenericRow {
    pub id: String,
    pub text: String,
    pub category: Category,
    pub scope: String,
    pub score: f64,
    pub metadata: RowMetadata,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum ReflectionKind {
    Invariant,
    Derived,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallReflectionRow {
    pub id: String,
    pub text: String,
    pub kind: ReflectionKind,
    pub strict_key: Option<String>,
    pub scope: String,
    pub score: f64,
    pub metadata: ReflectionMetadata,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReflectionMetadata {
    pub timestamp: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppendSessionTranscriptRequest {
    pub actor: Actor,
    pub items: Vec<CaptureItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnqueueDistillJobRequest {
    pub actor: Actor,
    pub mode: DistillMode,
    pub source: DistillSource,
    pub options: DistillOptions,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DistillMode {
    SessionLessons,
    GovernanceCandidates,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DistillSource {
    SessionTranscript {
        #[serde(rename = "sessionKey")]
        session_key: String,
        #[serde(default)]
        #[serde(rename = "sessionId")]
        session_id: Option<String>,
    },
    InlineMessages {
        messages: Vec<CaptureItem>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DistillOptions {
    #[serde(default)]
    pub max_messages: Option<u64>,
    #[serde(default)]
    pub chunk_chars: Option<u64>,
    #[serde(default)]
    pub chunk_overlap_messages: Option<u64>,
    #[serde(default)]
    pub max_artifacts: Option<u64>,
    pub persist_mode: DistillPersistMode,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DistillPersistMode {
    ArtifactsOnly,
    PersistMemoryRows,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppendSessionTranscriptResponse {
    pub appended: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueDistillJobResponse {
    pub job_id: String,
    pub status: DistillJobStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DistillJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillJobStatusResponse {
    pub job_id: String,
    pub status: DistillJobStatus,
    pub mode: DistillMode,
    pub source_kind: DistillSourceKind,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<DistillJobResultSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JobStatusError>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DistillSourceKind {
    SessionTranscript,
    InlineMessages,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillJobResultSummary {
    pub artifact_count: u64,
    pub persisted_memory_count: u64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillArtifact {
    pub artifact_id: String,
    pub job_id: String,
    pub kind: DistillArtifactKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<DistillArtifactSubtype>,
    pub category: Category,
    pub importance: f64,
    pub text: String,
    pub evidence: Vec<DistillArtifactEvidence>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistence: Option<DistillArtifactPersistence>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DistillArtifactKind {
    Lesson,
    GovernanceCandidate,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DistillArtifactSubtype {
    FollowUpFocus,
    NextTurnGuidance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillArtifactEvidence {
    pub message_ids: Vec<u64>,
    pub quote: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistillArtifactPersistence {
    pub persist_mode: DistillPersistMode,
    pub persisted_memory_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

pub fn validate_non_empty(field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::invalid_request(format!(
            "{field} cannot be empty"
        )));
    }
    Ok(())
}

pub fn validate_importance(value: f64) -> AppResult<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(AppError::invalid_request(
            "importance must be within [0, 1]",
        ));
    }
    Ok(())
}

pub fn validate_limit(field: &str, value: u64) -> AppResult<()> {
    if value == 0 {
        return Err(AppError::invalid_request(format!("{field} must be > 0")));
    }
    Ok(())
}

pub fn clamped_limit(value: u64) -> u64 {
    value.min(MAX_LIMIT)
}

pub fn validate_store_request(req: &StoreRequest) -> AppResult<()> {
    match req {
        StoreRequest::ToolStore { actor, memory } => {
            actor.validate()?;
            validate_non_empty("memory.text", &memory.text)?;
            if let Some(importance) = memory.importance {
                validate_importance(importance)?;
            }
        }
        StoreRequest::AutoCapture { actor, items } => {
            actor.validate()?;
            if items.is_empty() {
                return Err(AppError::invalid_request(
                    "items must be a non-empty array for auto-capture",
                ));
            }
            for item in items {
                validate_non_empty("items[].text", &item.text)?;
            }
        }
    }
    Ok(())
}

pub fn validate_update_request(req: &UpdateRequest) -> AppResult<()> {
    req.actor.validate()?;
    validate_non_empty("memoryId", &req.memory_id)?;
    if req.patch.text.is_none() && req.patch.category.is_none() && req.patch.importance.is_none() {
        return Err(AppError::invalid_request(
            "patch must include at least one of text/category/importance",
        ));
    }
    if let Some(text) = &req.patch.text {
        validate_non_empty("patch.text", text)?;
    }
    if let Some(importance) = req.patch.importance {
        validate_importance(importance)?;
    }
    Ok(())
}

pub fn validate_delete_request(req: &DeleteRequest) -> AppResult<()> {
    req.actor.validate()?;
    match (&req.memory_id, &req.query) {
        (Some(memory_id), None) => validate_non_empty("memoryId", memory_id),
        (None, Some(query)) => validate_non_empty("query", query),
        _ => Err(AppError::invalid_request(
            "exactly one of memoryId or query is required",
        )),
    }
}

pub fn validate_list_request(req: &ListRequest) -> AppResult<()> {
    req.actor.validate()?;
    validate_limit("limit", req.limit)
}

pub fn validate_stats_request(req: &StatsRequest) -> AppResult<()> {
    req.actor.validate()
}

pub fn validate_recall_generic_request(req: &RecallGenericRequest) -> AppResult<()> {
    req.actor.validate()?;
    validate_non_empty("query", &req.query)?;
    validate_limit("limit", req.limit)?;
    if let Some(max_age_days) = req.max_age_days {
        validate_limit("maxAgeDays", max_age_days)?;
    }
    if let Some(max_entries_per_key) = req.max_entries_per_key {
        validate_limit("maxEntriesPerKey", max_entries_per_key)?;
    }
    Ok(())
}

pub fn validate_recall_reflection_request(req: &RecallReflectionRequest) -> AppResult<()> {
    req.actor.validate()?;
    validate_non_empty("query", &req.query)?;
    validate_limit("limit", req.limit)?;
    if let Some(min_score) = req.min_score {
        if !(0.0..=1.0).contains(&min_score) {
            return Err(AppError::invalid_request("minScore must be within [0, 1]"));
        }
    }
    Ok(())
}

pub fn validate_append_session_transcript_request(
    req: &AppendSessionTranscriptRequest,
) -> AppResult<()> {
    req.actor.validate()?;
    if req.items.is_empty() {
        return Err(AppError::invalid_request("items must be non-empty"));
    }
    for item in &req.items {
        validate_non_empty("items[].text", &item.text)?;
    }
    Ok(())
}

pub fn validate_enqueue_distill_job_request(req: &EnqueueDistillJobRequest) -> AppResult<()> {
    req.actor.validate()?;
    match &req.source {
        DistillSource::SessionTranscript {
            session_key,
            session_id,
        } => {
            validate_non_empty("source.sessionKey", session_key)?;
            if let Some(session_id) = session_id {
                validate_non_empty("source.sessionId", session_id)?;
            }
        }
        DistillSource::InlineMessages { messages } => {
            if messages.is_empty() {
                return Err(AppError::invalid_request(
                    "source.messages must be non-empty for inline-messages",
                ));
            }
            for item in messages {
                validate_non_empty("source.messages[].text", &item.text)?;
            }
        }
    }

    if matches!(
        req.options.persist_mode,
        DistillPersistMode::PersistMemoryRows
    ) && !matches!(req.mode, DistillMode::SessionLessons)
    {
        return Err(AppError::invalid_request(
            "persistMemoryRows is only allowed for mode=session-lessons",
        ));
    }

    validate_optional_positive("options.maxMessages", req.options.max_messages)?;
    validate_optional_positive("options.chunkChars", req.options.chunk_chars)?;
    validate_optional_positive(
        "options.chunkOverlapMessages",
        req.options.chunk_overlap_messages,
    )?;
    validate_optional_positive("options.maxArtifacts", req.options.max_artifacts)?;
    Ok(())
}

fn validate_optional_positive(field: &str, value: Option<u64>) -> AppResult<()> {
    if let Some(value) = value {
        validate_limit(field, value)?;
    }
    Ok(())
}
