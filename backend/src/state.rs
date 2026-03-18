use crate::{
    config::AppConfig,
    error::{AppError, AppResult},
    models::{
        clamped_limit, validate_non_empty, Actor, Category, DeleteRequest, DistillArtifact,
        DistillArtifactEvidence, DistillArtifactKind, DistillArtifactPersistence,
        DistillArtifactSubtype, DistillJobResultSummary, DistillJobStatus,
        DistillJobStatusResponse, DistillMode, DistillPersistMode, DistillSource,
        DistillSourceKind, EnqueueDistillJobRequest, EnqueueDistillJobResponse, ListRequest,
        ListResponse, ListRow, MemoryAction, MemoryMutationResult, MessageRole, Principal,
        RecallGenericRequest, RecallGenericResponse, RecallGenericRow, RecallReflectionRequest,
        RecallReflectionResponse, ReflectionKind, ReflectionMetadata, ReflectionRecallMode,
        RetrievalTrace, RetrievalTraceKind, RetrievalTraceQuery, RetrievalTraceStage,
        RetrievalTraceStageStatus, RowMetadata, StatsResponse, StoreRequest, StoreResponse,
        ToolStoreMemory, UpdateRequest, UpdateResponse, DEFAULT_IMPORTANCE,
    },
};
use arrow_array::{
    types::Float32Type, Array, ArrayRef, FixedSizeListArray, Float32Array, Float64Array,
    Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{ArrowError, DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::{
    connect,
    index::{scalar::FullTextSearchQuery, Index, IndexType},
    query::{ExecutableQuery, QueryBase},
    Connection as LanceConnection, DistanceType, Error as LanceError, Table as LanceTable,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const MEMORY_TABLE_NAME: &str = "memories_v1";
const DEFAULT_EMBEDDINGS_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_RERANK_ENDPOINT: &str = "https://api.jina.ai/v1/rerank";
const DEFAULT_RERANK_MODEL: &str = "jina-reranker-v3";
const MAX_CHUNK_RECOVERY_DEPTH: usize = 2;
const MAX_EMBEDDING_RECOVERY_CHUNKS: usize = 256;
const ACCESS_DECAY_HALF_LIFE_DAYS: f64 = 30.0;
const MAX_ACCESS_COUNT: i64 = 10_000;
const ACCESS_UPDATE_MAX_ROWS: usize = 64;
const DISTILL_MAX_QUOTE_LEN: usize = 160;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub memory_repo: Arc<LanceMemoryRepo>,
    pub job_store: JobStore,
    pub idempotency_store: IdempotencyStore,
}

impl AppState {
    pub fn new(config: AppConfig) -> anyhow::Result<Self> {
        let job_store = JobStore::new(config.storage.sqlite_path.clone())?;
        let idempotency_store = IdempotencyStore::new(config.storage.sqlite_path.clone())?;
        let memory_repo = Arc::new(LanceMemoryRepo::new(
            config.storage.lancedb_path.clone(),
            &config,
        )?);
        Ok(Self {
            config,
            memory_repo,
            job_store,
            idempotency_store,
        })
    }

    pub async fn execute_distill_job(
        &self,
        job_id: String,
        req: EnqueueDistillJobRequest,
    ) -> AppResult<()> {
        self.job_store.mark_distill_running(&job_id)?;
        match self.run_distill_job(&job_id, &req).await {
            Ok(summary) => self.job_store.complete_distill(&job_id, &summary),
            Err(error) => self.job_store.fail_distill(&job_id, &error),
        }
    }

    async fn run_distill_job(
        &self,
        job_id: &str,
        req: &EnqueueDistillJobRequest,
    ) -> AppResult<DistillJobResultSummary> {
        let prepared = match &req.source {
            DistillSource::InlineMessages { messages } => prepare_inline_distill_messages(messages),
            DistillSource::SessionTranscript {
                session_key,
                session_id,
            } => prepare_stored_session_transcript_messages(
                &self.job_store.load_session_transcript(
                    &req.actor.principal(),
                    session_key,
                    session_id.as_deref(),
                    req.options.max_messages,
                )?,
            ),
        };

        let max_artifacts = req.options.max_artifacts.unwrap_or(20).min(50) as usize;
        let mut warnings = Vec::new();
        if prepared.is_empty() {
            warnings.push("all transcript messages were filtered as noise".to_string());
        }

        let mut artifacts =
            build_distill_artifacts(job_id, &prepared, req.mode, &req.options, max_artifacts);
        let mut persisted_memory_count = 0u64;

        if let DistillPersistMode::PersistMemoryRows = req.options.persist_mode {
            for artifact in &mut artifacts {
                let response = self
                    .memory_repo
                    .store(StoreRequest::ToolStore {
                        actor: req.actor.clone(),
                        memory: ToolStoreMemory {
                            text: artifact.text.clone(),
                            category: Some(artifact.category),
                            importance: Some(artifact.importance),
                        },
                    })
                    .await?;
                let persisted_ids: Vec<String> =
                    response.results.into_iter().map(|row| row.id).collect();
                persisted_memory_count += persisted_ids.len() as u64;
                artifact.persistence = Some(DistillArtifactPersistence {
                    persist_mode: DistillPersistMode::PersistMemoryRows,
                    persisted_memory_ids: persisted_ids,
                });
            }
        }

        self.job_store
            .insert_distill_artifacts(job_id, &artifacts)?;

        Ok(DistillJobResultSummary {
            artifact_count: artifacts.len() as u64,
            persisted_memory_count,
            warnings,
        })
    }
}

#[derive(Clone)]
pub struct LanceMemoryRepo {
    db_path: PathBuf,
    generic_recall_engine: GenericRecallEngine,
    vector_dimensions: usize,
}

#[derive(Clone)]
struct MemoryRow {
    id: String,
    principal_user_id: String,
    principal_agent_id: String,
    text: String,
    category: Category,
    importance: f64,
    scope: String,
    created_at: i64,
    updated_at: i64,
    access_count: i64,
    last_accessed_at: i64,
    reflection_kind: Option<ReflectionKind>,
    strict_key: Option<String>,
    vector: Option<Vec<f32>>,
}

#[derive(Clone)]
struct GenericRecallEngine {
    settings: GenericRecallSettings,
    embedder: EmbeddingProviderClient,
    reranker: RerankProviderClient,
}

#[derive(Clone, Copy)]
struct GenericRecallSettings {
    candidate_pool_size: usize,
    vector_weight: f64,
    bm25_weight: f64,
    min_score: f64,
    hard_min_score: f64,
    recency_half_life_days: f64,
    recency_weight: f64,
    length_norm_anchor: usize,
    time_decay_half_life_days: f64,
    reinforcement_factor: f64,
    max_half_life_multiplier: f64,
    mmr_diversity: bool,
    mmr_similarity_threshold: f64,
    query_expansion: bool,
    filter_noise: bool,
    diagnostics: bool,
    rerank_mode: RerankMode,
    rerank_blend: f64,
}

#[derive(Clone)]
struct HashingEmbedder {
    dimensions: usize,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RerankMode {
    None,
    Lightweight,
    CrossEncoder,
}

#[derive(Clone, Copy)]
enum EmbeddingPurpose {
    Query,
    Passage,
}

#[derive(Clone)]
enum EmbeddingProviderClient {
    Hashing(HashingEmbedder),
    OpenAiCompatible(OpenAiCompatibleEmbedder),
}

#[derive(Clone)]
struct OpenAiCompatibleEmbedder {
    client: reqwest::Client,
    endpoint: String,
    model: String,
    task_query: Option<String>,
    task_passage: Option<String>,
    normalized: Option<bool>,
    allow_extended_tuning_fields: bool,
    api_keys: Vec<String>,
    next_api_key: Arc<AtomicUsize>,
    dimensions: usize,
    cache: Arc<parking_lot::Mutex<EmbeddingCache>>,
}

#[derive(Clone)]
struct RerankProviderClient {
    client: reqwest::Client,
    provider: String,
    endpoint: String,
    model: String,
    api_keys: Vec<String>,
    next_api_key: Arc<AtomicUsize>,
}

#[derive(Clone, Debug)]
struct EmbeddingCache {
    entries: HashMap<String, EmbeddingCacheEntry>,
    order: VecDeque<String>,
    capacity: usize,
    ttl_ms: u64,
}

#[derive(Clone, Debug)]
struct EmbeddingCacheEntry {
    vector: Vec<f32>,
    cached_at: i64,
}

#[derive(Clone, Copy, Debug)]
struct TableCapabilities {
    has_vector_column: bool,
    vector_dimensions: Option<usize>,
    has_access_metadata_columns: bool,
}

#[derive(Clone)]
struct ScoredCandidate {
    row: MemoryRow,
    normalized_text: String,
    token_counts: HashMap<String, usize>,
    token_len: usize,
    vector_score: f64,
    bm25_score: f64,
    score: f64,
}

#[derive(Clone)]
struct CandidateSeed {
    row: MemoryRow,
    vector_score: Option<f64>,
    bm25_score: Option<f64>,
}

#[derive(Clone)]
struct RankedMemoryRow {
    row: MemoryRow,
    score: f64,
}

#[derive(Default)]
struct RetrievalTraceCollector {
    stages: Vec<RetrievalTraceStage>,
    final_row_ids: Vec<String>,
}

#[derive(Default)]
struct RerankTraceSummary {
    stage: Option<RetrievalTraceStage>,
}

#[derive(Clone)]
struct DistillPreparedMessage {
    message_id: u64,
    role: MessageRole,
    text: String,
}

#[derive(Clone)]
struct SessionTranscriptStoredMessage {
    message_id: u64,
    role: MessageRole,
    text: String,
}

#[derive(Clone)]
struct DistillCandidate {
    kind: DistillArtifactKind,
    subtype: Option<DistillArtifactSubtype>,
    category: Category,
    importance: f64,
    text: String,
    evidence: Vec<DistillArtifactEvidence>,
    tags: Vec<String>,
    score: f64,
    dedupe_key: String,
}

impl RetrievalTraceCollector {
    fn push(&mut self, stage: RetrievalTraceStage) {
        self.stages.push(stage);
    }

    fn set_final_rows(&mut self, rows: &[RankedMemoryRow]) {
        self.final_row_ids = rows.iter().map(|row| row.row.id.clone()).collect();
    }

    fn finish(
        self,
        kind: RetrievalTraceKind,
        query: &str,
        lexical_query: &str,
        mode: Option<String>,
    ) -> RetrievalTrace {
        RetrievalTrace {
            kind,
            query: RetrievalTraceQuery {
                preview: truncate_for_error(query, 160),
                raw_len: query.chars().count(),
                lexical_preview: truncate_for_error(lexical_query, 160),
                lexical_len: lexical_query.chars().count(),
            },
            mode,
            stages: self.stages,
            final_row_ids: self.final_row_ids,
        }
    }
}

impl GenericRecallEngine {
    fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let rerank_mode =
            if !config.providers.rerank.enabled || config.providers.rerank.mode.trim() == "none" {
                RerankMode::None
            } else if config.providers.rerank.mode.trim() == "cross-encoder" {
                RerankMode::CrossEncoder
            } else {
                RerankMode::Lightweight
            };
        let settings = GenericRecallSettings {
            candidate_pool_size: config.retrieval.candidate_pool_size,
            vector_weight: config.retrieval.vector_weight,
            bm25_weight: config.retrieval.bm25_weight,
            min_score: config.retrieval.min_score,
            hard_min_score: config.retrieval.hard_min_score,
            recency_half_life_days: config.retrieval.recency_half_life_days,
            recency_weight: config.retrieval.recency_weight,
            length_norm_anchor: config.retrieval.length_norm_anchor,
            time_decay_half_life_days: config.retrieval.time_decay_half_life_days,
            reinforcement_factor: config.retrieval.reinforcement_factor,
            max_half_life_multiplier: config.retrieval.max_half_life_multiplier,
            mmr_diversity: config.retrieval.mmr_diversity,
            mmr_similarity_threshold: config.retrieval.mmr_similarity_threshold,
            query_expansion: config.retrieval.query_expansion,
            filter_noise: config.retrieval.filter_noise,
            diagnostics: config.retrieval.diagnostics,
            rerank_mode,
            rerank_blend: config.providers.rerank.blend,
        };
        let embedder = EmbeddingProviderClient::from_config(config)?;
        let reranker = RerankProviderClient::from_config(config)?;
        Ok(Self {
            settings,
            embedder,
            reranker,
        })
    }

    fn vector_dimensions(&self) -> usize {
        self.embedder.dimensions()
    }

    fn candidate_pool_size(&self, limit: usize) -> usize {
        self.settings
            .candidate_pool_size
            .max(limit.saturating_mul(4))
            .max(limit.max(1))
    }

    fn lexical_query(&self, query: &str) -> String {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        if !self.settings.query_expansion {
            return trimmed.to_string();
        }
        expand_query_terms(trimmed)
    }

    async fn embed_query(&self, text: &str) -> AppResult<Vec<f32>> {
        self.embedder.embed_query(text).await
    }

    async fn embed_passage(&self, text: &str) -> AppResult<Vec<f32>> {
        self.embedder.embed_passage(text).await
    }

    async fn embed_passages_batch(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        self.embedder.embed_passages_batch(texts).await
    }

    async fn rank_candidates(
        &self,
        query: &str,
        lexical_query: &str,
        query_embedding: &[f32],
        seeds: Vec<CandidateSeed>,
        limit: usize,
        mut trace: Option<&mut RetrievalTraceCollector>,
    ) -> AppResult<Vec<RankedMemoryRow>> {
        if seeds.is_empty() || limit == 0 {
            if let Some(trace) = trace {
                trace.push(make_trace_stage(
                    "rank.finalize",
                    RetrievalTraceStageStatus::Skipped,
                ));
            }
            return Ok(Vec::new());
        }

        let normalized_query = normalize_recall_text(lexical_query);
        let query_tokens_raw = lexical_tokens(&normalized_query);
        let query_tokens = if query_tokens_raw.is_empty() && !normalized_query.is_empty() {
            vec![normalized_query.clone()]
        } else {
            query_tokens_raw
        };
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }
        let query_token_counts = token_count_map(&query_tokens);
        let mut candidates = Vec::with_capacity(seeds.len());
        for seed in seeds {
            let normalized_text = normalize_recall_text(&seed.row.text);
            let doc_tokens = lexical_tokens(&normalized_text);
            let token_counts = token_count_map(&doc_tokens);
            let fallback_vector_score = seed
                .row
                .vector
                .as_ref()
                .map(|vector| {
                    clamp_score(
                        (cosine_similarity_f32(query_embedding, vector).clamp(-1.0, 1.0) + 1.0)
                            / 2.0,
                    )
                })
                .unwrap_or(0.0);
            candidates.push(ScoredCandidate {
                row: seed.row,
                normalized_text,
                token_counts,
                token_len: doc_tokens.len().max(1),
                vector_score: seed.vector_score.unwrap_or(0.0).max(fallback_vector_score),
                bm25_score: seed.bm25_score.unwrap_or(0.0),
                score: 0.0,
            });
        }

        let unique_query_tokens = unique_tokens(&query_tokens);
        let avg_doc_len = candidates
            .iter()
            .map(|candidate| candidate.token_len as f64)
            .sum::<f64>()
            / (candidates.len().max(1) as f64);
        let doc_frequency = query_doc_frequency(&candidates, &unique_query_tokens);
        let total_docs = candidates.len();

        for candidate in &mut candidates {
            let lexical_score = bm25_like_score(
                &query_token_counts,
                &normalized_query,
                candidate,
                total_docs,
                avg_doc_len,
                &doc_frequency,
            );
            candidate.bm25_score = candidate.bm25_score.max(lexical_score);
        }

        let candidate_pool_size = self.candidate_pool_size(limit).min(candidates.len());
        let vector_ranked = ranked_indices_by(&candidates, |candidate| candidate.vector_score);
        let bm25_ranked = ranked_indices_by(&candidates, |candidate| candidate.bm25_score);

        let mut selected = HashSet::new();
        for idx in vector_ranked.iter().take(candidate_pool_size) {
            if candidates[*idx].vector_score > 0.0 {
                selected.insert(*idx);
            }
        }
        for idx in bm25_ranked.iter().take(candidate_pool_size) {
            if candidates[*idx].bm25_score > 0.0 {
                selected.insert(*idx);
            }
        }
        if selected.is_empty() {
            for idx in vector_ranked.iter().take(candidate_pool_size) {
                selected.insert(*idx);
            }
        }

        let mut selected_indices: Vec<usize> = selected.into_iter().collect();
        selected_indices.sort_unstable();

        let weight_sum =
            (self.settings.vector_weight + self.settings.bm25_weight).max(f64::EPSILON);
        let mut rerank_candidates = Vec::new();
        let mut dropped_below_min_score = 0_usize;
        let now = now_millis();

        for idx in selected_indices {
            let candidate = candidates[idx].clone();
            let has_vector = candidate.vector_score > 0.0;
            let has_bm25 = candidate.bm25_score > 0.0;

            let mut score = if has_vector && has_bm25 {
                (candidate.vector_score * self.settings.vector_weight
                    + candidate.bm25_score * self.settings.bm25_weight)
                    / weight_sum
            } else if has_vector {
                candidate.vector_score * (0.85 + 0.15 * (self.settings.vector_weight / weight_sum))
            } else {
                candidate.bm25_score * (0.72 + 0.28 * (self.settings.bm25_weight / weight_sum))
            };
            if has_vector && has_bm25 {
                score += 0.08 * candidate.vector_score.min(candidate.bm25_score);
            }
            score = clamp_score(score);
            if score < self.settings.min_score {
                dropped_below_min_score += 1;
                continue;
            }

            let mut candidate = candidate;
            candidate.score = score;
            rerank_candidates.push(candidate);
        }
        let selected_for_rerank = rerank_candidates.len();

        if let Some(trace) = trace.as_deref_mut() {
            let mut stage = make_trace_stage("rank.pre-rerank", RetrievalTraceStageStatus::Ok);
            stage.input_count = Some(candidates.len() as u64);
            stage.output_count = Some(selected_for_rerank as u64);
            stage
                .metrics
                .insert("candidatePoolSize".to_string(), json!(candidate_pool_size));
            stage.metrics.insert(
                "droppedBelowMinScoreCount".to_string(),
                json!(dropped_below_min_score),
            );
            stage.metrics.insert("limit".to_string(), json!(limit));
            trace.push(stage);
        }

        if rerank_candidates.is_empty() {
            if let Some(trace) = trace {
                let mut stage =
                    make_trace_stage("rank.finalize", RetrievalTraceStageStatus::Skipped);
                stage.input_count = Some(candidates.len() as u64);
                stage.output_count = Some(0);
                stage.reason = Some("no candidates survived pre-rerank scoring".to_string());
                trace.push(stage);
            }
            return Ok(Vec::new());
        }

        let rerank_summary = self
            .reranker
            .apply(
                query,
                &normalized_query,
                &unique_query_tokens,
                &mut rerank_candidates,
                self.settings.rerank_blend,
                self.settings.rerank_mode,
                self.settings.diagnostics,
                selected_for_rerank,
            )
            .await?;
        if let Some(trace) = trace.as_deref_mut() {
            if let Some(stage) = rerank_summary.stage {
                trace.push(stage);
            }
        }

        let mut ranked_rows = Vec::new();
        let mut noise_filtered = 0_usize;
        for mut candidate in rerank_candidates {
            let mut score = candidate.score;

            let freshness_ts = candidate.row.updated_at.max(candidate.row.created_at);
            let age_days = ((now - freshness_ts).max(0) as f64) / 86_400_000.0;
            let recency_boost = (-age_days / self.settings.recency_half_life_days).exp()
                * self.settings.recency_weight;
            score = clamp_score(score + recency_boost);

            let importance = candidate.row.importance.clamp(0.0, 1.0);
            score = clamp_score(score * (0.7 + 0.3 * importance));

            let char_len = candidate.row.text.chars().count().max(1) as f64;
            let length_ratio = (char_len / self.settings.length_norm_anchor as f64).max(1.0);
            let length_factor = 1.0 / (1.0 + 0.5 * length_ratio.log2());
            score = clamp_score(score * length_factor);

            let effective_half_life_days = compute_effective_half_life_days(
                self.settings.time_decay_half_life_days,
                candidate.row.access_count,
                candidate.row.last_accessed_at,
                self.settings.reinforcement_factor,
                self.settings.max_half_life_multiplier,
                now,
            );
            let decay_factor = 0.5 + 0.5 * (-age_days / effective_half_life_days).exp();
            score = clamp_score(score * decay_factor);

            if score < self.settings.hard_min_score {
                continue;
            }

            if self.settings.filter_noise && is_noise_memory_text(&candidate.row.text) {
                noise_filtered += 1;
                continue;
            }

            candidate.score = score;
            ranked_rows.push(RankedMemoryRow {
                row: candidate.row,
                score: round_score(score),
            });
        }

        ranked_rows.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| b.row.updated_at.cmp(&a.row.updated_at))
                .then_with(|| a.row.id.cmp(&b.row.id))
        });
        let mut diversity_deferred = 0_usize;
        if self.settings.mmr_diversity {
            let (diversified, deferred) =
                apply_mmr_diversity(ranked_rows, self.settings.mmr_similarity_threshold);
            ranked_rows = diversified;
            diversity_deferred = deferred;
        }
        ranked_rows.truncate(limit);
        emit_internal_diagnostic(
            self.settings.diagnostics,
            json!({
                "event": "retrieval.rank.summary",
                "stage": "finalize",
                "queryLen": query.chars().count(),
                "lexicalQueryLen": lexical_query.chars().count(),
                "seedCount": candidates.len(),
                "selectedCount": selected_for_rerank,
                "noiseFilteredCount": noise_filtered,
                "mmrEnabled": self.settings.mmr_diversity,
                "mmrSimilarityThreshold": self.settings.mmr_similarity_threshold,
                "mmrDeferredCount": diversity_deferred,
                "resultCount": ranked_rows.len(),
            }),
        );
        if let Some(trace) = trace {
            let mut stage = make_trace_stage("rank.finalize", RetrievalTraceStageStatus::Ok);
            stage.input_count = Some(selected_for_rerank as u64);
            stage.output_count = Some(ranked_rows.len() as u64);
            stage
                .metrics
                .insert("noiseFilteredCount".to_string(), json!(noise_filtered));
            stage
                .metrics
                .insert("mmrEnabled".to_string(), json!(self.settings.mmr_diversity));
            stage
                .metrics
                .insert("mmrDeferredCount".to_string(), json!(diversity_deferred));
            stage.metrics.insert(
                "hardMinScore".to_string(),
                json!(self.settings.hard_min_score),
            );
            trace.set_final_rows(&ranked_rows);
            trace.push(stage);
        }
        Ok(ranked_rows)
    }
}

impl EmbeddingProviderClient {
    fn from_config(config: &AppConfig) -> anyhow::Result<Self> {
        let provider = config.providers.embedding.provider.trim();
        if provider == "hashing" {
            return Ok(Self::Hashing(HashingEmbedder::new(
                config.providers.embedding.dimensions,
            )?));
        }
        Ok(Self::OpenAiCompatible(OpenAiCompatibleEmbedder::new(
            config,
        )?))
    }

    fn dimensions(&self) -> usize {
        match self {
            Self::Hashing(embedder) => embedder.dimensions,
            Self::OpenAiCompatible(embedder) => embedder.dimensions,
        }
    }

    async fn embed_query(&self, text: &str) -> AppResult<Vec<f32>> {
        match self {
            Self::Hashing(embedder) => embedder.embed(text),
            Self::OpenAiCompatible(embedder) => embedder.embed_query(text).await,
        }
    }

    async fn embed_passage(&self, text: &str) -> AppResult<Vec<f32>> {
        match self {
            Self::Hashing(embedder) => embedder.embed(text),
            Self::OpenAiCompatible(embedder) => embedder.embed_passage(text).await,
        }
    }

    async fn embed_passages_batch(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        match self {
            Self::Hashing(embedder) => {
                let mut out = Vec::with_capacity(texts.len());
                for text in texts {
                    out.push(embedder.embed(text)?);
                }
                Ok(out)
            }
            Self::OpenAiCompatible(embedder) => embedder.embed_passages_batch(texts).await,
        }
    }
}

impl OpenAiCompatibleEmbedder {
    fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let base_url = config
            .providers
            .embedding
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_EMBEDDINGS_BASE_URL.to_string());
        let endpoint = normalize_embeddings_endpoint(&base_url);
        let raw_api_key = resolve_secret(config.providers.embedding.api_key.as_deref())?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.providers.embedding.timeout_ms))
            .build()
            .map_err(|err| anyhow::anyhow!("failed to build embedding HTTP client: {err}"))?;
        let api_keys = parse_api_keys(raw_api_key.as_deref());
        let cache = EmbeddingCache::new(
            config.providers.embedding.cache_max_entries,
            config.providers.embedding.cache_ttl_ms,
        );
        let task_query = trim_optional_string(config.providers.embedding.task_query.as_deref());
        let task_passage = trim_optional_string(config.providers.embedding.task_passage.as_deref());
        let normalized = config.providers.embedding.normalized;
        let allow_extended_tuning_fields = supports_embedding_tuning_fields(
            &endpoint,
            &config.providers.embedding.model,
            &config.providers.embedding.api,
        );
        Ok(Self {
            client,
            endpoint,
            model: config.providers.embedding.model.clone(),
            task_query,
            task_passage,
            normalized,
            allow_extended_tuning_fields,
            api_keys,
            next_api_key: Arc::new(AtomicUsize::new(0)),
            dimensions: config.providers.embedding.dimensions,
            cache: Arc::new(parking_lot::Mutex::new(cache)),
        })
    }

    async fn embed_query(&self, text: &str) -> AppResult<Vec<f32>> {
        let mut vectors = self
            .embed_many_with_purpose(&[text.to_string()], EmbeddingPurpose::Query)
            .await?;
        Ok(vectors.pop().unwrap_or_default())
    }

    async fn embed_passage(&self, text: &str) -> AppResult<Vec<f32>> {
        let mut vectors = self
            .embed_many_with_purpose(&[text.to_string()], EmbeddingPurpose::Passage)
            .await?;
        Ok(vectors.pop().unwrap_or_default())
    }

    async fn embed_passages_batch(&self, texts: &[String]) -> AppResult<Vec<Vec<f32>>> {
        self.embed_many_with_purpose(texts, EmbeddingPurpose::Passage)
            .await
    }

    async fn embed_many_with_purpose(
        &self,
        texts: &[String],
        purpose: EmbeddingPurpose,
    ) -> AppResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let now = now_millis();
        let mut resolved = vec![Vec::<f32>::new(); texts.len()];
        let mut missing_keys = Vec::new();
        let mut missing_key_index = HashMap::new();
        let mut missing_positions = Vec::new();
        {
            let mut cache = self.cache.lock();
            for (idx, text) in texts.iter().enumerate() {
                let key = self.cache_key(text, purpose);
                if let Some(vector) = cache.get(&key, now) {
                    resolved[idx] = vector;
                    continue;
                }
                let slot = if let Some(slot) = missing_key_index.get(&key).copied() {
                    slot
                } else {
                    let slot = missing_keys.len();
                    missing_keys.push((key.clone(), text.clone()));
                    missing_key_index.insert(key, slot);
                    slot
                };
                missing_positions.push((idx, slot));
            }
        }

        if !missing_keys.is_empty() {
            let request_texts: Vec<String> =
                missing_keys.iter().map(|(_, text)| text.clone()).collect();
            let vectors = match self
                .request_embeddings_with_failover(&request_texts, purpose)
                .await
            {
                Ok(vectors) => vectors,
                Err(message) if is_embedding_context_limit_error(&message) => {
                    self.request_embeddings_with_chunk_recovery(&request_texts, &message, purpose)
                        .await?
                }
                Err(message) => return Err(AppError::upstream_embedding(message)),
            };
            if vectors.len() != request_texts.len() {
                return Err(AppError::upstream_embedding(format!(
                    "embedding provider returned {} vectors for {} inputs",
                    vectors.len(),
                    request_texts.len()
                )));
            }
            let mut cache = self.cache.lock();
            for ((key, _), vector) in missing_keys.iter().zip(vectors.iter()) {
                cache.put(key.clone(), vector.clone(), now);
            }
            for (target_idx, source_idx) in missing_positions {
                resolved[target_idx] = vectors[source_idx].clone();
            }
        }

        Ok(resolved)
    }

    fn embedding_task_for(&self, purpose: EmbeddingPurpose) -> Option<&str> {
        match purpose {
            EmbeddingPurpose::Query => self.task_query.as_deref(),
            EmbeddingPurpose::Passage => self.task_passage.as_deref(),
        }
    }

    fn cache_key(&self, text: &str, purpose: EmbeddingPurpose) -> String {
        let digest = stable_hash64(0xA5A5_5A5A_1337_4242, text.as_bytes());
        let effective_task = if self.allow_extended_tuning_fields {
            self.embedding_task_for(purpose)
        } else {
            None
        };
        let task_marker = effective_task.unwrap_or("-");
        let normalized_marker = if self.allow_extended_tuning_fields {
            match self.normalized {
                Some(true) => "n1",
                Some(false) => "n0",
                None => "n-",
            }
        } else {
            "n-"
        };
        format!(
            "{}:{}:{}:{}:{digest:016x}",
            self.model, self.dimensions, task_marker, normalized_marker
        )
    }

    fn api_key_attempt_order(&self) -> Vec<Option<String>> {
        if self.api_keys.is_empty() {
            return vec![None];
        }
        let key_count = self.api_keys.len();
        let start = self.next_api_key.fetch_add(1, Ordering::Relaxed) % key_count;
        (0..key_count)
            .map(|offset| Some(self.api_keys[(start + offset) % key_count].clone()))
            .collect()
    }

    async fn request_embeddings_with_failover(
        &self,
        texts: &[String],
        purpose: EmbeddingPurpose,
    ) -> Result<Vec<Vec<f32>>, String> {
        let mut last_error: Option<String> = None;
        let attempts = self.api_key_attempt_order();
        for (attempt_idx, api_key) in attempts.iter().enumerate() {
            match self
                .request_embeddings_once(texts, api_key.as_deref(), purpose)
                .await
            {
                Ok(vectors) => return Ok(vectors),
                Err((retryable, message)) => {
                    last_error = Some(message.clone());
                    if retryable && attempt_idx + 1 < attempts.len() {
                        continue;
                    }
                    return Err(message);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            "embedding provider request failed across all configured credentials".to_string()
        }))
    }

    async fn request_embeddings_with_chunk_recovery(
        &self,
        texts: &[String],
        root_error: &str,
        purpose: EmbeddingPurpose,
    ) -> AppResult<Vec<Vec<f32>>> {
        let mut recovered = Vec::with_capacity(texts.len());
        for text in texts {
            recovered.push(
                self.embed_text_with_chunk_recovery(text, root_error, purpose)
                    .await?,
            );
        }
        Ok(recovered)
    }

    async fn embed_text_with_chunk_recovery(
        &self,
        text: &str,
        root_error: &str,
        purpose: EmbeddingPurpose,
    ) -> AppResult<Vec<f32>> {
        let chunks = smart_chunk_text(text, &self.model);
        if chunks.len() <= 1 {
            return Err(AppError::upstream_embedding(root_error.to_string()));
        }
        if chunks.len() > MAX_EMBEDDING_RECOVERY_CHUNKS {
            return Err(AppError::upstream_embedding(format!(
                "embedding context recovery generated too many chunks: {}",
                chunks.len()
            )));
        }

        let vectors = self
            .embed_chunks_with_recursive_rechunk(chunks, 0, purpose)
            .await
            .map_err(AppError::upstream_embedding)?;
        average_embeddings(&vectors, self.dimensions).map_err(AppError::upstream_embedding)
    }

    async fn embed_chunks_with_recursive_rechunk(
        &self,
        chunks: Vec<String>,
        depth: usize,
        purpose: EmbeddingPurpose,
    ) -> Result<Vec<Vec<f32>>, String> {
        match self
            .request_embeddings_with_failover(&chunks, purpose)
            .await
        {
            Ok(vectors) => Ok(vectors),
            Err(message)
                if depth < MAX_CHUNK_RECOVERY_DEPTH
                    && is_embedding_context_limit_error(&message) =>
            {
                let mut expanded = Vec::new();
                let mut split_applied = false;
                for chunk in chunks {
                    let nested = smart_chunk_text(&chunk, &self.model);
                    if nested.len() > 1 {
                        split_applied = true;
                        expanded.extend(nested);
                    } else {
                        expanded.push(chunk);
                    }
                }
                if !split_applied {
                    return Err(message);
                }
                if expanded.len() > MAX_EMBEDDING_RECOVERY_CHUNKS {
                    return Err(format!(
                        "embedding context recovery exceeded max chunk count ({MAX_EMBEDDING_RECOVERY_CHUNKS})"
                    ));
                }
                Box::pin(self.embed_chunks_with_recursive_rechunk(expanded, depth + 1, purpose))
                    .await
            }
            Err(message) => Err(message),
        }
    }

    async fn request_embeddings_once(
        &self,
        texts: &[String],
        api_key: Option<&str>,
        purpose: EmbeddingPurpose,
    ) -> Result<Vec<Vec<f32>>, (bool, String)> {
        let input = if texts.len() == 1 {
            Value::String(texts[0].clone())
        } else {
            Value::Array(texts.iter().cloned().map(Value::String).collect())
        };
        let mut payload = serde_json::json!({
            "model": self.model,
            "input": input,
            "encoding_format": "float"
        });
        payload["dimensions"] = serde_json::json!(self.dimensions);
        if self.allow_extended_tuning_fields {
            if let Some(task) = self.embedding_task_for(purpose) {
                payload["task"] = serde_json::json!(task);
            }
            if let Some(normalized) = self.normalized {
                payload["normalized"] = serde_json::json!(normalized);
            }
        }

        let mut request = self
            .client
            .post(&self.endpoint)
            .header(CONTENT_TYPE, "application/json");
        if let Some(api_key) = api_key {
            let trimmed = api_key.trim();
            if !trimmed.is_empty() {
                request = request.header(AUTHORIZATION, format!("Bearer {trimmed}"));
            }
        }

        let response = request
            .json(&payload)
            .send()
            .await
            .map_err(|err| (true, format!("embedding provider request failed: {err}")))?;
        let status = response.status();
        let body = response.text().await.map_err(|err| {
            (
                true,
                format!("failed to read embedding provider response: {err}"),
            )
        })?;
        if !status.is_success() {
            let retryable = is_embedding_failover_retryable(status.as_u16());
            return Err((
                retryable,
                format!(
                    "embedding provider returned status {}: {}",
                    status.as_u16(),
                    truncate_for_error(&body, 240),
                ),
            ));
        }

        let value: Value = serde_json::from_str(&body).map_err(|err| {
            (
                false,
                format!("invalid embedding provider JSON response: {err}"),
            )
        })?;
        let data = value
            .get("data")
            .and_then(|rows| rows.as_array())
            .ok_or_else(|| {
                (
                    false,
                    "embedding provider response missing data[]".to_string(),
                )
            })?;
        if data.len() != texts.len() {
            return Err((
                false,
                format!(
                    "embedding provider returned {} vectors for {} inputs",
                    data.len(),
                    texts.len()
                ),
            ));
        }

        let mut vectors = Vec::with_capacity(data.len());
        for item in data {
            let values = item
                .get("embedding")
                .and_then(|embedding| embedding.as_array())
                .ok_or_else(|| {
                    (
                        false,
                        "embedding provider response missing embedding[]".to_string(),
                    )
                })?;
            let mut vector = Vec::with_capacity(values.len());
            for value in values {
                let component = value.as_f64().ok_or_else(|| {
                    (
                        false,
                        "embedding provider returned non-numeric embedding value".to_string(),
                    )
                })?;
                vector.push(component as f32);
            }
            if vector.len() != self.dimensions {
                return Err((
                    false,
                    format!(
                        "embedding dimension mismatch: expected {}, got {}",
                        self.dimensions,
                        vector.len()
                    ),
                ));
            }
            vectors.push(vector);
        }
        Ok(vectors)
    }
}

impl RerankProviderClient {
    fn from_config(config: &AppConfig) -> anyhow::Result<Self> {
        let endpoint = config
            .providers
            .rerank
            .base_url
            .clone()
            .unwrap_or_else(|| DEFAULT_RERANK_ENDPOINT.to_string());
        let model = config
            .providers
            .rerank
            .model
            .clone()
            .unwrap_or_else(|| DEFAULT_RERANK_MODEL.to_string());
        let raw_api_key = resolve_secret(config.providers.rerank.api_key.as_deref())?;
        let api_keys = parse_api_keys(raw_api_key.as_deref());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.providers.rerank.timeout_ms))
            .build()
            .map_err(|err| anyhow::anyhow!("failed to build rerank HTTP client: {err}"))?;
        Ok(Self {
            client,
            provider: config.providers.rerank.provider.trim().to_string(),
            endpoint,
            model,
            api_keys,
            next_api_key: Arc::new(AtomicUsize::new(0)),
        })
    }

    async fn apply(
        &self,
        query: &str,
        normalized_query: &str,
        query_tokens: &[String],
        candidates: &mut [ScoredCandidate],
        blend: f64,
        mode: RerankMode,
        diagnostics: bool,
        candidate_count: usize,
    ) -> AppResult<RerankTraceSummary> {
        if candidates.is_empty() || mode == RerankMode::None {
            let mut stage = make_trace_stage("rerank", RetrievalTraceStageStatus::Skipped);
            stage.input_count = Some(candidate_count as u64);
            stage.output_count = Some(candidates.len() as u64);
            stage.reason = Some("rerank disabled or no candidates available".to_string());
            return Ok(RerankTraceSummary { stage: Some(stage) });
        }

        if mode == RerankMode::CrossEncoder {
            match self
                .cross_encoder_rerank(query, candidates, blend, diagnostics)
                .await
            {
                Ok(stage) => return Ok(RerankTraceSummary { stage: Some(stage) }),
                Err(err) => {
                    emit_internal_diagnostic(
                        diagnostics,
                        json!({
                            "event": "retrieval.rerank.fallback",
                            "stage": "cross-encoder",
                            "fallback": "lightweight",
                            "reason": truncate_for_error(&format!("{err:?}"), 240),
                        }),
                    );
                    let mut stage = make_trace_stage("rerank", RetrievalTraceStageStatus::Fallback);
                    stage.input_count = Some(candidate_count as u64);
                    stage.output_count = Some(candidates.len() as u64);
                    stage.fallback_to = Some("lightweight".to_string());
                    stage.reason = Some(truncate_for_error(&format!("{err:?}"), 240));
                    stage
                        .metrics
                        .insert("requestedMode".to_string(), json!("cross-encoder"));
                    stage
                        .metrics
                        .insert("appliedMode".to_string(), json!("lightweight"));
                    self.lightweight_rerank(normalized_query, query_tokens, candidates, blend);
                    return Ok(RerankTraceSummary { stage: Some(stage) });
                }
            }
        }

        self.lightweight_rerank(normalized_query, query_tokens, candidates, blend);
        let mut stage = make_trace_stage("rerank", RetrievalTraceStageStatus::Ok);
        stage.input_count = Some(candidate_count as u64);
        stage.output_count = Some(candidates.len() as u64);
        stage
            .metrics
            .insert("requestedMode".to_string(), json!("lightweight"));
        stage
            .metrics
            .insert("appliedMode".to_string(), json!("lightweight"));
        Ok(RerankTraceSummary { stage: Some(stage) })
    }

    fn api_key_attempt_order(&self, needs_api_key: bool) -> Vec<Option<String>> {
        if self.api_keys.is_empty() {
            if needs_api_key {
                return Vec::new();
            }
            return vec![None];
        }
        let key_count = self.api_keys.len();
        let start = self.next_api_key.fetch_add(1, Ordering::Relaxed) % key_count;
        (0..key_count)
            .map(|offset| Some(self.api_keys[(start + offset) % key_count].clone()))
            .collect()
    }

    fn lightweight_rerank(
        &self,
        normalized_query: &str,
        query_tokens: &[String],
        candidates: &mut [ScoredCandidate],
        blend: f64,
    ) {
        let blend = blend.clamp(0.0, 1.0);
        for candidate in candidates.iter_mut() {
            let rerank_signal = lightweight_rerank_signal(
                query_tokens,
                normalized_query,
                &candidate.normalized_text,
                &candidate.token_counts,
            );
            candidate.score = clamp_score(candidate.score * (1.0 - blend) + rerank_signal * blend);
        }
        candidates.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| b.row.updated_at.cmp(&a.row.updated_at))
                .then_with(|| a.row.id.cmp(&b.row.id))
        });
    }

    async fn cross_encoder_rerank(
        &self,
        query: &str,
        candidates: &mut [ScoredCandidate],
        blend: f64,
        diagnostics: bool,
    ) -> AppResult<RetrievalTraceStage> {
        let needs_api_key = self.provider != "vllm";
        let attempts = self.api_key_attempt_order(needs_api_key);
        if needs_api_key && attempts.is_empty() {
            return Err(AppError::upstream_rerank(
                "rerank provider requires api_key for the configured provider",
            ));
        }

        let mut last_error: Option<String> = None;
        for (attempt_idx, api_key) in attempts.iter().enumerate() {
            match self
                .cross_encoder_rerank_once(query, candidates, blend, api_key.as_deref())
                .await
            {
                Ok(()) => {
                    let mut stage = make_trace_stage("rerank", RetrievalTraceStageStatus::Ok);
                    stage.input_count = Some(candidates.len() as u64);
                    stage.output_count = Some(candidates.len() as u64);
                    stage
                        .metrics
                        .insert("requestedMode".to_string(), json!("cross-encoder"));
                    stage
                        .metrics
                        .insert("appliedMode".to_string(), json!("cross-encoder"));
                    stage
                        .metrics
                        .insert("attemptCount".to_string(), json!(attempt_idx + 1));
                    return Ok(stage);
                }
                Err((retryable, message)) => {
                    last_error = Some(message.clone());
                    emit_internal_diagnostic(
                        diagnostics,
                        json!({
                            "event": "retrieval.rerank.provider-attempt",
                            "stage": "cross-encoder",
                            "attempt": attempt_idx + 1,
                            "attemptTotal": attempts.len(),
                            "retryable": retryable,
                            "usedApiKey": api_key.is_some(),
                            "reason": truncate_for_error(&message, 240),
                        }),
                    );
                    if retryable && attempt_idx + 1 < attempts.len() {
                        continue;
                    }
                    return Err(AppError::upstream_rerank(message));
                }
            }
        }

        Err(AppError::upstream_rerank(last_error.unwrap_or_else(|| {
            "rerank provider request failed across all configured credentials".to_string()
        })))
    }

    async fn cross_encoder_rerank_once(
        &self,
        query: &str,
        candidates: &mut [ScoredCandidate],
        blend: f64,
        api_key: Option<&str>,
    ) -> Result<(), (bool, String)> {
        let docs: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.row.text.clone())
            .collect();
        let top_n = docs.len();
        let (headers, body) = self
            .build_rerank_request(query, &docs, top_n, api_key)
            .map_err(|err| (false, format!("{err:?}")))?;
        let response = self
            .client
            .post(&self.endpoint)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(|err| (true, format!("rerank provider request failed: {err}")))?;
        let status = response.status();
        let body_text = response.text().await.map_err(|err| {
            (
                true,
                format!("failed to read rerank provider response: {err}"),
            )
        })?;
        if !status.is_success() {
            return Err((
                is_rerank_failover_retryable(status.as_u16()),
                format!(
                    "rerank provider returned status {}: {}",
                    status.as_u16(),
                    truncate_for_error(&body_text, 240),
                ),
            ));
        }

        let value: Value = serde_json::from_str(&body_text).map_err(|err| {
            (
                false,
                format!("invalid rerank provider JSON response: {err}"),
            )
        })?;
        let items =
            parse_rerank_items(&self.provider, &value).map_err(|message| (false, message))?;
        let mut returned = HashSet::new();
        for (idx, raw_score) in items {
            if idx >= candidates.len() {
                continue;
            }
            returned.insert(idx);
            let candidate = &mut candidates[idx];
            candidate.score = clamp_score(candidate.score * (1.0 - blend) + raw_score * blend);
        }
        for (idx, candidate) in candidates.iter_mut().enumerate() {
            if !returned.contains(&idx) {
                candidate.score = clamp_score(candidate.score * 0.8);
            }
        }
        candidates.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| b.row.updated_at.cmp(&a.row.updated_at))
                .then_with(|| a.row.id.cmp(&b.row.id))
        });
        Ok(())
    }

    fn build_rerank_request(
        &self,
        query: &str,
        docs: &[String],
        top_n: usize,
        api_key: Option<&str>,
    ) -> AppResult<(reqwest::header::HeaderMap, String)> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let api_key = api_key.map(str::trim).filter(|value| !value.is_empty());
        let provider = self.provider.as_str();
        let body = match provider {
            "pinecone" => {
                if let Some(api_key) = api_key {
                    let value = reqwest::header::HeaderValue::from_str(api_key).map_err(|err| {
                        AppError::upstream_rerank(format!(
                            "invalid rerank api key header value: {err}"
                        ))
                    })?;
                    headers.insert("Api-Key", value);
                }
                headers.insert(
                    "X-Pinecone-API-Version",
                    reqwest::header::HeaderValue::from_static("2024-10"),
                );
                serde_json::json!({
                    "model": self.model,
                    "query": query,
                    "documents": docs.iter().map(|text| serde_json::json!({ "text": text })).collect::<Vec<_>>(),
                    "top_n": top_n,
                    "rank_fields": ["text"],
                })
                .to_string()
            }
            "voyage" => {
                if let Some(api_key) = api_key {
                    let value =
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))
                            .map_err(|err| {
                                AppError::upstream_rerank(format!(
                                    "invalid rerank authorization header value: {err}"
                                ))
                            })?;
                    headers.insert(AUTHORIZATION, value);
                }
                serde_json::json!({
                    "model": self.model,
                    "query": query,
                    "documents": docs,
                    "top_k": top_n,
                })
                .to_string()
            }
            "vllm" => serde_json::json!({
                "model": self.model,
                "query": query,
                "documents": docs,
                "top_n": top_n,
            })
            .to_string(),
            _ => {
                if let Some(api_key) = api_key {
                    let value =
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))
                            .map_err(|err| {
                                AppError::upstream_rerank(format!(
                                    "invalid rerank authorization header value: {err}"
                                ))
                            })?;
                    headers.insert(AUTHORIZATION, value);
                }
                serde_json::json!({
                    "model": self.model,
                    "query": query,
                    "documents": docs,
                    "top_n": top_n,
                })
                .to_string()
            }
        };
        Ok((headers, body))
    }
}

impl HashingEmbedder {
    fn new(dimensions: usize) -> anyhow::Result<Self> {
        if dimensions == 0 {
            anyhow::bail!("embedding dimensions must be > 0");
        }
        Ok(Self { dimensions })
    }

    fn embed(&self, text: &str) -> AppResult<Vec<f32>> {
        if self.dimensions == 0 {
            return Err(AppError::upstream_embedding(
                "embedding provider has invalid dimensions",
            ));
        }

        let normalized = normalize_recall_text(text);
        let mut tokens = lexical_tokens(&normalized);
        if tokens.is_empty() && !normalized.is_empty() {
            tokens.push(normalized);
        }

        let mut vector = vec![0.0_f32; self.dimensions];
        for token in tokens {
            let idx = (stable_hash64(0, token.as_bytes()) as usize) % self.dimensions;
            let sign = if stable_hash64(0x9E37_79B9_7F4A_7C15, token.as_bytes()) & 1 == 0 {
                1.0_f32
            } else {
                -1.0_f32
            };
            let weight = 1.0_f32 + (token.chars().count() as f32).ln_1p();
            vector[idx] += sign * weight;
        }

        let norm = vector
            .iter()
            .map(|value| f64::from(*value) * f64::from(*value))
            .sum::<f64>()
            .sqrt() as f32;
        if norm > 0.0_f32 {
            for value in &mut vector {
                *value /= norm;
            }
        }
        Ok(vector)
    }
}

impl EmbeddingCache {
    fn new(capacity: usize, ttl_ms: u64) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity,
            ttl_ms,
        }
    }

    fn get(&mut self, key: &str, now: i64) -> Option<Vec<f32>> {
        let entry = self.entries.get(key)?.clone();
        if self.ttl_ms > 0 && now.saturating_sub(entry.cached_at) > self.ttl_ms as i64 {
            self.entries.remove(key);
            self.order.retain(|existing| existing != key);
            return None;
        }
        self.order.retain(|existing| existing != key);
        self.order.push_back(key.to_string());
        Some(entry.vector)
    }

    fn put(&mut self, key: String, vector: Vec<f32>, now: i64) {
        if self.capacity == 0 {
            return;
        }
        if self.entries.contains_key(&key) {
            self.order.retain(|existing| existing != &key);
        } else if self.entries.len() >= self.capacity {
            while let Some(oldest) = self.order.pop_front() {
                if self.entries.remove(&oldest).is_some() {
                    break;
                }
            }
        }
        self.entries.insert(
            key.clone(),
            EmbeddingCacheEntry {
                vector,
                cached_at: now,
            },
        );
        self.order.push_back(key);
    }
}

impl LanceMemoryRepo {
    pub fn new(db_path: PathBuf, config: &AppConfig) -> anyhow::Result<Self> {
        fs::create_dir_all(&db_path)?;
        let generic_recall_engine = GenericRecallEngine::new(config)?;
        let vector_dimensions = generic_recall_engine.vector_dimensions();
        Ok(Self {
            db_path,
            generic_recall_engine,
            vector_dimensions,
        })
    }

    pub async fn store(&self, req: StoreRequest) -> AppResult<StoreResponse> {
        let mut rows = Vec::new();
        let mut results = Vec::new();
        match req {
            StoreRequest::ToolStore { actor, memory } => {
                let category = memory.category.unwrap_or(Category::Other);
                let importance = memory.importance.unwrap_or(DEFAULT_IMPORTANCE);
                let now = now_millis();
                let scope = actor.derived_scope();
                let mut row = MemoryRow {
                    id: format!("mem_{}", Uuid::new_v4().simple()),
                    principal_user_id: actor.user_id,
                    principal_agent_id: actor.agent_id,
                    text: memory.text,
                    category,
                    importance,
                    scope,
                    created_at: now,
                    updated_at: now,
                    access_count: 0,
                    last_accessed_at: 0,
                    reflection_kind: None,
                    strict_key: None,
                    vector: None,
                };
                normalize_reflection_fields(&mut row);
                row.vector = Some(self.generic_recall_engine.embed_passage(&row.text).await?);
                results.push(to_mutation_result(&row, MemoryAction::Add));
                rows.push(row);
            }
            StoreRequest::AutoCapture { actor, items } => {
                for item in items {
                    let now = now_millis();
                    let row = MemoryRow {
                        id: format!("mem_{}", Uuid::new_v4().simple()),
                        principal_user_id: actor.user_id.clone(),
                        principal_agent_id: actor.agent_id.clone(),
                        text: item.text,
                        category: Category::Other,
                        importance: DEFAULT_IMPORTANCE,
                        scope: actor.derived_scope(),
                        created_at: now,
                        updated_at: now,
                        access_count: 0,
                        last_accessed_at: 0,
                        reflection_kind: None,
                        strict_key: None,
                        vector: None,
                    };
                    results.push(to_mutation_result(&row, MemoryAction::Add));
                    rows.push(row);
                }
                let texts: Vec<String> = rows.iter().map(|row| row.text.clone()).collect();
                let vectors = self
                    .generic_recall_engine
                    .embed_passages_batch(&texts)
                    .await?;
                for (row, vector) in rows.iter_mut().zip(vectors) {
                    row.vector = Some(vector);
                }
            }
        }

        self.insert_rows(&rows).await?;
        Ok(StoreResponse { results })
    }

    pub async fn update(&self, req: UpdateRequest) -> AppResult<UpdateResponse> {
        let table = self.open_or_create_table().await?;
        let principal_filter = principal_filter(&req.actor);
        let filter = format!(
            "id = '{}' AND {principal_filter}",
            escape_sql_literal(&req.memory_id)
        );
        let mut rows = self.query_rows(&table, Some(filter)).await?;
        if rows.is_empty() {
            return Err(AppError::not_found("memory not found"));
        }
        if rows.len() > 1 {
            return Err(AppError::conflict(
                "memory update is ambiguous because multiple rows matched the same memoryId",
            ));
        }

        let mut row = rows
            .pop()
            .ok_or_else(|| AppError::not_found("memory not found"))?;
        let previous_updated_at = row.updated_at;

        let mut text_changed = false;
        if let Some(text) = req.patch.text {
            row.text = text;
            text_changed = true;
        }
        if let Some(category) = req.patch.category {
            row.category = category;
        }
        if let Some(importance) = req.patch.importance {
            row.importance = importance;
        }
        row.updated_at = now_millis();
        normalize_reflection_fields(&mut row);
        if text_changed {
            row.vector = Some(self.generic_recall_engine.embed_passage(&row.text).await?);
        }

        let update_filter = format!(
            "id = '{}' AND {principal_filter} AND updated_at = {}",
            escape_sql_literal(&row.id),
            previous_updated_at,
        );
        let mut update_builder = table
            .update()
            .only_if(update_filter)
            .column("text", sql_string_literal(&row.text))
            .column("category", sql_string_literal(row.category.as_str()))
            .column("importance", row.importance.to_string())
            .column("updated_at", row.updated_at.to_string());

        let reflection_kind_expr = row.reflection_kind.map(reflection_kind_to_str);
        update_builder = update_builder.column(
            "reflection_kind",
            sql_optional_string_literal(reflection_kind_expr),
        );
        update_builder = update_builder.column(
            "strict_key",
            sql_optional_string_literal(row.strict_key.as_deref()),
        );
        update_builder = update_builder.column(
            "vector",
            sql_optional_f32_list_literal(row.vector.as_deref()),
        );

        let update_result = update_builder
            .execute()
            .await
            .map_err(|err| AppError::internal(format!("failed to update memory row: {err}")))?;

        if update_result.rows_updated == 0 {
            return Err(AppError::conflict(
                "memory update failed due to a concurrent modification; retry with a new idempotency key",
            ));
        }
        if update_result.rows_updated > 1 {
            return Err(AppError::internal(format!(
                "memory update affected {} rows; expected exactly one row",
                update_result.rows_updated
            )));
        }

        Ok(UpdateResponse {
            result: to_mutation_result(&row, MemoryAction::Update),
        })
    }

    pub async fn delete(&self, req: DeleteRequest) -> AppResult<u64> {
        let table = self.open_or_create_table().await?;
        if let Some(memory_id) = req.memory_id {
            let filter = format!(
                "id = '{}' AND {}",
                escape_sql_literal(&memory_id),
                principal_filter(&req.actor)
            );
            let rows = self.query_rows(&table, Some(filter)).await?;
            if rows.is_empty() {
                return Err(AppError::not_found("memory not found"));
            }
            self.delete_row_by_id(&table, &memory_id, &req.actor)
                .await?;
            return Ok(1);
        }

        let query = req.query.unwrap_or_default().to_lowercase();
        let rows = self
            .query_rows(&table, Some(principal_filter(&req.actor)))
            .await?;
        let delete_ids: Vec<String> = rows
            .into_iter()
            .filter(|row| row.text.to_lowercase().contains(&query))
            .map(|row| row.id)
            .collect();

        let deleted = delete_ids.len() as u64;
        for id in delete_ids {
            self.delete_row_by_id(&table, &id, &req.actor).await?;
        }
        Ok(deleted)
    }

    pub async fn list(&self, req: ListRequest) -> AppResult<ListResponse> {
        let limit = clamped_limit(req.limit) as usize;
        let table = self.open_or_create_table().await?;
        let mut rows = self
            .query_rows(&table, Some(principal_filter(&req.actor)))
            .await?;

        if let Some(category) = req.category {
            rows.retain(|row| row.category == category);
        }

        rows.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });

        let total = rows.len();
        let start = req.offset as usize;
        let page_rows: Vec<ListRow> = if start >= total {
            Vec::new()
        } else {
            rows.into_iter()
                .skip(start)
                .take(limit)
                .map(to_list_row)
                .collect()
        };
        let consumed = start + page_rows.len();
        let next_offset = if consumed < total {
            Some(consumed as u64)
        } else {
            None
        };

        Ok(ListResponse {
            rows: page_rows,
            next_offset,
        })
    }

    pub async fn stats(&self, actor: &Actor) -> AppResult<StatsResponse> {
        let table = self.open_or_create_table().await?;
        let rows = self
            .query_rows(&table, Some(principal_filter(actor)))
            .await?;

        let mut categories = BTreeMap::new();
        let mut reflection_count = 0_u64;
        for row in &rows {
            if row.category == Category::Reflection {
                reflection_count += 1;
            }
            *categories
                .entry(row.category.as_str().to_string())
                .or_insert(0_u64) += 1;
        }

        Ok(StatsResponse {
            memory_count: rows.len() as u64,
            reflection_count,
            categories,
        })
    }

    pub async fn recall_generic(
        &self,
        req: RecallGenericRequest,
    ) -> AppResult<RecallGenericResponse> {
        let (response, _) = self.recall_generic_internal(req, false).await?;
        Ok(response)
    }

    pub async fn recall_generic_with_trace(
        &self,
        req: RecallGenericRequest,
    ) -> AppResult<(RecallGenericResponse, RetrievalTrace)> {
        let (response, trace) = self.recall_generic_internal(req, true).await?;
        Ok((
            response,
            trace.expect("trace must exist when recall_generic_with_trace is requested"),
        ))
    }

    async fn recall_generic_internal(
        &self,
        req: RecallGenericRequest,
        include_trace: bool,
    ) -> AppResult<(RecallGenericResponse, Option<RetrievalTrace>)> {
        let limit = clamped_limit(req.limit) as usize;
        let table = self.open_or_create_table().await?;
        let query_embedding = self.generic_recall_engine.embed_query(&req.query).await?;
        let lexical_query = self.generic_recall_engine.lexical_query(&req.query);
        let filter = principal_filter(&req.actor);
        let mut trace = include_trace.then(RetrievalTraceCollector::default);
        if let Some(trace) = trace.as_mut() {
            let mut stage = make_trace_stage("embed.query", RetrievalTraceStageStatus::Ok);
            stage
                .metrics
                .insert("dimensions".to_string(), json!(query_embedding.len()));
            trace.push(stage);
        }
        let seeds = self
            .fetch_recall_seeds(
                &table,
                &lexical_query,
                &query_embedding,
                &filter,
                limit,
                trace.as_mut(),
            )
            .await?;
        let ranked = self
            .generic_recall_engine
            .rank_candidates(
                &req.query,
                &lexical_query,
                &query_embedding,
                seeds,
                limit,
                trace.as_mut(),
            )
            .await?;
        let ranked = apply_generic_recall_filters(ranked, &req);
        if let Err(err) = self
            .record_recall_access_metadata(&table, &req.actor, &ranked)
            .await
        {
            emit_internal_diagnostic(
                self.generic_recall_engine.settings.diagnostics,
                json!({
                    "event": "retrieval.access.update-failed",
                    "reason": truncate_for_error(&format!("{err:?}"), 240),
                }),
            );
            if let Some(trace) = trace.as_mut() {
                let mut stage =
                    make_trace_stage("access-update", RetrievalTraceStageStatus::Failed);
                stage.input_count = Some(ranked.len() as u64);
                stage.output_count = Some(0);
                stage.reason = Some(truncate_for_error(&format!("{err:?}"), 240));
                trace.push(stage);
            }
        } else if let Some(trace) = trace.as_mut() {
            let mut stage = make_trace_stage("access-update", RetrievalTraceStageStatus::Ok);
            stage.input_count = Some(ranked.len() as u64);
            stage.output_count = Some(ranked.len().min(ACCESS_UPDATE_MAX_ROWS) as u64);
            trace.push(stage);
        }
        let response_rows = ranked
            .into_iter()
            .map(|ranked_row| RecallGenericRow {
                id: ranked_row.row.id,
                text: ranked_row.row.text,
                category: ranked_row.row.category,
                scope: ranked_row.row.scope,
                score: ranked_row.score,
                metadata: RowMetadata {
                    created_at: ranked_row.row.created_at,
                    updated_at: ranked_row.row.updated_at,
                },
            })
            .collect();

        let response = RecallGenericResponse {
            rows: response_rows,
        };
        let trace = trace.map(|trace| {
            trace.finish(
                RetrievalTraceKind::Generic,
                &req.query,
                &lexical_query,
                None,
            )
        });
        Ok((response, trace))
    }

    pub async fn recall_reflection(
        &self,
        req: RecallReflectionRequest,
    ) -> AppResult<RecallReflectionResponse> {
        let (response, _) = self.recall_reflection_internal(req, false).await?;
        Ok(response)
    }

    pub async fn recall_reflection_with_trace(
        &self,
        req: RecallReflectionRequest,
    ) -> AppResult<(RecallReflectionResponse, RetrievalTrace)> {
        let (response, trace) = self.recall_reflection_internal(req, true).await?;
        Ok((
            response,
            trace.expect("trace must exist when recall_reflection_with_trace is requested"),
        ))
    }

    async fn recall_reflection_internal(
        &self,
        req: RecallReflectionRequest,
        include_trace: bool,
    ) -> AppResult<(RecallReflectionResponse, Option<RetrievalTrace>)> {
        let mode = req.mode.unwrap_or(ReflectionRecallMode::InvariantDerived);
        let limit = clamped_limit(req.limit) as usize;
        let table = self.open_or_create_table().await?;
        let query_embedding = self.generic_recall_engine.embed_query(&req.query).await?;
        let lexical_query = self.generic_recall_engine.lexical_query(&req.query);
        let mut trace = include_trace.then(RetrievalTraceCollector::default);
        if let Some(trace) = trace.as_mut() {
            let mut stage = make_trace_stage("embed.query", RetrievalTraceStageStatus::Ok);
            stage
                .metrics
                .insert("dimensions".to_string(), json!(query_embedding.len()));
            stage.metrics.insert(
                "reflectionMode".to_string(),
                json!(match mode {
                    ReflectionRecallMode::InvariantOnly => "invariant-only",
                    ReflectionRecallMode::InvariantDerived => "invariant+derived",
                }),
            );
            trace.push(stage);
        }
        let mut filter = format!(
            "{} AND category = '{}'",
            principal_filter(&req.actor),
            Category::Reflection.as_str()
        );
        if matches!(mode, ReflectionRecallMode::InvariantOnly) {
            filter.push_str(" AND reflection_kind = 'invariant'");
        }
        let seeds = self
            .fetch_recall_seeds(
                &table,
                &lexical_query,
                &query_embedding,
                &filter,
                limit,
                trace.as_mut(),
            )
            .await?;
        let ranked = self
            .generic_recall_engine
            .rank_candidates(
                &req.query,
                &lexical_query,
                &query_embedding,
                seeds,
                limit,
                trace.as_mut(),
            )
            .await?;
        let ranked = apply_reflection_recall_filters(ranked, &req);
        if let Err(err) = self
            .record_recall_access_metadata(&table, &req.actor, &ranked)
            .await
        {
            emit_internal_diagnostic(
                self.generic_recall_engine.settings.diagnostics,
                json!({
                    "event": "retrieval.access.update-failed",
                    "reason": truncate_for_error(&format!("{err:?}"), 240),
                }),
            );
            if let Some(trace) = trace.as_mut() {
                let mut stage =
                    make_trace_stage("access-update", RetrievalTraceStageStatus::Failed);
                stage.input_count = Some(ranked.len() as u64);
                stage.output_count = Some(0);
                stage.reason = Some(truncate_for_error(&format!("{err:?}"), 240));
                trace.push(stage);
            }
        } else if let Some(trace) = trace.as_mut() {
            let mut stage = make_trace_stage("access-update", RetrievalTraceStageStatus::Ok);
            stage.input_count = Some(ranked.len() as u64);
            stage.output_count = Some(ranked.len().min(ACCESS_UPDATE_MAX_ROWS) as u64);
            trace.push(stage);
        }

        let rows = ranked
            .into_iter()
            .map(|ranked_row| {
                let kind = ranked_row
                    .row
                    .reflection_kind
                    .unwrap_or(ReflectionKind::Derived);
                let strict_key = if matches!(kind, ReflectionKind::Invariant) {
                    Some(
                        ranked_row
                            .row
                            .strict_key
                            .unwrap_or_else(|| default_strict_key(&ranked_row.row.id)),
                    )
                } else {
                    None
                };
                crate::models::RecallReflectionRow {
                    id: ranked_row.row.id,
                    text: ranked_row.row.text,
                    kind,
                    strict_key,
                    scope: ranked_row.row.scope,
                    score: ranked_row.score,
                    metadata: ReflectionMetadata {
                        timestamp: ranked_row.row.created_at,
                    },
                }
            })
            .collect::<Vec<_>>();

        let response = RecallReflectionResponse { rows };
        let trace = trace.map(|trace| {
            trace.finish(
                RetrievalTraceKind::Reflection,
                &req.query,
                &lexical_query,
                Some(match mode {
                    ReflectionRecallMode::InvariantOnly => "invariant-only".to_string(),
                    ReflectionRecallMode::InvariantDerived => "invariant+derived".to_string(),
                }),
            )
        });
        Ok((response, trace))
    }

    async fn record_recall_access_metadata(
        &self,
        table: &LanceTable,
        actor: &Actor,
        ranked: &[RankedMemoryRow],
    ) -> AppResult<()> {
        if ranked.is_empty() {
            return Ok(());
        }

        let now = now_millis();
        let principal = principal_filter(actor);
        for ranked_row in ranked.iter().take(ACCESS_UPDATE_MAX_ROWS) {
            let predicate = format!(
                "id = '{}' AND {}",
                escape_sql_literal(&ranked_row.row.id),
                principal
            );
            let next_access_count =
                clamp_access_count(ranked_row.row.access_count.saturating_add(1));
            table
                .update()
                .only_if(predicate)
                .column("access_count", next_access_count.to_string())
                .column("last_accessed_at", now.to_string())
                .execute()
                .await
                .map_err(|err| {
                    AppError::internal(format!("failed to update recall access metadata: {err}"))
                })?;
        }
        Ok(())
    }

    async fn fetch_recall_seeds(
        &self,
        table: &LanceTable,
        lexical_query: &str,
        query_embedding: &[f32],
        filter: &str,
        limit: usize,
        mut trace: Option<&mut RetrievalTraceCollector>,
    ) -> AppResult<Vec<CandidateSeed>> {
        let candidate_pool_size = self.generic_recall_engine.candidate_pool_size(limit);
        let diagnostics = self.generic_recall_engine.settings.diagnostics;

        let mut had_retrieval_error = false;
        let vector_hits = match self
            .query_vector_candidates(table, query_embedding, filter, candidate_pool_size)
            .await
        {
            Ok(rows) => {
                if let Some(trace) = trace.as_deref_mut() {
                    let mut stage =
                        make_trace_stage("seed.vector-search", RetrievalTraceStageStatus::Ok);
                    stage.output_count = Some(rows.len() as u64);
                    stage
                        .metrics
                        .insert("candidatePoolSize".to_string(), json!(candidate_pool_size));
                    trace.push(stage);
                }
                rows
            }
            Err(err) => {
                had_retrieval_error = true;
                emit_internal_diagnostic(
                    diagnostics,
                    json!({
                        "event": "retrieval.seed.fallback",
                        "stage": "vector-search",
                        "fallback": "fts-or-scan",
                        "reason": truncate_for_error(&format!("{err:?}"), 240),
                    }),
                );
                if let Some(trace) = trace.as_deref_mut() {
                    let mut stage =
                        make_trace_stage("seed.vector-search", RetrievalTraceStageStatus::Fallback);
                    stage.output_count = Some(0);
                    stage.fallback_to = Some("fts-or-scan".to_string());
                    stage.reason = Some(truncate_for_error(&format!("{err:?}"), 240));
                    trace.push(stage);
                }
                Vec::new()
            }
        };
        let fts_hits = match self
            .query_fts_candidates(table, lexical_query, filter, candidate_pool_size)
            .await
        {
            Ok(rows) => {
                if let Some(trace) = trace.as_deref_mut() {
                    let mut stage =
                        make_trace_stage("seed.fts-search", RetrievalTraceStageStatus::Ok);
                    stage.output_count = Some(rows.len() as u64);
                    stage
                        .metrics
                        .insert("candidatePoolSize".to_string(), json!(candidate_pool_size));
                    trace.push(stage);
                }
                rows
            }
            Err(err) => {
                had_retrieval_error = true;
                emit_internal_diagnostic(
                    diagnostics,
                    json!({
                        "event": "retrieval.seed.fallback",
                        "stage": "fts-search",
                        "fallback": "vector-or-scan",
                        "reason": truncate_for_error(&format!("{err:?}"), 240),
                    }),
                );
                if let Some(trace) = trace.as_deref_mut() {
                    let mut stage =
                        make_trace_stage("seed.fts-search", RetrievalTraceStageStatus::Fallback);
                    stage.output_count = Some(0);
                    stage.fallback_to = Some("vector-or-scan".to_string());
                    stage.reason = Some(truncate_for_error(&format!("{err:?}"), 240));
                    trace.push(stage);
                }
                Vec::new()
            }
        };

        let mut merged: HashMap<String, CandidateSeed> = HashMap::new();
        let vector_hit_count = vector_hits.len();
        for (row, vector_score) in vector_hits {
            let entry = merged.entry(row.id.clone()).or_insert(CandidateSeed {
                row,
                vector_score: None,
                bm25_score: None,
            });
            entry.vector_score = Some(entry.vector_score.unwrap_or(0.0).max(vector_score));
        }

        let mut fts_hits = fts_hits;
        let fts_hit_count = fts_hits.len();
        fts_hits.sort_by(|a, b| {
            b.updated_at
                .cmp(&a.updated_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        for (idx, row) in fts_hits.into_iter().enumerate() {
            let rank = idx as f64 + 1.0;
            let bm25_score = clamp_score(1.0 / (1.0 + rank.ln_1p()));
            let entry = merged.entry(row.id.clone()).or_insert(CandidateSeed {
                row,
                vector_score: None,
                bm25_score: None,
            });
            entry.bm25_score = Some(entry.bm25_score.unwrap_or(0.0).max(bm25_score));
        }

        let mut seeds: Vec<CandidateSeed> = merged.into_values().collect();
        let mut full_scan_used = false;
        if seeds.is_empty() && had_retrieval_error {
            emit_internal_diagnostic(
                diagnostics,
                json!({
                    "event": "retrieval.seed.fallback",
                    "stage": "full-scan",
                    "fallback": "scan-principal-rows",
                    "reason": "vector-and-fts-empty-after-errors",
                }),
            );
            full_scan_used = true;
            seeds = self
                .query_rows(table, Some(filter.to_string()))
                .await?
                .into_iter()
                .map(|row| CandidateSeed {
                    row,
                    vector_score: None,
                    bm25_score: None,
                })
                .collect();
        }
        if let Some(trace) = trace {
            let mut stage = if full_scan_used {
                make_trace_stage("seed.merge", RetrievalTraceStageStatus::Fallback)
            } else {
                make_trace_stage("seed.merge", RetrievalTraceStageStatus::Ok)
            };
            stage.input_count = Some((vector_hit_count + fts_hit_count) as u64);
            stage.output_count = Some(seeds.len() as u64);
            stage
                .metrics
                .insert("vectorHitCount".to_string(), json!(vector_hit_count));
            stage
                .metrics
                .insert("ftsHitCount".to_string(), json!(fts_hit_count));
            stage
                .metrics
                .insert("fullScanUsed".to_string(), json!(full_scan_used));
            if full_scan_used {
                stage.fallback_to = Some("full-scan".to_string());
                stage.reason = Some("vector-and-fts-empty-after-errors".to_string());
            }
            trace.push(stage);
        }
        Ok(seeds)
    }

    async fn query_vector_candidates(
        &self,
        table: &LanceTable,
        query_embedding: &[f32],
        filter: &str,
        candidate_pool_size: usize,
    ) -> AppResult<Vec<(MemoryRow, f64)>> {
        let search = table
            .vector_search(query_embedding.to_vec())
            .map_err(|err| AppError::internal(format!("failed to build vector query: {err}")))?
            .distance_type(DistanceType::Cosine)
            .limit(candidate_pool_size)
            .only_if(filter.to_string());
        let stream = search.execute().await.map_err(|err| {
            AppError::internal(format!("failed to execute vector candidate query: {err}"))
        })?;
        let batches = stream
            .try_collect::<Vec<RecordBatch>>()
            .await
            .map_err(|err| {
                AppError::internal(format!("failed to stream vector candidates: {err}"))
            })?;
        let rows = rows_from_batches(&batches)?;
        let hits = rows
            .into_iter()
            .map(|row| {
                let score = row
                    .vector
                    .as_ref()
                    .map(|vector| {
                        clamp_score(
                            (cosine_similarity_f32(query_embedding, vector).clamp(-1.0, 1.0) + 1.0)
                                / 2.0,
                        )
                    })
                    .unwrap_or(0.0);
                (row, score)
            })
            .collect();
        Ok(hits)
    }

    async fn query_fts_candidates(
        &self,
        table: &LanceTable,
        query: &str,
        filter: &str,
        candidate_pool_size: usize,
    ) -> AppResult<Vec<MemoryRow>> {
        self.ensure_text_fts_index(table).await?;
        let search = table
            .query()
            .full_text_search(FullTextSearchQuery::new(query.to_string()))
            .limit(candidate_pool_size)
            .only_if(filter.to_string());
        let stream = search.execute().await.map_err(|err| {
            AppError::internal(format!("failed to execute FTS candidate query: {err}"))
        })?;
        let batches = stream
            .try_collect::<Vec<RecordBatch>>()
            .await
            .map_err(|err| AppError::internal(format!("failed to stream FTS candidates: {err}")))?;
        rows_from_batches(&batches)
    }

    async fn insert_rows(&self, rows: &[MemoryRow]) -> AppResult<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let table = self.open_or_create_table().await?;
        self.insert_rows_into_table(&table, rows).await?;
        self.ensure_vector_index(&table).await?;
        Ok(())
    }

    async fn insert_rows_into_table(
        &self,
        table: &LanceTable,
        rows: &[MemoryRow],
    ) -> AppResult<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let batch = rows_to_record_batch(rows, self.vector_dimensions)?;
        let schema = batch.schema();
        let reader = RecordBatchIterator::new(
            vec![Ok::<RecordBatch, ArrowError>(batch)].into_iter(),
            schema,
        );
        table
            .add(reader)
            .execute()
            .await
            .map_err(|err| AppError::internal(format!("failed to insert memory rows: {err}")))?;
        Ok(())
    }

    async fn delete_row_by_id(
        &self,
        table: &LanceTable,
        memory_id: &str,
        actor: &Actor,
    ) -> AppResult<()> {
        let predicate = format!(
            "id = '{}' AND {}",
            escape_sql_literal(memory_id),
            principal_filter(actor)
        );
        table
            .delete(&predicate)
            .await
            .map_err(|err| AppError::internal(format!("failed to delete memory row: {err}")))?;
        Ok(())
    }

    async fn query_rows(
        &self,
        table: &LanceTable,
        filter: Option<String>,
    ) -> AppResult<Vec<MemoryRow>> {
        let query = match filter {
            Some(filter) => table.query().only_if(filter),
            None => table.query(),
        };

        let stream = query
            .execute()
            .await
            .map_err(|err| AppError::internal(format!("failed to query memory table: {err}")))?;
        let batches = stream
            .try_collect::<Vec<RecordBatch>>()
            .await
            .map_err(|err| AppError::internal(format!("failed to stream memory rows: {err}")))?;
        rows_from_batches(&batches)
    }

    async fn open_or_create_table(&self) -> AppResult<LanceTable> {
        let conn = self.connect().await?;
        match conn.open_table(MEMORY_TABLE_NAME).execute().await {
            Ok(table) => {
                self.ensure_table_compatibility_and_indexes(&conn, table)
                    .await
            }
            Err(LanceError::TableNotFound { .. }) => self.create_table(&conn).await,
            Err(err) => Err(AppError::internal(format!(
                "failed to open LanceDB memory table: {err}"
            ))),
        }
    }

    async fn ensure_table_compatibility_and_indexes(
        &self,
        conn: &LanceConnection,
        table: LanceTable,
    ) -> AppResult<LanceTable> {
        let capabilities = self.inspect_table_capabilities(&table).await?;
        let needs_rebuild = if capabilities.has_vector_column {
            if let Some(dim) = capabilities.vector_dimensions {
                if dim != self.vector_dimensions {
                    return Err(AppError::backend_unavailable(format!(
                        "LanceDB vector column dimension mismatch: table={} config={}; reindex or migrate data",
                        dim, self.vector_dimensions
                    )));
                }
            }
            !capabilities.has_access_metadata_columns
        } else {
            true
        };

        let table = if needs_rebuild {
            self.migrate_legacy_table_to_current_schema(conn, &table, capabilities)
                .await?
        } else {
            table
        };

        self.ensure_text_fts_index(&table).await?;
        self.ensure_vector_index(&table).await?;
        Ok(table)
    }

    async fn inspect_table_capabilities(&self, table: &LanceTable) -> AppResult<TableCapabilities> {
        let schema = table
            .schema()
            .await
            .map_err(|err| AppError::internal(format!("failed to read table schema: {err}")))?;
        let has_access_metadata_columns = schema.field_with_name("access_count").is_ok()
            && schema.field_with_name("last_accessed_at").is_ok();
        match schema.field_with_name("vector") {
            Ok(field) => {
                let vector_dimensions = match field.data_type() {
                    DataType::FixedSizeList(item, dims) => {
                        if item.data_type() != &DataType::Float32 {
                            return Err(AppError::backend_unavailable(
                                "LanceDB vector column item type must be Float32",
                            ));
                        }
                        if *dims <= 0 {
                            return Err(AppError::backend_unavailable(
                                "LanceDB vector column has invalid dimensions",
                            ));
                        }
                        Some(*dims as usize)
                    }
                    _ => {
                        return Err(AppError::backend_unavailable(
                            "LanceDB vector column has unexpected data type",
                        ));
                    }
                };
                Ok(TableCapabilities {
                    has_vector_column: true,
                    vector_dimensions,
                    has_access_metadata_columns,
                })
            }
            Err(_) => Ok(TableCapabilities {
                has_vector_column: false,
                vector_dimensions: None,
                has_access_metadata_columns,
            }),
        }
    }

    async fn migrate_legacy_table_to_current_schema(
        &self,
        conn: &LanceConnection,
        table: &LanceTable,
        capabilities: TableCapabilities,
    ) -> AppResult<LanceTable> {
        let backup_table_name = format!("{}_legacy_backup_{}", MEMORY_TABLE_NAME, now_millis());
        let legacy_schema = table.schema().await.map_err(|err| {
            AppError::internal(format!("failed to inspect legacy table schema: {err}"))
        })?;
        let legacy_batches = table
            .query()
            .execute()
            .await
            .map_err(|err| AppError::internal(format!("failed to snapshot legacy table: {err}")))?
            .try_collect::<Vec<RecordBatch>>()
            .await
            .map_err(|err| {
                AppError::internal(format!("failed to stream legacy table snapshot: {err}"))
            })?;
        let mut legacy_rows = rows_from_batches(&legacy_batches)?;
        let needs_vector_backfill = !capabilities.has_vector_column;
        if needs_vector_backfill {
            for row in &mut legacy_rows {
                row.vector = None;
            }
        }

        let backup_table = conn
            .create_empty_table(&backup_table_name, legacy_schema.clone())
            .execute()
            .await
            .map_err(|err| {
                AppError::internal(format!("failed to create legacy backup table: {err}"))
            })?;
        if !legacy_batches.is_empty() {
            let backup_reader = RecordBatchIterator::new(
                legacy_batches
                    .clone()
                    .into_iter()
                    .map(Ok::<RecordBatch, ArrowError>),
                legacy_schema,
            );
            backup_table
                .add(backup_reader)
                .execute()
                .await
                .map_err(|err| {
                    AppError::internal(format!(
                        "failed to populate legacy backup table {}: {err}",
                        backup_table_name
                    ))
                })?;
        }

        conn.drop_table(MEMORY_TABLE_NAME, &[])
            .await
            .map_err(|err| {
                AppError::internal(format!("failed to drop legacy memory table: {err}"))
            })?;
        let rebuilt = self.create_table(conn).await?;
        if !legacy_rows.is_empty() && needs_vector_backfill {
            let mut texts = Vec::with_capacity(legacy_rows.len());
            for row in &legacy_rows {
                texts.push(row.text.clone());
            }
            match self
                .generic_recall_engine
                .embed_passages_batch(&texts)
                .await
            {
                Ok(vectors) if vectors.len() == legacy_rows.len() => {
                    for (row, vector) in legacy_rows.iter_mut().zip(vectors) {
                        row.vector = Some(vector);
                    }
                }
                Ok(vectors) => {
                    eprintln!(
                        "legacy vector backfill returned {} vectors for {} rows; preserving rows with null vectors",
                        vectors.len(),
                        legacy_rows.len()
                    );
                }
                Err(err) => {
                    eprintln!(
                        "legacy vector backfill failed: {err:?}; preserving rows with null vectors"
                    );
                }
            }
        }
        if !legacy_rows.is_empty() {
            self.insert_rows_into_table(&rebuilt, &legacy_rows).await?;
        }
        self.ensure_text_fts_index(&rebuilt).await?;
        self.ensure_vector_index(&rebuilt).await?;
        eprintln!(
            "migrated LanceDB table to current schema; backup table={}",
            backup_table_name
        );
        Ok(rebuilt)
    }

    async fn ensure_text_fts_index(&self, table: &LanceTable) -> AppResult<()> {
        let indices = table
            .list_indices()
            .await
            .map_err(|err| AppError::internal(format!("failed to list table indices: {err}")))?;
        let has_text_fts = indices.iter().any(|index| {
            index.index_type == IndexType::FTS
                && index.columns.iter().any(|column| column.as_str() == "text")
        });
        if has_text_fts {
            return Ok(());
        }
        table
            .create_index(&["text"], Index::FTS(Default::default()))
            .replace(true)
            .execute()
            .await
            .map_err(|err| AppError::internal(format!("failed to create text FTS index: {err}")))?;
        Ok(())
    }

    async fn ensure_vector_index(&self, table: &LanceTable) -> AppResult<()> {
        let capabilities = self.inspect_table_capabilities(table).await?;
        if !capabilities.has_vector_column {
            return Ok(());
        }

        let indices = table
            .list_indices()
            .await
            .map_err(|err| AppError::internal(format!("failed to list table indices: {err}")))?;
        let has_vector_index = indices.iter().any(|index| {
            index
                .columns
                .iter()
                .any(|column| column.as_str() == "vector")
                && is_vector_index_type(&index.index_type)
        });
        if has_vector_index {
            return Ok(());
        }

        let row_count = table
            .count_rows(None)
            .await
            .map_err(|err| AppError::internal(format!("failed to count table rows: {err}")))?;
        if row_count == 0 {
            return Ok(());
        }

        table
            .create_index(&["vector"], Index::IvfFlat(Default::default()))
            .replace(true)
            .execute()
            .await
            .map_err(|err| {
                AppError::internal(format!("failed to create vector index on 'vector': {err}"))
            })?;
        Ok(())
    }

    async fn create_table(&self, conn: &LanceConnection) -> AppResult<LanceTable> {
        let schema = memory_table_schema(self.vector_dimensions);
        let table = match conn
            .create_empty_table(MEMORY_TABLE_NAME, schema)
            .execute()
            .await
        {
            Ok(table) => table,
            Err(LanceError::TableAlreadyExists { .. }) => conn
                .open_table(MEMORY_TABLE_NAME)
                .execute()
                .await
                .map_err(|err| {
                    AppError::internal(format!(
                        "failed to open existing LanceDB memory table: {err}"
                    ))
                })?,
            Err(err) => {
                return Err(AppError::internal(format!(
                    "failed to create LanceDB memory table: {err}"
                )));
            }
        };
        self.ensure_text_fts_index(&table).await?;
        self.ensure_vector_index(&table).await?;
        Ok(table)
    }

    async fn connect(&self) -> AppResult<LanceConnection> {
        let uri = self.db_path.to_string_lossy().to_string();
        connect(&uri).execute().await.map_err(|err| {
            AppError::backend_unavailable(format!(
                "failed to connect LanceDB at {}: {err}",
                self.db_path.display()
            ))
        })
    }
}

fn apply_generic_recall_filters(
    ranked: Vec<RankedMemoryRow>,
    req: &RecallGenericRequest,
) -> Vec<RankedMemoryRow> {
    let category_allowlist = req
        .categories
        .as_ref()
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
        });
    let exclude_reflection = req.exclude_reflection.unwrap_or(false);
    let max_age_ms = req.max_age_days.map(|days| days.saturating_mul(86_400_000));
    let now = now_millis();

    let filtered = ranked
        .into_iter()
        .filter(|row| {
            category_allowlist
                .as_ref()
                .map(|allowed| allowed.contains(&row.row.category))
                .unwrap_or(true)
        })
        .filter(|row| !(exclude_reflection && matches!(row.row.category, Category::Reflection)))
        .filter(|row| {
            max_age_ms
                .map(|max_age| {
                    let ts = if row.row.updated_at > 0 {
                        row.row.updated_at
                    } else {
                        row.row.created_at
                    };
                    ts > 0 && now.saturating_sub(ts) <= max_age as i64
                })
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    apply_max_entries_per_key(filtered, req.max_entries_per_key)
}

fn apply_reflection_recall_filters(
    ranked: Vec<RankedMemoryRow>,
    req: &RecallReflectionRequest,
) -> Vec<RankedMemoryRow> {
    let allowed_kinds = req
        .include_kinds
        .as_ref()
        .filter(|items| !items.is_empty())
        .map(|items| {
            items
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
        });
    let min_score = req.min_score.unwrap_or(0.0);

    ranked
        .into_iter()
        .filter(|row| row.score.is_finite() && row.score >= min_score)
        .filter(|row| {
            allowed_kinds
                .as_ref()
                .map(|allowed| {
                    let kind = row.row.reflection_kind.unwrap_or(ReflectionKind::Derived);
                    allowed.contains(&kind)
                })
                .unwrap_or(true)
        })
        .collect()
}

fn apply_max_entries_per_key(
    ranked: Vec<RankedMemoryRow>,
    max_entries_per_key: Option<u64>,
) -> Vec<RankedMemoryRow> {
    let max_entries = max_entries_per_key.unwrap_or(0);
    if max_entries == 0 {
        return ranked;
    }

    let mut counts_by_key = std::collections::BTreeMap::<String, u64>::new();
    let mut kept = Vec::with_capacity(ranked.len());
    for row in ranked {
        let key = normalize_recall_text_key(&row.row.text);
        if key.is_empty() {
            kept.push(row);
            continue;
        }
        let current = counts_by_key.get(&key).copied().unwrap_or(0);
        if current >= max_entries {
            continue;
        }
        counts_by_key.insert(key, current + 1);
        kept.push(row);
    }
    kept
}

fn normalize_recall_text_key(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
}

#[derive(Clone)]
pub struct IdempotencyStore {
    sqlite_path: PathBuf,
}

#[derive(Clone)]
pub struct IdempotencyReservation {
    store: IdempotencyStore,
    principal: Principal,
    operation: String,
    idempotency_key: String,
}

#[derive(Clone, Copy)]
enum IdempotencyStatus {
    Reserved,
    InProgress,
    Completed,
    Failed,
}

impl IdempotencyStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    fn parse(raw: &str) -> AppResult<Self> {
        match raw {
            "reserved" => Ok(Self::Reserved),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(AppError::internal(format!(
                "unknown idempotency status persisted: {raw}"
            ))),
        }
    }
}

impl IdempotencyStore {
    pub fn new(sqlite_path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = sqlite_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let this = Self { sqlite_path };
        this.init_schema()?;
        Ok(this)
    }

    pub fn reserve(
        &self,
        principal: &Principal,
        operation: &str,
        idempotency_key: &str,
        request_fingerprint: &str,
    ) -> AppResult<IdempotencyReservation> {
        validate_non_empty("idempotency-key", idempotency_key)?;
        validate_non_empty("operation", operation)?;

        let now = now_millis();
        let mut conn = self.open_conn().map_err(AppError::from)?;
        let tx = conn.transaction().map_err(|err| {
            AppError::internal(format!(
                "failed to start idempotency reservation transaction: {err}"
            ))
        })?;

        let inserted = tx
            .execute(
                "INSERT OR IGNORE INTO idempotency_keys (
                    user_id,
                    agent_id,
                    operation,
                    idempotency_key,
                    request_fingerprint,
                    status,
                    created_at,
                    updated_at,
                    last_error
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, NULL)",
                params![
                    &principal.user_id,
                    &principal.agent_id,
                    operation,
                    idempotency_key,
                    request_fingerprint,
                    IdempotencyStatus::Reserved.as_str(),
                    now,
                ],
            )
            .map_err(|err| {
                AppError::internal(format!(
                    "failed to persist idempotency key reservation: {err}"
                ))
            })?;

        if inserted == 0 {
            let existing: Option<(String, String)> = tx
                .query_row(
                    "SELECT request_fingerprint, status
                     FROM idempotency_keys
                     WHERE user_id = ?1 AND agent_id = ?2 AND operation = ?3 AND idempotency_key = ?4",
                    params![
                        &principal.user_id,
                        &principal.agent_id,
                        operation,
                        idempotency_key
                    ],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|err| {
                    AppError::internal(format!("failed to read idempotency key reservation: {err}"))
                })?;

            let Some((existing_fingerprint, status_raw)) = existing else {
                return Err(AppError::internal(
                    "idempotency key was not inserted but no existing reservation was found",
                ));
            };

            if existing_fingerprint != request_fingerprint {
                return Err(AppError::idempotency_conflict(
                    "idempotency key is already bound to a different payload",
                ));
            }

            let status = IdempotencyStatus::parse(&status_raw)?;
            match status {
                IdempotencyStatus::Completed => {
                    return Err(AppError::idempotency_conflict(
                        "idempotent response replay is not supported yet in MVP; retry with a new idempotency key",
                    ));
                }
                IdempotencyStatus::Reserved | IdempotencyStatus::InProgress => {
                    return Err(AppError::idempotency_conflict(
                        "idempotency key is currently in progress",
                    ));
                }
                IdempotencyStatus::Failed => {
                    tx.execute(
                        "UPDATE idempotency_keys
                         SET status = ?5,
                             updated_at = ?6,
                             last_error = NULL
                         WHERE user_id = ?1
                           AND agent_id = ?2
                           AND operation = ?3
                           AND idempotency_key = ?4",
                        params![
                            &principal.user_id,
                            &principal.agent_id,
                            operation,
                            idempotency_key,
                            IdempotencyStatus::Reserved.as_str(),
                            now,
                        ],
                    )
                    .map_err(|err| {
                        AppError::internal(format!(
                            "failed to reset failed idempotency key reservation: {err}"
                        ))
                    })?;
                }
            }
        }

        let promoted = tx
            .execute(
                "UPDATE idempotency_keys
                 SET status = ?5,
                     updated_at = ?6,
                     last_error = NULL
                 WHERE user_id = ?1
                   AND agent_id = ?2
                   AND operation = ?3
                   AND idempotency_key = ?4
                   AND status = ?7",
                params![
                    &principal.user_id,
                    &principal.agent_id,
                    operation,
                    idempotency_key,
                    IdempotencyStatus::InProgress.as_str(),
                    now,
                    IdempotencyStatus::Reserved.as_str(),
                ],
            )
            .map_err(|err| {
                AppError::internal(format!("failed to promote idempotency reservation: {err}"))
            })?;

        if promoted != 1 {
            return Err(AppError::idempotency_conflict(
                "idempotency key is currently in progress",
            ));
        }

        tx.commit().map_err(|err| {
            AppError::internal(format!(
                "failed to commit idempotency reservation transaction: {err}"
            ))
        })?;

        Ok(IdempotencyReservation {
            store: self.clone(),
            principal: principal.clone(),
            operation: operation.to_string(),
            idempotency_key: idempotency_key.to_string(),
        })
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.open_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS idempotency_keys (
                user_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                request_fingerprint TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'completed',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                PRIMARY KEY (user_id, agent_id, operation, idempotency_key)
            );",
        )?;
        ensure_column(
            &conn,
            "idempotency_keys",
            "status",
            "ALTER TABLE idempotency_keys ADD COLUMN status TEXT NOT NULL DEFAULT 'completed'",
        )?;
        ensure_column(
            &conn,
            "idempotency_keys",
            "updated_at",
            "ALTER TABLE idempotency_keys ADD COLUMN updated_at INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(
            &conn,
            "idempotency_keys",
            "last_error",
            "ALTER TABLE idempotency_keys ADD COLUMN last_error TEXT",
        )?;
        conn.execute(
            "UPDATE idempotency_keys
             SET status = COALESCE(status, 'completed'),
                 updated_at = CASE
                     WHEN updated_at = 0 THEN created_at
                     ELSE updated_at
                 END",
            [],
        )?;
        Ok(())
    }

    fn transition(
        &self,
        principal: &Principal,
        operation: &str,
        idempotency_key: &str,
        from: IdempotencyStatus,
        to: IdempotencyStatus,
        last_error: Option<&str>,
    ) -> AppResult<()> {
        let conn = self.open_conn().map_err(AppError::from)?;
        let updated = conn
            .execute(
                "UPDATE idempotency_keys
                 SET status = ?5,
                     updated_at = ?6,
                     last_error = ?7
                 WHERE user_id = ?1
                   AND agent_id = ?2
                   AND operation = ?3
                   AND idempotency_key = ?4
                   AND status = ?8",
                params![
                    &principal.user_id,
                    &principal.agent_id,
                    operation,
                    idempotency_key,
                    to.as_str(),
                    now_millis(),
                    last_error,
                    from.as_str(),
                ],
            )
            .map_err(|err| {
                AppError::internal(format!("failed to update idempotency key status: {err}"))
            })?;
        if updated != 1 {
            return Err(AppError::internal(format!(
                "invalid idempotency state transition from {} to {} for operation {operation}",
                from.as_str(),
                to.as_str()
            )));
        }
        Ok(())
    }

    fn open_conn(&self) -> anyhow::Result<Connection> {
        Ok(Connection::open(&self.sqlite_path)?)
    }
}

impl IdempotencyReservation {
    pub fn mark_completed(self) -> AppResult<()> {
        self.store.transition(
            &self.principal,
            &self.operation,
            &self.idempotency_key,
            IdempotencyStatus::InProgress,
            IdempotencyStatus::Completed,
            None,
        )
    }

    pub fn mark_failed(self) -> AppResult<()> {
        self.store.transition(
            &self.principal,
            &self.operation,
            &self.idempotency_key,
            IdempotencyStatus::InProgress,
            IdempotencyStatus::Failed,
            Some("protected operation failed before completion"),
        )
    }
}

#[derive(Clone)]
pub struct JobStore {
    sqlite_path: PathBuf,
}

impl JobStore {
    pub fn new(sqlite_path: PathBuf) -> anyhow::Result<Self> {
        if let Some(parent) = sqlite_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let this = Self { sqlite_path };
        this.init_schema()?;
        Ok(this)
    }

    pub fn enqueue_distill(
        &self,
        req: &EnqueueDistillJobRequest,
    ) -> AppResult<EnqueueDistillJobResponse> {
        validate_non_empty("actor.userId", &req.actor.user_id)?;
        validate_non_empty("actor.agentId", &req.actor.agent_id)?;

        let status = DistillJobStatus::Queued;
        let now = now_millis();
        let job_id = format!("distill_job_{}", Uuid::new_v4().simple());
        let conn = self.open_conn().map_err(AppError::from)?;
        let (source_kind, session_key, session_id) = match &req.source {
            DistillSource::SessionTranscript {
                session_key,
                session_id,
            } => (
                distill_source_kind_to_str(DistillSourceKind::SessionTranscript),
                Some(session_key.as_str()),
                session_id.as_deref(),
            ),
            DistillSource::InlineMessages { .. } => (
                distill_source_kind_to_str(DistillSourceKind::InlineMessages),
                None,
                None,
            ),
        };

        conn.execute(
            "INSERT INTO distill_jobs (
                job_id,
                user_id,
                agent_id,
                session_key,
                session_id,
                mode,
                source_kind,
                status,
                result_summary_json,
                error_json,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9, ?9)",
            params![
                &job_id,
                &req.actor.user_id,
                &req.actor.agent_id,
                session_key,
                session_id,
                distill_mode_to_str(req.mode),
                source_kind,
                distill_status_to_str(status),
                now,
            ],
        )
        .map_err(|err| AppError::internal(format!("failed to enqueue distill job: {err}")))?;

        Ok(EnqueueDistillJobResponse { job_id, status })
    }

    pub fn append_session_transcript(
        &self,
        req: &crate::models::AppendSessionTranscriptRequest,
    ) -> AppResult<crate::models::AppendSessionTranscriptResponse> {
        validate_non_empty("actor.userId", &req.actor.user_id)?;
        validate_non_empty("actor.agentId", &req.actor.agent_id)?;
        if req.items.is_empty() {
            return Err(AppError::invalid_request("items must be non-empty"));
        }

        let mut conn = self.open_conn().map_err(AppError::from)?;
        let tx = conn.transaction().map_err(|err| {
            AppError::internal(format!("failed to open transcript transaction: {err}"))
        })?;
        let next_seq: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM session_transcript_messages
                 WHERE user_id = ?1
                   AND agent_id = ?2
                   AND session_key = ?3
                   AND session_id = ?4",
                params![
                    &req.actor.user_id,
                    &req.actor.agent_id,
                    &req.actor.session_key,
                    &req.actor.session_id,
                ],
                |row| row.get(0),
            )
            .map_err(|err| {
                AppError::internal(format!(
                    "failed to determine next session transcript sequence: {err}"
                ))
            })?;
        let now = now_millis();

        for (offset, item) in req.items.iter().enumerate() {
            tx.execute(
                "INSERT INTO session_transcript_messages (
                    user_id,
                    agent_id,
                    session_key,
                    session_id,
                    seq,
                    role,
                    text,
                    created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &req.actor.user_id,
                    &req.actor.agent_id,
                    &req.actor.session_key,
                    &req.actor.session_id,
                    next_seq + offset as i64 + 1,
                    message_role_to_str(item.role),
                    &item.text,
                    now,
                ],
            )
            .map_err(|err| {
                AppError::internal(format!("failed to insert session transcript row: {err}"))
            })?;
        }

        tx.commit().map_err(|err| {
            AppError::internal(format!("failed to commit session transcript rows: {err}"))
        })?;

        Ok(crate::models::AppendSessionTranscriptResponse {
            appended: req.items.len() as u64,
        })
    }

    fn load_session_transcript(
        &self,
        principal: &Principal,
        session_key: &str,
        session_id: Option<&str>,
        max_messages: Option<u64>,
    ) -> AppResult<Vec<SessionTranscriptStoredMessage>> {
        let conn = self.open_conn().map_err(AppError::from)?;
        let mut sql = String::from(
            "SELECT seq, role, text
             FROM session_transcript_messages
             WHERE user_id = ?1
               AND agent_id = ?2
               AND session_key = ?3",
        );
        if session_id.is_some() {
            sql.push_str(" AND session_id = ?4");
        }
        sql.push_str(" ORDER BY seq ASC");

        let mut rows = Vec::new();
        if let Some(session_id) = session_id {
            let mut stmt = conn.prepare(&sql).map_err(|err| {
                AppError::internal(format!("failed to prepare session transcript query: {err}"))
            })?;
            let mapped = stmt
                .query_map(
                    params![
                        &principal.user_id,
                        &principal.agent_id,
                        session_key,
                        session_id
                    ],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .map_err(|err| {
                    AppError::internal(format!("failed to query session transcript rows: {err}"))
                })?;
            for row in mapped {
                let (message_id, role, text) = row.map_err(|err| {
                    AppError::internal(format!("failed to decode session transcript row: {err}"))
                })?;
                rows.push(SessionTranscriptStoredMessage {
                    message_id: message_id as u64,
                    role: parse_message_role(&role)?,
                    text,
                });
            }
        } else {
            let mut stmt = conn.prepare(&sql).map_err(|err| {
                AppError::internal(format!("failed to prepare session transcript query: {err}"))
            })?;
            let mapped = stmt
                .query_map(
                    params![&principal.user_id, &principal.agent_id, session_key],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .map_err(|err| {
                    AppError::internal(format!("failed to query session transcript rows: {err}"))
                })?;
            for row in mapped {
                let (message_id, role, text) = row.map_err(|err| {
                    AppError::internal(format!("failed to decode session transcript row: {err}"))
                })?;
                rows.push(SessionTranscriptStoredMessage {
                    message_id: message_id as u64,
                    role: parse_message_role(&role)?,
                    text,
                });
            }
        }

        if rows.is_empty() {
            return Err(AppError::invalid_request(
                "session-transcript source has no persisted messages for the requested session",
            ));
        }

        if let Some(limit) = max_messages {
            let limit = limit.max(1).min(10_000) as usize;
            if rows.len() > limit {
                rows.drain(0..rows.len() - limit);
            }
        }

        Ok(rows)
    }

    pub fn get_scoped_distill(
        &self,
        job_id: &str,
        user_id: &str,
        agent_id: &str,
    ) -> AppResult<Option<DistillJobStatusResponse>> {
        let conn = self.open_conn().map_err(AppError::from)?;
        let mut stmt = conn
            .prepare(
                "SELECT
                    user_id,
                    agent_id,
                    mode,
                    source_kind,
                    status,
                    result_summary_json,
                    error_json,
                    created_at,
                    updated_at
                 FROM distill_jobs
                 WHERE job_id = ?1",
            )
            .map_err(|err| AppError::internal(format!("failed to prepare query: {err}")))?;

        let mut rows = stmt
            .query(params![job_id])
            .map_err(|err| AppError::internal(format!("failed to query distill job: {err}")))?;

        let Some(row) = rows
            .next()
            .map_err(|err| AppError::internal(format!("failed to fetch distill job row: {err}")))?
        else {
            return Ok(None);
        };

        let owner_user_id: String = row
            .get(0)
            .map_err(|err| AppError::internal(format!("failed to read owner user_id: {err}")))?;
        let owner_agent_id: String = row
            .get(1)
            .map_err(|err| AppError::internal(format!("failed to read owner agent_id: {err}")))?;

        if owner_user_id != user_id || owner_agent_id != agent_id {
            return Ok(None);
        }

        let mode = parse_distill_mode(
            &row.get::<_, String>(2)
                .map_err(|err| AppError::internal(format!("failed to read mode: {err}")))?,
        )?;
        let source_kind =
            parse_distill_source_kind(&row.get::<_, String>(3).map_err(|err| {
                AppError::internal(format!("failed to read source_kind: {err}"))
            })?)?;
        let status = parse_distill_status(
            &row.get::<_, String>(4)
                .map_err(|err| AppError::internal(format!("failed to read status: {err}")))?,
        )?;
        let result_summary_json: Option<String> = row.get(5).map_err(|err| {
            AppError::internal(format!("failed to read result_summary_json: {err}"))
        })?;
        let error_json: Option<String> = row
            .get(6)
            .map_err(|err| AppError::internal(format!("failed to read error_json: {err}")))?;
        let created_at: i64 = row
            .get(7)
            .map_err(|err| AppError::internal(format!("failed to read created_at: {err}")))?;
        let updated_at: i64 = row
            .get(8)
            .map_err(|err| AppError::internal(format!("failed to read updated_at: {err}")))?;

        let result = result_summary_json
            .as_deref()
            .map(parse_distill_job_result_summary)
            .transpose()?;
        let error = error_json
            .as_deref()
            .map(parse_job_status_error)
            .transpose()?;

        Ok(Some(DistillJobStatusResponse {
            job_id: job_id.to_string(),
            status,
            mode,
            source_kind,
            created_at,
            updated_at,
            result,
            error,
        }))
    }

    pub fn mark_distill_running(&self, job_id: &str) -> AppResult<()> {
        self.update_distill_status(job_id, DistillJobStatus::Running)
    }

    pub fn complete_distill(
        &self,
        job_id: &str,
        summary: &DistillJobResultSummary,
    ) -> AppResult<()> {
        let conn = self.open_conn().map_err(AppError::from)?;
        let updated = conn
            .execute(
                "UPDATE distill_jobs
                 SET status = ?2,
                     result_summary_json = ?3,
                     error_json = NULL,
                     updated_at = ?4
                 WHERE job_id = ?1",
                params![
                    job_id,
                    distill_status_to_str(DistillJobStatus::Completed),
                    serde_json::to_string(summary).map_err(|err| {
                        AppError::internal(format!(
                            "failed to serialize distill result summary: {err}"
                        ))
                    })?,
                    now_millis(),
                ],
            )
            .map_err(|err| AppError::internal(format!("failed to complete distill job: {err}")))?;
        if updated == 0 {
            return Err(AppError::not_found(
                "distill job not found during completion",
            ));
        }
        Ok(())
    }

    pub fn fail_distill(&self, job_id: &str, error: &AppError) -> AppResult<()> {
        let payload = crate::models::JobStatusError {
            code: match error.status() {
                axum::http::StatusCode::BAD_REQUEST => "DISTILL_SOURCE_UNAVAILABLE".to_string(),
                axum::http::StatusCode::SERVICE_UNAVAILABLE => "UPSTREAM_DISTILL_ERROR".to_string(),
                _ => "INTERNAL_ERROR".to_string(),
            },
            message: error.message().to_string(),
            retryable: error.status() == axum::http::StatusCode::SERVICE_UNAVAILABLE,
            details: json!({}),
        };
        let conn = self.open_conn().map_err(AppError::from)?;
        let updated = conn
            .execute(
                "UPDATE distill_jobs
                 SET status = ?2,
                     result_summary_json = NULL,
                     error_json = ?3,
                     updated_at = ?4
                 WHERE job_id = ?1",
                params![
                    job_id,
                    distill_status_to_str(DistillJobStatus::Failed),
                    serde_json::to_string(&payload).map_err(|err| {
                        AppError::internal(format!(
                            "failed to serialize distill error payload: {err}"
                        ))
                    })?,
                    now_millis(),
                ],
            )
            .map_err(|err| AppError::internal(format!("failed to fail distill job: {err}")))?;
        if updated == 0 {
            return Err(AppError::not_found("distill job not found during failure"));
        }
        Ok(())
    }

    pub fn insert_distill_artifacts(
        &self,
        job_id: &str,
        artifacts: &[DistillArtifact],
    ) -> AppResult<()> {
        let mut conn = self.open_conn().map_err(AppError::from)?;
        let tx = conn.transaction().map_err(|err| {
            AppError::internal(format!(
                "failed to start distill artifact transaction: {err}"
            ))
        })?;
        for artifact in artifacts {
            tx.execute(
                "INSERT INTO distill_artifacts (
                    artifact_id,
                    job_id,
                    kind,
                    subtype,
                    category,
                    importance,
                    text,
                    evidence_json,
                    tags_json,
                    persistence_json,
                    created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    &artifact.artifact_id,
                    job_id,
                    distill_artifact_kind_to_str(artifact.kind),
                    artifact.subtype.map(distill_artifact_subtype_to_str),
                    artifact.category.as_str(),
                    artifact.importance,
                    &artifact.text,
                    serde_json::to_string(&artifact.evidence).map_err(|err| {
                        AppError::internal(format!("failed to serialize artifact evidence: {err}"))
                    })?,
                    serde_json::to_string(&artifact.tags).map_err(|err| {
                        AppError::internal(format!("failed to serialize artifact tags: {err}"))
                    })?,
                    artifact
                        .persistence
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(|err| {
                            AppError::internal(format!(
                                "failed to serialize artifact persistence: {err}"
                            ))
                        })?,
                    now_millis(),
                ],
            )
            .map_err(|err| {
                AppError::internal(format!("failed to insert distill artifact: {err}"))
            })?;
        }
        tx.commit().map_err(|err| {
            AppError::internal(format!("failed to commit distill artifacts: {err}"))
        })?;
        Ok(())
    }

    fn update_distill_status(&self, job_id: &str, status: DistillJobStatus) -> AppResult<()> {
        let conn = self.open_conn().map_err(AppError::from)?;
        let updated = conn
            .execute(
                "UPDATE distill_jobs
                 SET status = ?2,
                     updated_at = ?3
                 WHERE job_id = ?1",
                params![job_id, distill_status_to_str(status), now_millis()],
            )
            .map_err(|err| {
                AppError::internal(format!("failed to update distill job status: {err}"))
            })?;
        if updated == 0 {
            return Err(AppError::not_found(
                "distill job not found during status update",
            ));
        }
        Ok(())
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.open_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS distill_jobs (
                job_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                session_key TEXT,
                session_id TEXT,
                mode TEXT NOT NULL,
                source_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                result_summary_json TEXT,
                error_json TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS distill_artifacts (
                artifact_id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                subtype TEXT,
                category TEXT NOT NULL,
                importance REAL NOT NULL,
                text TEXT NOT NULL,
                evidence_json TEXT NOT NULL,
                tags_json TEXT NOT NULL,
                persistence_json TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(job_id) REFERENCES distill_jobs(job_id)
            );
            CREATE TABLE IF NOT EXISTS session_transcript_messages (
                row_id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                session_key TEXT NOT NULL,
                session_id TEXT NOT NULL,
                seq INTEGER NOT NULL,
                role TEXT NOT NULL,
                text TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );",
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session_transcript_lookup
             ON session_transcript_messages (user_id, agent_id, session_key, session_id, seq)",
            [],
        )?;
        ensure_column(
            &conn,
            "distill_artifacts",
            "subtype",
            "ALTER TABLE distill_artifacts ADD COLUMN subtype TEXT",
        )?;
        Ok(())
    }

    fn open_conn(&self) -> anyhow::Result<Connection> {
        Ok(Connection::open(&self.sqlite_path)?)
    }
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> anyhow::Result<()> {
    if table_has_column(conn, table, column)? {
        return Ok(());
    }
    conn.execute(alter_sql, [])?;
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn rows_to_record_batch(rows: &[MemoryRow], vector_dimensions: usize) -> AppResult<RecordBatch> {
    let schema = memory_table_schema(vector_dimensions);
    let id_values: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    let principal_user_values: Vec<&str> = rows
        .iter()
        .map(|row| row.principal_user_id.as_str())
        .collect();
    let principal_agent_values: Vec<&str> = rows
        .iter()
        .map(|row| row.principal_agent_id.as_str())
        .collect();
    let text_values: Vec<&str> = rows.iter().map(|row| row.text.as_str()).collect();
    let category_values: Vec<&str> = rows.iter().map(|row| row.category.as_str()).collect();
    let importance_values: Vec<f64> = rows.iter().map(|row| row.importance).collect();
    let scope_values: Vec<&str> = rows.iter().map(|row| row.scope.as_str()).collect();
    let created_values: Vec<i64> = rows.iter().map(|row| row.created_at).collect();
    let updated_values: Vec<i64> = rows.iter().map(|row| row.updated_at).collect();
    let access_count_values: Vec<i64> = rows
        .iter()
        .map(|row| clamp_access_count(row.access_count))
        .collect();
    let last_accessed_at_values: Vec<i64> =
        rows.iter().map(|row| row.last_accessed_at.max(0)).collect();
    let reflection_kind_values: Vec<Option<&str>> = rows
        .iter()
        .map(|row| row.reflection_kind.map(reflection_kind_to_str))
        .collect();
    let strict_key_values: Vec<Option<&str>> =
        rows.iter().map(|row| row.strict_key.as_deref()).collect();
    let mut vector_values: Vec<Option<Vec<Option<f32>>>> = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(vector) = &row.vector {
            if vector.len() != vector_dimensions {
                return Err(AppError::internal(format!(
                    "vector dimension mismatch while writing row {}: expected {}, got {}",
                    row.id,
                    vector_dimensions,
                    vector.len()
                )));
            }
            vector_values.push(Some(vector.iter().map(|value| Some(*value)).collect()));
        } else {
            vector_values.push(None);
        }
    }
    let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
        vector_values,
        vector_dimensions as i32,
    );

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(id_values)),
        Arc::new(StringArray::from(principal_user_values)),
        Arc::new(StringArray::from(principal_agent_values)),
        Arc::new(StringArray::from(text_values)),
        Arc::new(vector_array),
        Arc::new(StringArray::from(category_values)),
        Arc::new(Float64Array::from(importance_values)),
        Arc::new(StringArray::from(scope_values)),
        Arc::new(Int64Array::from(created_values)),
        Arc::new(Int64Array::from(updated_values)),
        Arc::new(Int64Array::from(access_count_values)),
        Arc::new(Int64Array::from(last_accessed_at_values)),
        Arc::new(StringArray::from(reflection_kind_values)),
        Arc::new(StringArray::from(strict_key_values)),
    ];

    RecordBatch::try_new(schema, columns)
        .map_err(|err| AppError::internal(format!("failed to build Arrow record batch: {err}")))
}

fn rows_from_batches(batches: &[RecordBatch]) -> AppResult<Vec<MemoryRow>> {
    let mut rows = Vec::new();

    for batch in batches {
        let id_idx = schema_index(batch, "id")?;
        let principal_user_idx = schema_index(batch, "principal_user_id")?;
        let principal_agent_idx = schema_index(batch, "principal_agent_id")?;
        let text_idx = schema_index(batch, "text")?;
        let vector_idx = schema_index_optional(batch, "vector");
        let category_idx = schema_index(batch, "category")?;
        let importance_idx = schema_index(batch, "importance")?;
        let scope_idx = schema_index(batch, "scope")?;
        let created_idx = schema_index(batch, "created_at")?;
        let updated_idx = schema_index(batch, "updated_at")?;
        let access_count_idx = schema_index_optional(batch, "access_count");
        let last_accessed_at_idx = schema_index_optional(batch, "last_accessed_at");
        let reflection_kind_idx = schema_index(batch, "reflection_kind")?;
        let strict_key_idx = schema_index(batch, "strict_key")?;

        let id_col = as_string_array(batch.column(id_idx), "id")?;
        let principal_user_col =
            as_string_array(batch.column(principal_user_idx), "principal_user_id")?;
        let principal_agent_col =
            as_string_array(batch.column(principal_agent_idx), "principal_agent_id")?;
        let text_col = as_string_array(batch.column(text_idx), "text")?;
        let vector_col = match vector_idx {
            Some(idx) => Some(as_fixed_size_list_array(batch.column(idx), "vector")?),
            None => None,
        };
        let category_col = as_string_array(batch.column(category_idx), "category")?;
        let importance_col = as_f64_array(batch.column(importance_idx), "importance")?;
        let scope_col = as_string_array(batch.column(scope_idx), "scope")?;
        let created_col = as_i64_array(batch.column(created_idx), "created_at")?;
        let updated_col = as_i64_array(batch.column(updated_idx), "updated_at")?;
        let access_count_col = match access_count_idx {
            Some(idx) => Some(as_i64_array(batch.column(idx), "access_count")?),
            None => None,
        };
        let last_accessed_at_col = match last_accessed_at_idx {
            Some(idx) => Some(as_i64_array(batch.column(idx), "last_accessed_at")?),
            None => None,
        };
        let reflection_kind_col =
            as_string_array(batch.column(reflection_kind_idx), "reflection_kind")?;
        let strict_key_col = as_string_array(batch.column(strict_key_idx), "strict_key")?;

        for row_idx in 0..batch.num_rows() {
            let category = parse_category(category_col.value(row_idx))?;
            let reflection_kind = if reflection_kind_col.is_null(row_idx) {
                None
            } else {
                Some(parse_reflection_kind(reflection_kind_col.value(row_idx))?)
            };
            let strict_key = if strict_key_col.is_null(row_idx) {
                None
            } else {
                Some(strict_key_col.value(row_idx).to_string())
            };

            rows.push(MemoryRow {
                id: id_col.value(row_idx).to_string(),
                principal_user_id: principal_user_col.value(row_idx).to_string(),
                principal_agent_id: principal_agent_col.value(row_idx).to_string(),
                text: text_col.value(row_idx).to_string(),
                category,
                importance: importance_col.value(row_idx),
                scope: scope_col.value(row_idx).to_string(),
                created_at: created_col.value(row_idx),
                updated_at: updated_col.value(row_idx),
                access_count: access_count_col
                    .map(|col| clamp_access_count(col.value(row_idx)))
                    .unwrap_or(0),
                last_accessed_at: last_accessed_at_col
                    .map(|col| col.value(row_idx).max(0))
                    .unwrap_or(0),
                reflection_kind,
                strict_key,
                vector: vector_col
                    .as_ref()
                    .map(|column| vector_from_list_column(column, row_idx))
                    .transpose()?
                    .flatten(),
            });
        }
    }

    Ok(rows)
}

fn schema_index(batch: &RecordBatch, column: &str) -> AppResult<usize> {
    batch.schema().index_of(column).map_err(|err| {
        AppError::internal(format!("missing '{column}' column in LanceDB batch: {err}"))
    })
}

fn schema_index_optional(batch: &RecordBatch, column: &str) -> Option<usize> {
    batch.schema().index_of(column).ok()
}

fn as_string_array<'a>(array: &'a ArrayRef, column: &str) -> AppResult<&'a StringArray> {
    array.as_any().downcast_ref::<StringArray>().ok_or_else(|| {
        AppError::internal(format!(
            "column '{column}' has unexpected type; expected Utf8 string"
        ))
    })
}

fn as_f64_array<'a>(array: &'a ArrayRef, column: &str) -> AppResult<&'a Float64Array> {
    array
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| {
            AppError::internal(format!(
                "column '{column}' has unexpected type; expected Float64"
            ))
        })
}

fn as_i64_array<'a>(array: &'a ArrayRef, column: &str) -> AppResult<&'a Int64Array> {
    array.as_any().downcast_ref::<Int64Array>().ok_or_else(|| {
        AppError::internal(format!(
            "column '{column}' has unexpected type; expected Int64"
        ))
    })
}

fn as_fixed_size_list_array<'a>(
    array: &'a ArrayRef,
    column: &str,
) -> AppResult<&'a FixedSizeListArray> {
    array
        .as_any()
        .downcast_ref::<FixedSizeListArray>()
        .ok_or_else(|| {
            AppError::internal(format!(
                "column '{column}' has unexpected type; expected FixedSizeList<Float32>"
            ))
        })
}

fn vector_from_list_column(
    column: &FixedSizeListArray,
    row_idx: usize,
) -> AppResult<Option<Vec<f32>>> {
    if column.is_null(row_idx) {
        return Ok(None);
    }
    let values = column.value(row_idx);
    if let Some(float_col) = values.as_any().downcast_ref::<Float32Array>() {
        let mut vector = Vec::with_capacity(float_col.len());
        for value_idx in 0..float_col.len() {
            if float_col.is_null(value_idx) {
                return Err(AppError::internal(
                    "vector column contained null item in non-null embedding",
                ));
            }
            vector.push(float_col.value(value_idx));
        }
        return Ok(Some(vector));
    }
    Err(AppError::internal(
        "vector column item type is not Float32 as expected",
    ))
}

fn prepare_inline_distill_messages(
    messages: &[crate::models::CaptureItem],
) -> Vec<DistillPreparedMessage> {
    messages
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| prepare_distill_message((idx + 1) as u64, item.role, &item.text))
        .collect()
}

fn prepare_stored_session_transcript_messages(
    messages: &[SessionTranscriptStoredMessage],
) -> Vec<DistillPreparedMessage> {
    messages
        .iter()
        .filter(|message| matches!(message.role, MessageRole::User | MessageRole::Assistant))
        .filter_map(|message| {
            prepare_distill_message(message.message_id, message.role, &message.text)
        })
        .collect()
}

fn prepare_distill_message(
    message_id: u64,
    role: MessageRole,
    text: &str,
) -> Option<DistillPreparedMessage> {
    let cleaned = clean_distill_text(text);
    if cleaned.is_empty() || is_noise_distill_text(&cleaned) {
        return None;
    }
    Some(DistillPreparedMessage {
        message_id,
        role,
        text: cleaned,
    })
}

fn clean_distill_text(text: &str) -> String {
    let mut cleaned = text.trim().to_string();
    if cleaned.is_empty() {
        return String::new();
    }

    if cleaned.contains("<relevant-memories>") {
        while let Some(start) = cleaned.find("<relevant-memories>") {
            if let Some(end_rel) = cleaned[start..].find("</relevant-memories>") {
                let end = start + end_rel + "</relevant-memories>".len();
                cleaned.replace_range(start..end, "");
            } else {
                cleaned.truncate(start);
                break;
            }
        }
    }

    for prefix in [
        "Conversation info (untrusted metadata):",
        "Replied message (untrusted, for context):",
    ] {
        if let Some(rest) = cleaned.strip_prefix(prefix) {
            cleaned = rest.trim_start().to_string();
        }
    }

    cleaned = strip_json_fences(&cleaned);
    cleaned = collapse_blank_lines(&cleaned);
    cleaned.trim().to_string()
}

fn strip_json_fences(input: &str) -> String {
    let mut out = String::new();
    let mut remaining = input;
    loop {
        let Some(start) = remaining.find("```json") else {
            out.push_str(remaining);
            break;
        };
        out.push_str(&remaining[..start]);
        let after_start = &remaining[start + "```json".len()..];
        let Some(end) = after_start.find("```") else {
            break;
        };
        remaining = &after_start[end + "```".len()..];
    }
    out
}

fn collapse_blank_lines(input: &str) -> String {
    let mut out = String::new();
    let mut blank_run = 0_u8;
    for line in input.lines() {
        if line.trim().is_empty() {
            blank_run = blank_run.saturating_add(1);
            if blank_run <= 1 && !out.is_empty() {
                out.push('\n');
            }
            continue;
        }
        blank_run = 0;
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(line.trim_end());
    }
    out
}

fn is_noise_distill_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    if trimmed.starts_with('/') {
        return true;
    }
    if trimmed.starts_with("✅ New session started") || trimmed.starts_with("NO_REPLY") {
        return true;
    }
    let lowered = trimmed.to_lowercase();
    if lowered.contains("[queued messages while agent was busy]")
        || lowered.contains("you are running a boot check")
        || lowered.contains("boot.md — gateway startup health check")
        || lowered.contains("read heartbeat.md")
        || lowered.contains("[claude_code_done]")
        || lowered.contains("claude_code_done")
    {
        return true;
    }
    if trimmed.len() > 2000 {
        return true;
    }
    if trimmed.starts_with("```") && trimmed.ends_with("```") {
        return true;
    }
    false
}

fn build_distill_artifacts(
    job_id: &str,
    prepared: &[DistillPreparedMessage],
    mode: DistillMode,
    options: &crate::models::DistillOptions,
    max_artifacts: usize,
) -> Vec<DistillArtifact> {
    reduce_distill_candidates(
        build_distill_candidates(prepared, mode, options),
        max_artifacts,
    )
    .into_iter()
    .map(|candidate| DistillArtifact {
        artifact_id: format!("art_{}", Uuid::new_v4().simple()),
        job_id: job_id.to_string(),
        kind: candidate.kind,
        subtype: candidate.subtype,
        category: candidate.category,
        importance: candidate.importance,
        text: candidate.text,
        evidence: candidate.evidence,
        tags: candidate.tags,
        persistence: None,
    })
    .collect()
}

fn build_distill_candidates(
    prepared: &[DistillPreparedMessage],
    mode: DistillMode,
    options: &crate::models::DistillOptions,
) -> Vec<DistillCandidate> {
    let chunk_chars = options.chunk_chars.unwrap_or(12_000).clamp(400, 24_000) as usize;
    let overlap_messages = options.chunk_overlap_messages.unwrap_or(10).min(32) as usize;
    let mut candidates = Vec::new();
    for window in build_distill_windows(prepared, chunk_chars, overlap_messages) {
        for span in build_distill_spans(window, mode) {
            if let Some(candidate) = build_distill_candidate(span, mode) {
                candidates.push(candidate);
            }
        }
    }
    candidates
}

fn build_distill_windows<'a>(
    prepared: &'a [DistillPreparedMessage],
    chunk_chars: usize,
    overlap_messages: usize,
) -> Vec<&'a [DistillPreparedMessage]> {
    if prepared.is_empty() {
        return Vec::new();
    }

    let mut windows = Vec::new();
    let mut start = 0usize;
    while start < prepared.len() && windows.len() < 200 {
        let mut end = start;
        let mut size = 0usize;
        while end < prepared.len() {
            let next_size = distill_message_window_len(&prepared[end]);
            if end > start && size + next_size > chunk_chars {
                break;
            }
            size += next_size;
            end += 1;
        }
        if end == start {
            end += 1;
        }
        windows.push(&prepared[start..end]);
        if end >= prepared.len() {
            break;
        }
        let retained_overlap = overlap_messages.min(end.saturating_sub(start).saturating_sub(1));
        let next_start = end.saturating_sub(retained_overlap);
        start = if next_start <= start { end } else { next_start };
    }
    windows
}

fn distill_message_window_len(message: &DistillPreparedMessage) -> usize {
    message.text.chars().count() + 16
}

fn build_distill_spans(
    window: &[DistillPreparedMessage],
    mode: DistillMode,
) -> Vec<&[DistillPreparedMessage]> {
    if window.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut start = 0usize;
    while start < window.len() {
        let mut end = start + 1;
        while end < window.len()
            && end - start < 4
            && should_extend_distill_span(&window[start..end], &window[end], mode)
        {
            end += 1;
        }
        spans.push(&window[start..end]);
        start = end;
    }
    spans
}

fn should_extend_distill_span(
    current: &[DistillPreparedMessage],
    next: &DistillPreparedMessage,
    mode: DistillMode,
) -> bool {
    let current_text = current
        .iter()
        .map(|message| message.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let combined_len = current_text.chars().count() + next.text.chars().count();
    if combined_len > 1_400 {
        return false;
    }
    let current_tags = extract_distill_tags(&current_text);
    let next_tags = extract_distill_tags(&next.text);
    let shared_tags = current_tags
        .iter()
        .filter(|tag| next_tags.iter().any(|candidate| candidate == *tag))
        .count();
    let current_signals = detect_distill_signal_count(&current_text);
    let next_signals = detect_distill_signal_count(&next.text);
    let role_bridge = current
        .last()
        .map(|message| message.role != next.role)
        .unwrap_or(false);
    if matches!(mode, DistillMode::SessionLessons)
        && session_lessons_prefix_conflicts(&current_text, &next.text)
    {
        return false;
    }
    if matches!(mode, DistillMode::GovernanceCandidates)
        && governance_label_conflicts(&current_text, &next.text)
    {
        return false;
    }
    shared_tags > 0 || (role_bridge && (current_signals > 0 || next_signals > 0))
}

fn build_distill_candidate(
    span: &[DistillPreparedMessage],
    mode: DistillMode,
) -> Option<DistillCandidate> {
    let span_text = span
        .iter()
        .map(|message| message.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let summary = normalize_distill_candidate_text(&summarize_distill_text(&span_text, span, mode));
    let (kind, subtype) = infer_distill_artifact_metadata(mode, &summary);
    let evidence = normalize_distill_evidence(
        span.iter()
            .take(3)
            .map(|message| DistillArtifactEvidence {
                message_ids: vec![message.message_id],
                quote: truncate_for_error(&message.text, DISTILL_MAX_QUOTE_LEN),
            })
            .collect(),
    );
    let mut raw_tags = extract_distill_tags(&span_text);
    if matches!(kind, DistillArtifactKind::GovernanceCandidate) {
        raw_tags.push("governance-candidate".to_string());
    }
    if let Some(subtype) = subtype {
        raw_tags.push(distill_artifact_subtype_to_str(subtype).to_string());
    }
    let tags = normalize_distill_tags(raw_tags);
    let mut candidate = DistillCandidate {
        kind,
        subtype,
        category: normalize_distill_category(infer_distill_category(&span_text, mode)),
        importance: normalize_distill_importance(infer_distill_importance(&span_text, span)),
        dedupe_key: build_distill_candidate_dedupe_key(kind, subtype, &summary),
        text: summary,
        evidence,
        tags,
        score: 0.0,
    };
    if !should_keep_distill_candidate(&candidate) {
        return None;
    }
    candidate.score = score_distill_candidate(&candidate);
    Some(candidate)
}

fn reduce_distill_candidates(
    candidates: Vec<DistillCandidate>,
    max_artifacts: usize,
) -> Vec<DistillCandidate> {
    let mut reduced: Vec<DistillCandidate> = Vec::new();
    for candidate in candidates {
        if candidate.dedupe_key.is_empty() {
            continue;
        }
        if let Some(existing) = reduced
            .iter_mut()
            .find(|existing| should_merge_distill_candidate(existing, &candidate))
        {
            merge_distill_candidate(existing, candidate);
            continue;
        }
        reduced.push(candidate);
    }
    for candidate in &mut reduced {
        candidate.score = score_distill_candidate(candidate);
    }
    reduced.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.importance
                    .partial_cmp(&a.importance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.text.cmp(&b.text))
    });
    reduced.truncate(max_artifacts);
    reduced
}

fn normalize_distill_candidate_text(text: &str) -> String {
    truncate_for_error(text.trim(), 480)
}

fn normalize_distill_candidate_key(text: &str) -> String {
    normalize_recall_text(text)
}

fn build_distill_candidate_dedupe_key(
    kind: DistillArtifactKind,
    subtype: Option<DistillArtifactSubtype>,
    text: &str,
) -> String {
    let base = normalize_distill_candidate_key(text);
    match (kind, subtype) {
        (_, Some(subtype)) => format!("{}::{base}", distill_artifact_subtype_to_str(subtype)),
        (DistillArtifactKind::GovernanceCandidate, None) => {
            format!("governance-candidate::{base}")
        }
        _ => base,
    }
}

fn normalize_distill_category(category: Category) -> Category {
    match category {
        Category::Preference | Category::Fact | Category::Decision | Category::Other => category,
        _ => Category::Other,
    }
}

fn normalize_distill_importance(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.7;
    }
    value.clamp(0.0, 0.95)
}

fn normalize_distill_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for tag in tags {
        let normalized_tag = normalize_recall_text(&tag);
        if normalized_tag.len() < 2 || !seen.insert(normalized_tag.clone()) {
            continue;
        }
        normalized.push(normalized_tag);
        if normalized.len() >= 8 {
            break;
        }
    }
    normalized
}

fn normalize_distill_evidence(
    evidence: Vec<DistillArtifactEvidence>,
) -> Vec<DistillArtifactEvidence> {
    let mut normalized = Vec::new();
    for row in evidence {
        let mut message_ids = row.message_ids;
        message_ids.sort_unstable();
        message_ids.dedup();
        let quote = truncate_for_error(row.quote.trim(), DISTILL_MAX_QUOTE_LEN);
        if message_ids.is_empty() || quote.is_empty() {
            continue;
        }
        normalized.push(DistillArtifactEvidence { message_ids, quote });
    }
    normalized
}

fn should_merge_distill_candidate(
    existing: &DistillCandidate,
    incoming: &DistillCandidate,
) -> bool {
    (existing.kind == incoming.kind && existing.subtype == incoming.subtype)
        && (existing.dedupe_key == incoming.dedupe_key
            || (existing.category == incoming.category
                && distill_evidence_overlaps(&existing.evidence, &incoming.evidence)
                && distill_tag_overlap(&existing.tags, &incoming.tags) > 0))
}

fn merge_distill_candidate(target: &mut DistillCandidate, incoming: DistillCandidate) {
    target.importance = target.importance.max(incoming.importance);
    target.text = select_preferred_distill_text(&target.text, &incoming.text);
    target.tags = normalize_distill_tags(
        target
            .tags
            .iter()
            .cloned()
            .chain(incoming.tags.iter().cloned())
            .collect(),
    );
    target.evidence = normalize_distill_evidence(
        target
            .evidence
            .iter()
            .cloned()
            .chain(incoming.evidence.iter().cloned())
            .collect(),
    );
    target.dedupe_key =
        build_distill_candidate_dedupe_key(target.kind, target.subtype, &target.text);
}

fn select_preferred_distill_text(left: &str, right: &str) -> String {
    let left_score = detect_distill_signal_count(left) * 10 + left.len().min(240);
    let right_score = detect_distill_signal_count(right) * 10 + right.len().min(240);
    if right_score > left_score {
        right.to_string()
    } else {
        left.to_string()
    }
}

fn distill_evidence_overlaps(
    left: &[DistillArtifactEvidence],
    right: &[DistillArtifactEvidence],
) -> bool {
    left.iter().any(|left_row| {
        right.iter().any(|right_row| {
            left_row.message_ids.iter().any(|message_id| {
                right_row
                    .message_ids
                    .iter()
                    .any(|candidate| candidate == message_id)
            })
        })
    })
}

fn distill_tag_overlap(left: &[String], right: &[String]) -> usize {
    left.iter()
        .filter(|tag| right.iter().any(|candidate| candidate == *tag))
        .count()
}

fn should_keep_distill_candidate(candidate: &DistillCandidate) -> bool {
    if candidate.evidence.is_empty() {
        return false;
    }
    let normalized = normalize_distill_candidate_key(&candidate.text);
    if normalized.chars().count() < 20 {
        return false;
    }
    if contains_vague_advice(&normalized) && !contains_distill_structure_signal(&normalized) {
        return false;
    }
    true
}

fn contains_vague_advice(text: &str) -> bool {
    ["be careful", "best practice", "should", "建议", "注意"]
        .iter()
        .any(|pattern| text.contains(pattern))
}

fn contains_distill_structure_signal(text: &str) -> bool {
    [
        "cause",
        "fix",
        "prevention",
        "trigger",
        "action",
        "decision principle",
        "stable decision",
        "durable practice",
        "follow-up focus",
        "next-turn guidance",
        "governance candidate",
        "worth promoting",
        "skill extraction candidate",
        "agents/soul/tools promotion candidate",
        "原因",
        "修复",
        "预防",
        "触发",
        "动作",
    ]
    .iter()
    .any(|pattern| text.contains(pattern))
}

fn score_distill_candidate(candidate: &DistillCandidate) -> f64 {
    let normalized = normalize_distill_candidate_key(&candidate.text);
    let mut score = candidate.importance;
    if normalized.contains("pitfall")
        || normalized.contains("cause")
        || normalized.contains("fix")
        || normalized.contains("prevention")
        || normalized.contains("原因")
        || normalized.contains("修复")
        || normalized.contains("预防")
    {
        score += 2.0;
    }
    if normalized.contains("decision principle")
        || normalized.contains("trigger")
        || normalized.contains("action")
        || normalized.contains("stable decision")
        || normalized.contains("durable practice")
        || normalized.contains("触发")
        || normalized.contains("动作")
    {
        score += 2.0;
    }
    if normalized.contains("follow-up focus")
        || normalized.contains("next-turn guidance")
        || normalized.contains("worth promoting")
        || normalized.contains("skill extraction candidate")
        || normalized.contains("agents/soul/tools promotion candidate")
    {
        score += 1.5;
    }
    if normalized.contains("openclaw")
        || normalized.contains("docker")
        || normalized.contains("systemd")
        || normalized.contains("ssh")
        || normalized.contains("git")
        || normalized.contains("api")
        || normalized.contains("json")
        || normalized.contains("yaml")
        || normalized.contains("config")
        || normalized.contains("mosdns")
        || normalized.contains("rclone")
        || normalized.contains("proxy")
    {
        score += 1.0;
    }
    if candidate.subtype.is_some() {
        score += 0.25;
    }
    if candidate.text.chars().count() < 120 {
        score += 0.5;
    }
    if !candidate.evidence.is_empty() {
        score += 1.0;
    }
    if candidate.evidence.len() >= 2 {
        score += 0.5;
    }
    score
}

fn summarize_distill_text(
    text: &str,
    span: &[DistillPreparedMessage],
    mode: DistillMode,
) -> String {
    let sentences = distill_sentences(text);
    let primary = select_primary_distill_sentence(&sentences);
    let cause = select_signal_sentence(&sentences, &["cause", "because", "root cause", "原因"]);
    let fix = select_signal_sentence(
        &sentences,
        &[
            "fix", "restart", "disable", "enable", "rollback", "migrate", "修复",
        ],
    );
    let prevention = select_signal_sentence(&sentences, &["prevention", "avoid", "guard", "预防"]);
    let trigger = select_signal_sentence(
        &sentences,
        &["trigger", "action", "decision", "must", "动作", "触发"],
    );
    let stable_decision = select_signal_sentence(
        &sentences,
        &[
            "decision",
            "decide",
            "default",
            "prefer",
            "standardize",
            "must",
            "keep",
        ],
    );
    let durable_practice = select_signal_sentence(
        &sentences,
        &[
            "durable practice",
            "always",
            "every time",
            "baseline",
            "guardrail",
            "keep",
            "verify",
        ],
    );
    let follow_up = select_signal_sentence(
        &sentences,
        &[
            "follow up",
            "follow-up",
            "open loop",
            "pending",
            "still need",
            "investigate",
            "confirm",
            "audit",
        ],
    );
    let next_turn = select_signal_sentence(
        &sentences,
        &[
            "next turn",
            "next-turn",
            "next step",
            "ask the user",
            "clarify",
            "request",
            "before proceeding",
        ],
    );

    let mut summary = match mode {
        DistillMode::SessionLessons => {
            let prefix = select_session_lessons_prefix_with_evidence(text, span);
            let mut parts = Vec::new();
            match prefix {
                "Follow-up focus" => {
                    let focus = follow_up
                        .or(prevention)
                        .or(trigger.clone())
                        .or(primary.clone())
                        .unwrap_or_else(|| truncate_for_error(text, 360));
                    format!("{prefix}: {}", compact_distill_signal_clause(&focus))
                }
                "Next-turn guidance" => {
                    let guidance = next_turn
                        .or(trigger.clone())
                        .or(primary.clone())
                        .unwrap_or_else(|| truncate_for_error(text, 360));
                    format!("{prefix}: {}", compact_distill_signal_clause(&guidance))
                }
                "Durable practice" => {
                    let practice = durable_practice
                        .or(prevention.clone())
                        .or(primary.clone())
                        .unwrap_or_else(|| truncate_for_error(text, 360));
                    parts.push(compact_distill_signal_clause(&practice));
                    if let Some(cause) = cause {
                        parts.push(format!("Cause: {}", compact_distill_signal_clause(&cause)));
                    }
                    if let Some(fix) = fix {
                        parts.push(format!("Fix: {}", compact_distill_signal_clause(&fix)));
                    }
                    if let Some(prevention) = prevention {
                        parts.push(format!(
                            "Prevention: {}",
                            compact_distill_signal_clause(&prevention)
                        ));
                    }
                    format!("{prefix}: {}", parts.join(" "))
                }
                "Stable decision" => {
                    let decision = stable_decision
                        .or(trigger.clone())
                        .or(primary.clone())
                        .unwrap_or_else(|| truncate_for_error(text, 360));
                    parts.push(compact_distill_signal_clause(&decision));
                    if let Some(cause) = cause {
                        parts.push(format!("Cause: {}", compact_distill_signal_clause(&cause)));
                    }
                    if let Some(fix) = fix {
                        parts.push(format!("Fix: {}", compact_distill_signal_clause(&fix)));
                    }
                    if let Some(prevention) = prevention {
                        parts.push(format!(
                            "Prevention: {}",
                            compact_distill_signal_clause(&prevention)
                        ));
                    }
                    format!("{prefix}: {}", parts.join(" "))
                }
                _ => {
                    if let Some(primary) = primary {
                        parts.push(compact_distill_signal_clause(&primary));
                    }
                    if let Some(cause) = cause {
                        parts.push(format!("Cause: {}", compact_distill_signal_clause(&cause)));
                    }
                    if let Some(fix) = fix {
                        parts.push(format!("Fix: {}", compact_distill_signal_clause(&fix)));
                    }
                    if let Some(prevention) = prevention {
                        parts.push(format!(
                            "Prevention: {}",
                            compact_distill_signal_clause(&prevention)
                        ));
                    }
                    if let Some(trigger) = trigger {
                        parts.push(format!(
                            "Action: {}",
                            compact_distill_signal_clause(&trigger)
                        ));
                    }
                    if parts.is_empty() {
                        parts.push(compact_distill_sentence(&truncate_for_error(text, 360)));
                    }
                    format!("{prefix}: {}", parts.join(" "))
                }
            }
        }
        DistillMode::GovernanceCandidates => {
            let label = select_governance_label(text);
            let body = primary
                .or(stable_decision)
                .or(durable_practice)
                .unwrap_or_else(|| truncate_for_error(text, 360));
            let mut governance_parts =
                vec![format!("{label}: {}", compact_distill_signal_clause(&body))];
            if let Some(trigger) = trigger {
                governance_parts.push(format!("Why: {}", compact_distill_signal_clause(&trigger)));
            } else if let Some(prevention) = prevention {
                governance_parts.push(format!(
                    "Why: {}",
                    compact_distill_signal_clause(&prevention)
                ));
            }
            format!("Governance candidate: {}", governance_parts.join(" "))
        }
    };
    if span.len() > 1 {
        let entities = select_distill_entities(text, 2);
        if !entities.is_empty()
            && !entities
                .iter()
                .all(|entity| summary.to_lowercase().contains(entity))
        {
            summary.push_str(&format!(" Context: {}.", entities.join(", ")));
        }
    }
    truncate_for_error(&summary, 440)
}

fn select_session_lessons_prefix(text: &str) -> &'static str {
    let lowered = text.to_lowercase();
    if contains_any_distill_pattern(
        &lowered,
        &[
            "next turn",
            "next-turn",
            "next step",
            "ask the user",
            "clarify",
            "before proceeding",
        ],
    ) {
        "Next-turn guidance"
    } else if contains_any_distill_pattern(
        &lowered,
        &[
            "follow up",
            "follow-up",
            "open loop",
            "pending",
            "still need",
            "remaining",
            "investigate",
            "audit",
        ],
    ) {
        "Follow-up focus"
    } else if contains_any_distill_pattern(
        &lowered,
        &["durable practice", "always", "every time", "guardrail"],
    ) {
        "Durable practice"
    } else if contains_any_distill_pattern(
        &lowered,
        &[
            "stable decision",
            "decision",
            "decide",
            "default",
            "prefer",
            "standardize",
        ],
    ) {
        "Stable decision"
    } else {
        "Lesson"
    }
}

fn select_session_lessons_prefix_with_evidence(
    text: &str,
    span: &[DistillPreparedMessage],
) -> &'static str {
    let prefix = select_session_lessons_prefix(text);
    match prefix {
        "Stable decision" | "Durable practice"
            if !session_lessons_promotion_has_evidence(prefix, span) =>
        {
            "Lesson"
        }
        _ => prefix,
    }
}

fn session_lessons_promotion_has_evidence(prefix: &str, span: &[DistillPreparedMessage]) -> bool {
    if distinct_distill_message_count(span) < 2 {
        return false;
    }

    let target_patterns = match prefix {
        "Durable practice" => &[
            "durable practice",
            "always",
            "every time",
            "baseline",
            "guardrail",
        ][..],
        "Stable decision" => &[
            "stable decision",
            "decision",
            "decide",
            "default",
            "prefer",
            "standardize",
        ][..],
        _ => return true,
    };

    let target_message_count = count_distill_messages_with_patterns(span, target_patterns);
    if target_message_count >= 2 {
        return true;
    }

    target_message_count >= 1
        && count_distill_messages_with_patterns(
            span,
            &[
                "cause",
                "because",
                "root cause",
                "fix",
                "restart",
                "disable",
                "enable",
                "rollback",
                "migrate",
                "prevention",
                "avoid",
                "guard",
                "verify",
                "原因",
                "修复",
                "预防",
            ],
        ) >= 2
        && count_distill_corroborating_signal_groups(span) >= 2
}

fn distinct_distill_message_count(span: &[DistillPreparedMessage]) -> usize {
    span.iter()
        .map(|message| message.message_id)
        .collect::<HashSet<_>>()
        .len()
}

fn count_distill_messages_with_patterns(
    span: &[DistillPreparedMessage],
    patterns: &[&str],
) -> usize {
    span.iter()
        .filter(|message| {
            let lowered = message.text.to_lowercase();
            contains_any_distill_pattern(&lowered, patterns)
        })
        .count()
}

fn count_distill_corroborating_signal_groups(span: &[DistillPreparedMessage]) -> usize {
    [
        &["cause", "because", "root cause", "原因"][..],
        &[
            "fix", "restart", "disable", "enable", "rollback", "migrate", "修复",
        ][..],
        &["prevention", "avoid", "guard", "verify", "预防"][..],
    ]
    .iter()
    .filter(|patterns| count_distill_messages_with_patterns(span, patterns) > 0)
    .count()
}

fn session_lessons_prefix_conflicts(current_text: &str, next_text: &str) -> bool {
    let current_prefix = select_session_lessons_prefix(current_text);
    let next_prefix = select_session_lessons_prefix(next_text);
    matches!(
        (current_prefix, next_prefix),
        ("Follow-up focus", "Next-turn guidance") | ("Next-turn guidance", "Follow-up focus")
    )
}

fn select_governance_label(text: &str) -> &'static str {
    let lowered = text.to_lowercase();
    if contains_any_distill_pattern(
        &lowered,
        &[
            "skill",
            "playbook",
            "runbook",
            "workflow",
            "extract",
            "automation",
        ],
    ) {
        "Skill extraction candidate"
    } else if contains_any_distill_pattern(
        &lowered,
        &[
            "agents.md",
            "soul.md",
            "tools.md",
            "guardrail",
            "policy",
            "checklist",
            "standard",
        ],
    ) {
        "AGENTS/SOUL/TOOLS promotion candidate"
    } else {
        "Worth promoting"
    }
}

fn governance_label_conflicts(current_text: &str, next_text: &str) -> bool {
    let current_label = select_governance_label(current_text);
    let next_label = select_governance_label(next_text);
    current_label != next_label
        && (current_label != "Worth promoting" || next_label != "Worth promoting")
}

fn infer_distill_artifact_metadata(
    mode: DistillMode,
    summary: &str,
) -> (DistillArtifactKind, Option<DistillArtifactSubtype>) {
    match mode {
        DistillMode::GovernanceCandidates => (DistillArtifactKind::GovernanceCandidate, None),
        DistillMode::SessionLessons => {
            let lowered = summary.to_lowercase();
            if lowered.starts_with("follow-up focus:") {
                (
                    DistillArtifactKind::Lesson,
                    Some(DistillArtifactSubtype::FollowUpFocus),
                )
            } else if lowered.starts_with("next-turn guidance:") {
                (
                    DistillArtifactKind::Lesson,
                    Some(DistillArtifactSubtype::NextTurnGuidance),
                )
            } else {
                (DistillArtifactKind::Lesson, None)
            }
        }
    }
}

fn contains_any_distill_pattern(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn infer_distill_category(text: &str, mode: DistillMode) -> Category {
    let lowered = text.to_lowercase();
    if matches!(mode, DistillMode::GovernanceCandidates) {
        return Category::Decision;
    }
    if lowered.contains("prefer") || lowered.contains("preference") {
        Category::Preference
    } else if lowered.contains("decide")
        || lowered.contains("decision")
        || lowered.contains("should")
        || lowered.contains("must")
        || lowered.contains("trigger")
        || lowered.contains("action")
        || lowered.contains("rollback")
        || lowered.contains("migrate")
        || lowered.contains("follow up")
        || lowered.contains("follow-up")
        || lowered.contains("next turn")
        || lowered.contains("next-turn")
    {
        Category::Decision
    } else if lowered.contains("error")
        || lowered.contains("fix")
        || lowered.contains("cause")
        || lowered.contains("timeout")
        || lowered.contains("incident")
        || lowered.contains("outage")
    {
        Category::Fact
    } else {
        Category::Other
    }
}

fn infer_distill_importance(text: &str, span: &[DistillPreparedMessage]) -> f64 {
    let mut score: f64 = 0.55;
    let lowered = text.to_lowercase();
    if lowered.contains("error")
        || lowered.contains("fix")
        || lowered.contains("timeout")
        || lowered.contains("restart")
    {
        score += 0.15;
    }
    if lowered.contains("because") || lowered.contains("cause") || lowered.contains("prevention") {
        score += 0.1;
    }
    if lowered.contains("trigger") || lowered.contains("action") {
        score += 0.05;
    }
    if lowered.contains("stable decision") || lowered.contains("durable practice") {
        score += 0.1;
    }
    if lowered.contains("follow-up")
        || lowered.contains("follow up")
        || lowered.contains("next turn")
    {
        score += 0.05;
    }
    if lowered.contains("skill") || lowered.contains("agents.md") || lowered.contains("tools.md") {
        score += 0.1;
    }
    if span
        .iter()
        .any(|message| matches!(message.role, MessageRole::Assistant))
    {
        score += 0.05;
    }
    if span.len() >= 2 {
        score += 0.05;
    }
    if span.len() >= 3 {
        score += 0.05;
    }
    if lowered.contains("rollback") || lowered.contains("migrate") || lowered.contains("verify") {
        score += 0.05;
    }
    if text.len() > 180 {
        score += 0.05;
    }
    score.clamp(0.0, 0.95)
}

fn extract_distill_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let normalized = normalize_recall_text(text);
    for entity in select_distill_entities(text, 4) {
        if !tags.iter().any(|existing| existing == &entity) {
            tags.push(entity);
        }
    }
    for token in lexical_tokens(&normalized) {
        if token.len() < 3 {
            continue;
        }
        if tags.iter().any(|existing| existing == &token) {
            continue;
        }
        tags.push(token);
        if tags.len() >= 5 {
            break;
        }
    }
    tags
}

fn distill_sentences(text: &str) -> Vec<String> {
    text.replace('\n', ". ")
        .split_terminator(&['.', '!', '?'][..])
        .map(compact_distill_sentence)
        .filter(|sentence| !sentence.is_empty())
        .collect()
}

fn compact_distill_sentence(input: &str) -> String {
    compact_distill_clause(input)
        .trim_matches(|c: char| c == '.' || c == ',' || c == ';' || c.is_whitespace())
        .to_string()
}

fn compact_distill_clause(input: &str) -> String {
    let mut out = input.trim().to_string();
    for prefix in [
        "conversation info (untrusted metadata):",
        "replied message (untrusted, for context):",
        "i think ",
        "we should ",
        "please ",
        "note that ",
    ] {
        if out.to_lowercase().starts_with(prefix) {
            out = out[prefix.len()..].trim().to_string();
        }
    }
    truncate_for_error(&out, 180)
}

fn compact_distill_signal_clause(input: &str) -> String {
    let mut out = compact_distill_clause(input);
    for prefix in [
        "cause:",
        "fix:",
        "prevention:",
        "stable decision:",
        "durable practice:",
        "open loop:",
        "follow-up:",
        "follow up:",
        "next turn:",
        "next-turn:",
    ] {
        if out.to_lowercase().starts_with(prefix) {
            out = out[prefix.len()..].trim().to_string();
        }
    }
    out
}

fn select_primary_distill_sentence(sentences: &[String]) -> Option<String> {
    sentences
        .iter()
        .max_by_key(|sentence| {
            detect_distill_signal_count(sentence) * 10
                + select_distill_entities(sentence, 3).len()
                + sentence.len().min(160)
        })
        .cloned()
}

fn select_signal_sentence(sentences: &[String], patterns: &[&str]) -> Option<String> {
    sentences
        .iter()
        .find(|sentence| {
            let lowered = sentence.to_lowercase();
            patterns.iter().any(|pattern| lowered.contains(pattern))
        })
        .cloned()
}

fn detect_distill_signal_count(text: &str) -> usize {
    let lowered = text.to_lowercase();
    [
        "pitfall",
        "cause",
        "because",
        "fix",
        "prevention",
        "decision principle",
        "trigger",
        "action",
        "stable decision",
        "durable practice",
        "follow-up",
        "next turn",
        "skill",
        "agents.md",
        "tools.md",
        "rollback",
        "migrate",
        "restart",
        "disable",
        "verify",
        "原因",
        "修复",
        "预防",
        "触发",
        "动作",
    ]
    .iter()
    .filter(|pattern| lowered.contains(**pattern))
    .count()
}

fn select_distill_entities(text: &str, limit: usize) -> Vec<String> {
    let mut selected = Vec::new();
    for token in lexical_tokens(&normalize_recall_text(text)) {
        if token.len() < 3 {
            continue;
        }
        if !is_distill_entity_token(&token) {
            continue;
        }
        if selected.iter().any(|existing| existing == &token) {
            continue;
        }
        selected.push(token);
        if selected.len() >= limit {
            break;
        }
    }
    selected
}

fn is_distill_entity_token(token: &str) -> bool {
    matches!(
        token,
        "openclaw"
            | "docker"
            | "systemd"
            | "mosdns"
            | "rclone"
            | "proxy"
            | "config"
            | "json"
            | "yaml"
            | "sqlite"
            | "lancedb"
            | "backend"
            | "session"
            | "transcript"
            | "api"
            | "http"
            | "https"
            | "ssh"
            | "git"
            | "timeout"
            | "dns"
            | "fuse"
            | "azure"
            | "token"
    )
}

fn parse_distill_status(raw: &str) -> AppResult<DistillJobStatus> {
    match raw {
        "queued" => Ok(DistillJobStatus::Queued),
        "running" => Ok(DistillJobStatus::Running),
        "completed" => Ok(DistillJobStatus::Completed),
        "failed" => Ok(DistillJobStatus::Failed),
        _ => Err(AppError::internal(format!(
            "unknown distill job status persisted: {raw}"
        ))),
    }
}

fn distill_status_to_str(status: DistillJobStatus) -> &'static str {
    match status {
        DistillJobStatus::Queued => "queued",
        DistillJobStatus::Running => "running",
        DistillJobStatus::Completed => "completed",
        DistillJobStatus::Failed => "failed",
    }
}

fn parse_distill_mode(raw: &str) -> AppResult<DistillMode> {
    match raw {
        "session-lessons" => Ok(DistillMode::SessionLessons),
        "governance-candidates" => Ok(DistillMode::GovernanceCandidates),
        _ => Err(AppError::internal(format!(
            "unknown distill mode persisted: {raw}"
        ))),
    }
}

fn distill_mode_to_str(mode: DistillMode) -> &'static str {
    match mode {
        DistillMode::SessionLessons => "session-lessons",
        DistillMode::GovernanceCandidates => "governance-candidates",
    }
}

fn parse_distill_source_kind(raw: &str) -> AppResult<DistillSourceKind> {
    match raw {
        "session-transcript" => Ok(DistillSourceKind::SessionTranscript),
        "inline-messages" => Ok(DistillSourceKind::InlineMessages),
        _ => Err(AppError::internal(format!(
            "unknown distill source kind persisted: {raw}"
        ))),
    }
}

fn distill_source_kind_to_str(kind: DistillSourceKind) -> &'static str {
    match kind {
        DistillSourceKind::SessionTranscript => "session-transcript",
        DistillSourceKind::InlineMessages => "inline-messages",
    }
}

fn parse_distill_job_result_summary(raw: &str) -> AppResult<DistillJobResultSummary> {
    serde_json::from_str(raw)
        .map_err(|err| AppError::internal(format!("invalid distill result_summary_json: {err}")))
}

fn parse_job_status_error(raw: &str) -> AppResult<crate::models::JobStatusError> {
    serde_json::from_str(raw)
        .map_err(|err| AppError::internal(format!("invalid job error_json: {err}")))
}

fn distill_artifact_kind_to_str(kind: DistillArtifactKind) -> &'static str {
    match kind {
        DistillArtifactKind::Lesson => "lesson",
        DistillArtifactKind::GovernanceCandidate => "governance-candidate",
    }
}

fn distill_artifact_subtype_to_str(subtype: DistillArtifactSubtype) -> &'static str {
    match subtype {
        DistillArtifactSubtype::FollowUpFocus => "follow-up-focus",
        DistillArtifactSubtype::NextTurnGuidance => "next-turn-guidance",
    }
}

fn parse_message_role(raw: &str) -> AppResult<MessageRole> {
    match raw {
        "user" => Ok(MessageRole::User),
        "assistant" => Ok(MessageRole::Assistant),
        "system" => Ok(MessageRole::System),
        _ => Err(AppError::internal(format!(
            "unknown session transcript message role persisted: {raw}"
        ))),
    }
}

fn message_role_to_str(role: MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
    }
}

fn parse_category(raw: &str) -> AppResult<Category> {
    match raw {
        "preference" => Ok(Category::Preference),
        "fact" => Ok(Category::Fact),
        "decision" => Ok(Category::Decision),
        "entity" => Ok(Category::Entity),
        "reflection" => Ok(Category::Reflection),
        "other" => Ok(Category::Other),
        _ => Err(AppError::internal(format!(
            "unknown category persisted in LanceDB: {raw}"
        ))),
    }
}

fn parse_reflection_kind(raw: &str) -> AppResult<ReflectionKind> {
    match raw {
        "invariant" => Ok(ReflectionKind::Invariant),
        "derived" => Ok(ReflectionKind::Derived),
        _ => Err(AppError::internal(format!(
            "unknown reflection kind persisted in LanceDB: {raw}"
        ))),
    }
}

fn reflection_kind_to_str(kind: ReflectionKind) -> &'static str {
    match kind {
        ReflectionKind::Invariant => "invariant",
        ReflectionKind::Derived => "derived",
    }
}

fn principal_filter(actor: &Actor) -> String {
    format!(
        "principal_user_id = '{}' AND principal_agent_id = '{}'",
        escape_sql_literal(&actor.user_id),
        escape_sql_literal(&actor.agent_id)
    )
}

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", escape_sql_literal(value))
}

fn sql_optional_string_literal(value: Option<&str>) -> String {
    match value {
        Some(value) => sql_string_literal(value),
        None => "NULL".to_string(),
    }
}

fn sql_optional_f32_list_literal(value: Option<&[f32]>) -> String {
    match value {
        None => "NULL".to_string(),
        Some(values) => {
            let mut out = String::from("[");
            for (idx, value) in values.iter().enumerate() {
                if idx > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("{:.8}", value));
            }
            out.push(']');
            out
        }
    }
}

fn memory_table_schema(vector_dimensions: usize) -> Arc<Schema> {
    let vector_dimensions = vector_dimensions.max(1) as i32;
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("principal_user_id", DataType::Utf8, false),
        Field::new("principal_agent_id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                vector_dimensions,
            ),
            true,
        ),
        Field::new("category", DataType::Utf8, false),
        Field::new("importance", DataType::Float64, false),
        Field::new("scope", DataType::Utf8, false),
        Field::new("created_at", DataType::Int64, false),
        Field::new("updated_at", DataType::Int64, false),
        Field::new("access_count", DataType::Int64, false),
        Field::new("last_accessed_at", DataType::Int64, false),
        Field::new("reflection_kind", DataType::Utf8, true),
        Field::new("strict_key", DataType::Utf8, true),
    ]))
}

fn default_strict_key(row_id: &str) -> String {
    format!("reflection:{row_id}")
}

fn normalize_reflection_fields(row: &mut MemoryRow) {
    if row.category != Category::Reflection {
        row.reflection_kind = None;
        row.strict_key = None;
        return;
    }

    if row.reflection_kind.is_none() {
        row.reflection_kind = Some(ReflectionKind::Invariant);
    }
    if matches!(row.reflection_kind, Some(ReflectionKind::Invariant)) && row.strict_key.is_none() {
        row.strict_key = Some(default_strict_key(&row.id));
    }
    if matches!(row.reflection_kind, Some(ReflectionKind::Derived)) {
        row.strict_key = None;
    }
}

fn to_list_row(row: MemoryRow) -> ListRow {
    ListRow {
        id: row.id,
        text: row.text,
        category: row.category,
        scope: row.scope,
        metadata: RowMetadata {
            created_at: row.created_at,
            updated_at: row.updated_at,
        },
    }
}

fn to_mutation_result(row: &MemoryRow, action: MemoryAction) -> MemoryMutationResult {
    MemoryMutationResult {
        id: row.id.clone(),
        action,
        text: row.text.clone(),
        category: row.category,
        importance: row.importance,
        scope: row.scope.clone(),
    }
}

fn normalize_recall_text(text: &str) -> String {
    let lowered = text.trim().to_lowercase();
    let mut normalized = String::with_capacity(lowered.len());
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() || is_cjk(ch) {
            normalized.push(ch);
        } else if ch.is_whitespace() {
            normalized.push(' ');
        } else {
            normalized.push(' ');
        }
    }
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct QueryExpansionRule {
    en_triggers: &'static [&'static str],
    expansions: &'static [&'static str],
}

const QUERY_EXPANSION_MAX_TERMS: usize = 5;
const QUERY_EXPANSION_RULES: &[QueryExpansionRule] = &[
    QueryExpansionRule {
        en_triggers: &["crashed", "shutdown", "failure"],
        expansions: &["crash", "error", "failed"],
    },
    QueryExpansionRule {
        en_triggers: &["hung", "frozen", "stuck"],
        expansions: &["timeout", "unresponsive", "hang"],
    },
    QueryExpansionRule {
        en_triggers: &["config", "configuration", "settings"],
        expansions: &["config", "settings", "configuration"],
    },
    QueryExpansionRule {
        en_triggers: &["deploy", "deployment", "release"],
        expansions: &["deploy", "release", "rollout"],
    },
    QueryExpansionRule {
        en_triggers: &["search", "retrieval", "memory"],
        expansions: &["search", "retrieval", "index", "memory"],
    },
];

const NOISE_SUBSTRING_PATTERNS: &[&str] = &[
    "i don't have any information",
    "i do not have any information",
    "i don't recall",
    "i do not recall",
    "i don't remember",
    "i do not remember",
    "no relevant memories found",
    "i wasn't able to find",
    "do you remember",
    "did i tell",
    "what did i tell",
];

const NOISE_EXACT_PATTERNS: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "fresh session",
    "new session",
    "heartbeat",
    "ok",
    "okay",
    "thanks",
    "thank you",
];

const NOISE_SHORT_PREFIX_PATTERNS: &[&str] = &["ok", "okay", "thanks", "thank you", "got it"];

fn is_cjk(ch: char) -> bool {
    let code = ch as u32;
    (0x4E00..=0x9FFF).contains(&code)
        || (0x3400..=0x4DBF).contains(&code)
        || (0x3040..=0x30FF).contains(&code)
        || (0xAC00..=0xD7AF).contains(&code)
}

fn lexical_tokens(normalized_text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut ascii_buf = String::new();
    let cjk_chars: Vec<char> = normalized_text.chars().filter(|ch| is_cjk(*ch)).collect();

    for ch in normalized_text.chars() {
        if ch.is_ascii_alphanumeric() {
            ascii_buf.push(ch);
            continue;
        }

        if !ascii_buf.is_empty() {
            tokens.push(ascii_buf.clone());
            ascii_buf.clear();
        }

        if is_cjk(ch) {
            tokens.push(ch.to_string());
        }
    }

    if !ascii_buf.is_empty() {
        tokens.push(ascii_buf);
    }

    for pair in cjk_chars.windows(2) {
        tokens.push(format!("{}{}", pair[0], pair[1]));
    }

    tokens
}

fn expand_query_terms(query: &str) -> String {
    let trimmed = query.trim();
    if trimmed.chars().count() < 2 {
        return trimmed.to_string();
    }

    let normalized_query = normalize_recall_text(trimmed);
    let query_tokens = lexical_tokens(&normalized_query);
    let mut seen = HashSet::new();
    let mut additions = Vec::new();

    for rule in QUERY_EXPANSION_RULES {
        let matched = rule.en_triggers.iter().any(|trigger| {
            let normalized_trigger = normalize_recall_text(trigger);
            if normalized_trigger.is_empty() {
                return false;
            }
            if normalized_trigger.contains(' ') {
                normalized_query.contains(&normalized_trigger)
            } else {
                query_tokens
                    .iter()
                    .any(|token| token.as_str() == normalized_trigger)
            }
        });
        if !matched {
            continue;
        }

        for expansion in rule.expansions {
            let normalized_expansion = normalize_recall_text(expansion);
            if normalized_expansion.is_empty() || normalized_query.contains(&normalized_expansion) {
                continue;
            }
            if seen.insert(normalized_expansion) {
                additions.push((*expansion).to_string());
            }
            if additions.len() >= QUERY_EXPANSION_MAX_TERMS {
                break;
            }
        }
        if additions.len() >= QUERY_EXPANSION_MAX_TERMS {
            break;
        }
    }

    if additions.is_empty() {
        trimmed.to_string()
    } else {
        format!("{trimmed} {}", additions.join(" "))
    }
}

fn is_noise_memory_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.chars().count() < 5 {
        return true;
    }
    let lowered = trimmed.to_lowercase();
    if NOISE_SUBSTRING_PATTERNS
        .iter()
        .any(|pattern| lowered.contains(pattern))
    {
        return true;
    }
    if NOISE_EXACT_PATTERNS
        .iter()
        .any(|pattern| lowered.as_str() == *pattern)
    {
        return true;
    }
    if trimmed.chars().count() <= 10
        && NOISE_SHORT_PREFIX_PATTERNS
            .iter()
            .any(|pattern| lowered.starts_with(pattern))
    {
        return true;
    }
    false
}

fn token_count_map(tokens: &[String]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for token in tokens {
        *counts.entry(token.clone()).or_insert(0) += 1;
    }
    counts
}

fn unique_tokens(tokens: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for token in tokens {
        if seen.insert(token.clone()) {
            unique.push(token.clone());
        }
    }
    unique
}

fn query_doc_frequency(
    candidates: &[ScoredCandidate],
    query_tokens: &[String],
) -> HashMap<String, usize> {
    let mut freq = HashMap::new();
    for token in query_tokens {
        let mut count = 0_usize;
        for candidate in candidates {
            if candidate.token_counts.contains_key(token) {
                count += 1;
            }
        }
        freq.insert(token.clone(), count);
    }
    freq
}

fn bm25_like_score(
    query_token_counts: &HashMap<String, usize>,
    normalized_query: &str,
    candidate: &ScoredCandidate,
    doc_count: usize,
    avg_doc_len: f64,
    doc_frequency: &HashMap<String, usize>,
) -> f64 {
    let k1 = 1.2;
    let b = 0.75;
    let mut score = 0.0;

    for (token, query_tf) in query_token_counts {
        let doc_tf = *candidate.token_counts.get(token).unwrap_or(&0) as f64;
        if doc_tf <= 0.0 {
            continue;
        }
        let df = *doc_frequency.get(token).unwrap_or(&0) as f64;
        let idf = ((doc_count as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
        let norm = doc_tf * (k1 + 1.0)
            / (doc_tf + k1 * (1.0 - b + b * (candidate.token_len as f64 / avg_doc_len.max(1.0))));
        score += idf * norm * (*query_tf as f64);
    }

    if !normalized_query.is_empty() && candidate.normalized_text.contains(normalized_query) {
        score += 1.2;
    }

    score += 0.6 * char_bigram_jaccard(normalized_query, &candidate.normalized_text);

    if score <= 0.0 {
        0.0
    } else {
        clamp_score(1.0 - (-score / 3.5).exp())
    }
}

fn ranked_indices_by<F>(candidates: &[ScoredCandidate], metric: F) -> Vec<usize>
where
    F: Fn(&ScoredCandidate) -> f64,
{
    let mut indices: Vec<usize> = (0..candidates.len()).collect();
    indices.sort_by(|left, right| {
        let left_candidate = &candidates[*left];
        let right_candidate = &candidates[*right];
        metric(right_candidate)
            .total_cmp(&metric(left_candidate))
            .then_with(|| {
                right_candidate
                    .row
                    .updated_at
                    .cmp(&left_candidate.row.updated_at)
            })
            .then_with(|| left_candidate.row.id.cmp(&right_candidate.row.id))
    });
    indices
}

fn lightweight_rerank_signal(
    unique_query_tokens: &[String],
    normalized_query: &str,
    normalized_text: &str,
    token_counts: &HashMap<String, usize>,
) -> f64 {
    let matched_tokens = unique_query_tokens
        .iter()
        .filter(|token| token_counts.contains_key(*token))
        .count();
    let overlap = if unique_query_tokens.is_empty() {
        0.0
    } else {
        matched_tokens as f64 / unique_query_tokens.len() as f64
    };
    let phrase_match = if !normalized_query.is_empty() && normalized_text.contains(normalized_query)
    {
        1.0
    } else {
        0.0
    };
    let char_overlap = char_bigram_jaccard(normalized_query, normalized_text);
    clamp_score(0.55 * overlap + 0.30 * char_overlap + 0.15 * phrase_match)
}

fn char_bigram_jaccard(left: &str, right: &str) -> f64 {
    let left_set = char_bigram_set(left);
    let right_set = char_bigram_set(right);
    if left_set.is_empty() || right_set.is_empty() {
        return 0.0;
    }

    let intersection = left_set.intersection(&right_set).count() as f64;
    let union = left_set.union(&right_set).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn char_bigram_set(text: &str) -> HashSet<String> {
    let chars: Vec<char> = text
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || is_cjk(*ch))
        .collect();
    let mut set = HashSet::new();
    for pair in chars.windows(2) {
        set.insert(format!("{}{}", pair[0], pair[1]));
    }
    if set.is_empty() && !chars.is_empty() {
        set.insert(chars[0].to_string());
    }
    set
}

fn parse_rerank_items(provider: &str, value: &Value) -> Result<Vec<(usize, f64)>, String> {
    let primary = if provider == "voyage" || provider == "pinecone" {
        "data"
    } else {
        "results"
    };
    let fallback = if primary == "data" { "results" } else { "data" };
    let items = value
        .get(primary)
        .and_then(|v| v.as_array())
        .or_else(|| value.get(fallback).and_then(|v| v.as_array()))
        .ok_or_else(|| "rerank provider response missing results/data array".to_string())?;

    let mut parsed = Vec::new();
    for item in items {
        let Some(index) = parse_rerank_index(item) else {
            continue;
        };
        let Some(score) = parse_rerank_score(item) else {
            continue;
        };
        parsed.push((index, clamp_score(score)));
    }
    if parsed.is_empty() {
        return Err(
            "rerank provider response did not include usable index/score items".to_string(),
        );
    }
    Ok(parsed)
}

fn parse_rerank_index(item: &Value) -> Option<usize> {
    item.get("index")
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
        .or_else(|| {
            item.get("index")
                .and_then(|value| value.as_i64())
                .filter(|value| *value >= 0)
                .map(|value| value as usize)
        })
        .or_else(|| {
            item.get("index")
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse::<usize>().ok())
        })
}

fn parse_rerank_score(item: &Value) -> Option<f64> {
    item.get("relevance_score")
        .and_then(|value| value.as_f64())
        .or_else(|| item.get("score").and_then(|value| value.as_f64()))
        .filter(|value| value.is_finite())
}

fn normalize_embeddings_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/embeddings") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/embeddings")
    }
}

fn parse_api_keys(raw: Option<&str>) -> Vec<String> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut keys = Vec::new();
    for part in raw.split([',', ';', '\n']) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            keys.push(trimmed.to_string());
        }
    }
    keys
}

fn trim_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn supports_embedding_tuning_fields(endpoint: &str, model: &str, api_hint: &str) -> bool {
    let model_lower = model.trim().to_ascii_lowercase();
    if model_lower.starts_with("jina-") || model_lower.contains("jina-embeddings") {
        return true;
    }

    let api_lower = api_hint.trim().to_ascii_lowercase();
    if api_lower == "jina" || api_lower == "jina-compatible" {
        return true;
    }

    endpoint.to_ascii_lowercase().contains("jina.ai")
}

fn clamp_access_count(value: i64) -> i64 {
    value.clamp(0, MAX_ACCESS_COUNT)
}

fn compute_effective_half_life_days(
    base_half_life_days: f64,
    access_count: i64,
    last_accessed_at: i64,
    reinforcement_factor: f64,
    max_half_life_multiplier: f64,
    now_ms: i64,
) -> f64 {
    if reinforcement_factor <= 0.0 || access_count <= 0 {
        return base_half_life_days;
    }

    let days_since_last_access = ((now_ms - last_accessed_at).max(0) as f64) / 86_400_000.0;
    let access_freshness =
        (-days_since_last_access * (std::f64::consts::LN_2 / ACCESS_DECAY_HALF_LIFE_DAYS)).exp();
    let effective_access_count = clamp_access_count(access_count) as f64 * access_freshness;
    let extension = base_half_life_days * reinforcement_factor * effective_access_count.ln_1p();
    let cap = base_half_life_days * max_half_life_multiplier.max(1.0);
    (base_half_life_days + extension).min(cap)
}

fn apply_mmr_diversity(
    ranked_rows: Vec<RankedMemoryRow>,
    similarity_threshold: f64,
) -> (Vec<RankedMemoryRow>, usize) {
    if ranked_rows.len() <= 1 {
        return (ranked_rows, 0);
    }

    let threshold = similarity_threshold.clamp(0.0, 1.0);
    let mut selected = Vec::with_capacity(ranked_rows.len());
    let mut deferred = Vec::new();
    for row in ranked_rows {
        let too_similar = selected.iter().any(|existing: &RankedMemoryRow| {
            let left = existing.row.vector.as_deref();
            let right = row.row.vector.as_deref();
            match (left, right) {
                (Some(left), Some(right)) => cosine_similarity_f32(left, right) > threshold,
                _ => false,
            }
        });
        if too_similar {
            deferred.push(row);
        } else {
            selected.push(row);
        }
    }
    let deferred_count = deferred.len();
    selected.extend(deferred);
    (selected, deferred_count)
}

fn is_embedding_failover_retryable(status: u16) -> bool {
    matches!(status, 401 | 403 | 408 | 409 | 425 | 429 | 500..=599)
}

fn is_rerank_failover_retryable(status: u16) -> bool {
    matches!(status, 401 | 403 | 408 | 409 | 425 | 429 | 500..=599)
}

fn is_embedding_context_limit_error(message: &str) -> bool {
    let lowered = message.to_ascii_lowercase();
    (lowered.contains("context")
        && (lowered.contains("length")
            || lowered.contains("window")
            || lowered.contains("limit")
            || lowered.contains("exceed")))
        || lowered.contains("maximum context length")
        || lowered.contains("too long")
        || lowered.contains("token limit")
        || lowered.contains("max input")
}

fn smart_chunk_text(text: &str, model: &str) -> Vec<String> {
    let normalized = text.trim();
    if normalized.is_empty() {
        return Vec::new();
    }

    let (max_chunk_size, overlap_size, min_chunk_size, max_lines_per_chunk) =
        chunking_config_for_model(model);
    let chars: Vec<char> = normalized.chars().collect();
    if chars.len() <= max_chunk_size {
        return vec![normalized.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    let max_guard = ((chars.len() / max_chunk_size.max(1)) + 8).max(8);
    let mut guard = 0usize;
    while start < chars.len() && guard < max_guard {
        guard += 1;
        let remaining = chars.len() - start;
        if remaining <= max_chunk_size {
            let tail = chars[start..].iter().collect::<String>().trim().to_string();
            if !tail.is_empty() {
                chunks.push(tail);
            }
            break;
        }

        let max_end = (start + max_chunk_size).min(chars.len());
        let min_end = (start + min_chunk_size).min(max_end).max(start + 1);
        let end = choose_chunk_end(&chars, start, min_end, max_end, max_lines_per_chunk)
            .max(start + 1)
            .min(chars.len());
        let chunk = chars[start..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        if end >= chars.len() {
            break;
        }
        let next = end.saturating_sub(overlap_size).max(start + 1);
        start = next;
    }

    if chunks.is_empty() {
        vec![normalized.to_string()]
    } else {
        chunks
    }
}

fn chunking_config_for_model(model: &str) -> (usize, usize, usize, usize) {
    let lowered = model.trim().to_ascii_lowercase();
    let base = match lowered.as_str() {
        "gemini-embedding-001" => 2048usize,
        "all-minilm-l6-v2" | "all-mpnet-base-v2" => 512usize,
        _ => 8192usize,
    };
    let max_chunk_size = ((base as f64) * 0.7).floor() as usize;
    let overlap_size = ((base as f64) * 0.05).floor() as usize;
    let min_chunk_size = ((base as f64) * 0.1).floor() as usize;
    (
        max_chunk_size.max(1000),
        overlap_size,
        min_chunk_size.max(100),
        50,
    )
}

fn choose_chunk_end(
    chars: &[char],
    start: usize,
    min_end: usize,
    max_end: usize,
    max_lines_per_chunk: usize,
) -> usize {
    if max_lines_per_chunk > 0 {
        let mut line_breaks = 0usize;
        for idx in start..max_end {
            if chars[idx] == '\n' {
                line_breaks += 1;
                if line_breaks >= max_lines_per_chunk {
                    return (idx + 1).max(min_end).min(max_end);
                }
            }
        }
    }

    for idx in (min_end..max_end).rev() {
        if matches!(chars[idx], '.' | '!' | '?' | '。' | '！' | '？') {
            let mut cursor = idx + 1;
            while cursor < max_end && chars[cursor].is_whitespace() {
                cursor += 1;
            }
            return cursor.max(min_end).min(max_end);
        }
    }

    for idx in (min_end..max_end).rev() {
        if chars[idx] == '\n' {
            return (idx + 1).max(min_end).min(max_end);
        }
    }

    for idx in (min_end..max_end).rev() {
        if chars[idx].is_whitespace() {
            return idx.max(min_end).min(max_end);
        }
    }

    max_end
}

fn average_embeddings(vectors: &[Vec<f32>], dimensions: usize) -> Result<Vec<f32>, String> {
    if vectors.is_empty() {
        return Err("embedding context recovery produced zero chunk vectors".to_string());
    }
    let mut sum = vec![0.0_f64; dimensions];
    for vector in vectors {
        if vector.len() != dimensions {
            return Err(format!(
                "embedding dimension mismatch during chunk recovery: expected {}, got {}",
                dimensions,
                vector.len()
            ));
        }
        for (idx, value) in vector.iter().enumerate() {
            sum[idx] += f64::from(*value);
        }
    }
    let factor = vectors.len() as f64;
    Ok(sum
        .into_iter()
        .map(|value| (value / factor) as f32)
        .collect())
}

fn emit_internal_diagnostic(enabled: bool, payload: Value) {
    if !enabled {
        return;
    }
    if let Ok(encoded) = serde_json::to_string(&payload) {
        eprintln!("{encoded}");
    } else {
        eprintln!("{{\"event\":\"retrieval.diagnostic.serialization_failed\",\"stage\":\"emit\"}}");
    }
}

fn make_trace_stage(name: &str, status: RetrievalTraceStageStatus) -> RetrievalTraceStage {
    RetrievalTraceStage {
        name: name.to_string(),
        status,
        input_count: None,
        output_count: None,
        fallback_to: None,
        reason: None,
        metrics: BTreeMap::new(),
    }
}

fn is_vector_index_type(index_type: &IndexType) -> bool {
    matches!(
        index_type,
        IndexType::IvfFlat
            | IndexType::IvfSq
            | IndexType::IvfPq
            | IndexType::IvfRq
            | IndexType::IvfHnswPq
            | IndexType::IvfHnswSq
    )
}

fn resolve_secret(value: Option<&str>) -> anyhow::Result<Option<String>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(resolve_env_placeholders(trimmed)?))
}

fn resolve_env_placeholders(value: &str) -> anyhow::Result<String> {
    let mut output = String::new();
    let bytes = value.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if idx + 1 < bytes.len() && bytes[idx] == b'$' && bytes[idx + 1] == b'{' {
            let mut cursor = idx + 2;
            while cursor < bytes.len() && bytes[cursor] != b'}' {
                cursor += 1;
            }
            if cursor >= bytes.len() {
                anyhow::bail!("unterminated environment placeholder in secret");
            }
            let key = &value[idx + 2..cursor];
            let env_value = std::env::var(key)
                .map_err(|_| anyhow::anyhow!("environment variable {} is not set", key))?;
            output.push_str(&env_value);
            idx = cursor + 1;
        } else {
            output.push(bytes[idx] as char);
            idx += 1;
        }
    }
    Ok(output)
}

fn truncate_for_error(value: &str, max_chars: usize) -> String {
    let mut truncated = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if idx >= max_chars {
            truncated.push_str("...");
            return truncated;
        }
        truncated.push(ch);
    }
    truncated
}

fn cosine_similarity_f32(left: &[f32], right: &[f32]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0;
    let mut left_norm = 0.0;
    let mut right_norm = 0.0;
    for (l, r) in left.iter().zip(right.iter()) {
        dot += f64::from(*l) * f64::from(*r);
        left_norm += f64::from(*l) * f64::from(*l);
        right_norm += f64::from(*r) * f64::from(*r);
    }
    let denom = left_norm.sqrt() * right_norm.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        (dot / denom).clamp(-1.0, 1.0)
    }
}

fn stable_hash64(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ seed;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn clamp_score(score: f64) -> f64 {
    if !score.is_finite() {
        return 0.0;
    }
    score.clamp(0.0, 1.0)
}

fn round_score(score: f64) -> f64 {
    (clamp_score(score) * 1_000_000.0).round() / 1_000_000.0
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
