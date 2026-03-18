use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    #[serde(default)]
    pub retrieval: RetrievalConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
    pub lancedb_path: PathBuf,
    pub sqlite_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AuthConfig {
    pub runtime: TokenConfig,
    pub admin: TokenConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TokenConfig {
    pub token: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub embedding: EmbeddingProviderConfig,
    #[serde(default)]
    pub rerank: RerankProviderConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EmbeddingProviderConfig {
    #[serde(default = "default_embedding_provider")]
    pub provider: String,
    #[serde(default = "default_embedding_dimensions")]
    pub dimensions: usize,
    #[serde(default = "default_embedding_model")]
    pub model: String,
    #[serde(default = "default_embedding_api")]
    pub api: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default, alias = "taskQuery")]
    pub task_query: Option<String>,
    #[serde(default, alias = "taskPassage")]
    pub task_passage: Option<String>,
    #[serde(default)]
    pub normalized: Option<bool>,
    #[serde(default = "default_embedding_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_embedding_cache_max_entries")]
    pub cache_max_entries: usize,
    #[serde(default = "default_embedding_cache_ttl_ms")]
    pub cache_ttl_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RerankProviderConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rerank_mode")]
    pub mode: String,
    #[serde(default = "default_rerank_provider")]
    pub provider: String,
    #[serde(default = "default_rerank_blend")]
    pub blend: f64,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_rerank_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RetrievalConfig {
    #[serde(default = "default_candidate_pool_size")]
    pub candidate_pool_size: usize,
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f64,
    #[serde(default = "default_bm25_weight")]
    pub bm25_weight: f64,
    #[serde(default = "default_min_score")]
    pub min_score: f64,
    #[serde(default = "default_hard_min_score")]
    pub hard_min_score: f64,
    #[serde(default = "default_recency_half_life_days")]
    pub recency_half_life_days: f64,
    #[serde(default = "default_recency_weight")]
    pub recency_weight: f64,
    #[serde(default = "default_length_norm_anchor")]
    pub length_norm_anchor: usize,
    #[serde(default = "default_time_decay_half_life_days")]
    pub time_decay_half_life_days: f64,
    #[serde(default = "default_reinforcement_factor")]
    pub reinforcement_factor: f64,
    #[serde(default = "default_max_half_life_multiplier")]
    pub max_half_life_multiplier: f64,
    #[serde(default = "default_mmr_diversity")]
    pub mmr_diversity: bool,
    #[serde(default = "default_mmr_similarity_threshold")]
    pub mmr_similarity_threshold: f64,
    #[serde(default = "default_query_expansion")]
    pub query_expansion: bool,
    #[serde(default = "default_filter_noise")]
    pub filter_noise: bool,
    #[serde(default = "default_retrieval_diagnostics")]
    pub diagnostics: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            embedding: EmbeddingProviderConfig::default(),
            rerank: RerankProviderConfig::default(),
        }
    }
}

impl Default for EmbeddingProviderConfig {
    fn default() -> Self {
        Self {
            provider: default_embedding_provider(),
            dimensions: default_embedding_dimensions(),
            model: default_embedding_model(),
            api: default_embedding_api(),
            base_url: None,
            api_key: None,
            task_query: None,
            task_passage: None,
            normalized: None,
            timeout_ms: default_embedding_timeout_ms(),
            cache_max_entries: default_embedding_cache_max_entries(),
            cache_ttl_ms: default_embedding_cache_ttl_ms(),
        }
    }
}

impl Default for RerankProviderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_rerank_mode(),
            provider: default_rerank_provider(),
            blend: default_rerank_blend(),
            model: None,
            api: None,
            base_url: None,
            api_key: None,
            timeout_ms: default_rerank_timeout_ms(),
        }
    }
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            candidate_pool_size: default_candidate_pool_size(),
            vector_weight: default_vector_weight(),
            bm25_weight: default_bm25_weight(),
            min_score: default_min_score(),
            hard_min_score: default_hard_min_score(),
            recency_half_life_days: default_recency_half_life_days(),
            recency_weight: default_recency_weight(),
            length_norm_anchor: default_length_norm_anchor(),
            time_decay_half_life_days: default_time_decay_half_life_days(),
            reinforcement_factor: default_reinforcement_factor(),
            max_half_life_multiplier: default_max_half_life_multiplier(),
            mmr_diversity: default_mmr_diversity(),
            mmr_similarity_threshold: default_mmr_similarity_threshold(),
            query_expansion: default_query_expansion(),
            filter_noise: default_filter_noise(),
            diagnostics: default_retrieval_diagnostics(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_embedding_provider() -> String {
    "hashing".to_string()
}

fn default_embedding_dimensions() -> usize {
    384
}

fn default_embedding_model() -> String {
    "hashing-384-v1".to_string()
}

fn default_embedding_api() -> String {
    "builtin".to_string()
}

fn default_rerank_mode() -> String {
    "lightweight".to_string()
}

fn default_rerank_provider() -> String {
    "jina".to_string()
}

fn default_rerank_blend() -> f64 {
    0.35
}

fn default_embedding_timeout_ms() -> u64 {
    10_000
}

fn default_embedding_cache_max_entries() -> usize {
    256
}

fn default_embedding_cache_ttl_ms() -> u64 {
    30 * 60_000
}

fn default_rerank_timeout_ms() -> u64 {
    5_000
}

fn default_candidate_pool_size() -> usize {
    64
}

fn default_vector_weight() -> f64 {
    0.7
}

fn default_bm25_weight() -> f64 {
    0.3
}

fn default_min_score() -> f64 {
    0.2
}

fn default_hard_min_score() -> f64 {
    0.25
}

fn default_recency_half_life_days() -> f64 {
    14.0
}

fn default_recency_weight() -> f64 {
    0.1
}

fn default_length_norm_anchor() -> usize {
    500
}

fn default_time_decay_half_life_days() -> f64 {
    60.0
}

fn default_reinforcement_factor() -> f64 {
    0.5
}

fn default_max_half_life_multiplier() -> f64 {
    3.0
}

fn default_mmr_diversity() -> bool {
    true
}

fn default_mmr_similarity_threshold() -> f64 {
    0.85
}

fn default_query_expansion() -> bool {
    true
}

fn default_filter_noise() -> bool {
    true
}

fn default_retrieval_diagnostics() -> bool {
    false
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let cfg: Self = toml::from_str(&raw)
            .with_context(|| format!("failed to parse TOML config: {}", path.display()))?;
        cfg.validate()
            .with_context(|| format!("invalid backend config loaded from {}", path.display()))?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.server.bind.trim().is_empty() {
            anyhow::bail!("server.bind cannot be empty");
        }
        if self.auth.runtime.token.trim().is_empty() {
            anyhow::bail!("auth.runtime.token cannot be empty");
        }
        if self.auth.admin.token.trim().is_empty() {
            anyhow::bail!("auth.admin.token cannot be empty");
        }
        if self.storage.sqlite_path.as_os_str().is_empty() {
            anyhow::bail!("storage.sqlite_path cannot be empty");
        }
        if self.storage.lancedb_path.as_os_str().is_empty() {
            anyhow::bail!("storage.lancedb_path cannot be empty");
        }
        let embedding_provider = self.providers.embedding.provider.trim();
        if embedding_provider != "hashing" && embedding_provider != "openai-compatible" {
            anyhow::bail!(
                "providers.embedding.provider must be one of: 'hashing', 'openai-compatible'"
            );
        }
        if !(64..=4096).contains(&self.providers.embedding.dimensions) {
            anyhow::bail!(
                "providers.embedding.dimensions must be within [64, 4096], got {}",
                self.providers.embedding.dimensions
            );
        }
        if self.providers.embedding.timeout_ms == 0 || self.providers.embedding.timeout_ms > 120_000
        {
            anyhow::bail!("providers.embedding.timeout_ms must be within [1, 120000]");
        }
        if self.providers.embedding.cache_max_entries > 10_000 {
            anyhow::bail!("providers.embedding.cache_max_entries must be within [0, 10000]");
        }
        if self.providers.embedding.cache_ttl_ms > 86_400_000 {
            anyhow::bail!("providers.embedding.cache_ttl_ms must be <= 86400000");
        }
        if let Some(task_query) = &self.providers.embedding.task_query {
            if task_query.trim().is_empty() {
                anyhow::bail!("providers.embedding.task_query cannot be empty when configured");
            }
        }
        if let Some(task_passage) = &self.providers.embedding.task_passage {
            if task_passage.trim().is_empty() {
                anyhow::bail!("providers.embedding.task_passage cannot be empty when configured");
            }
        }
        if embedding_provider != "openai-compatible"
            && (self.providers.embedding.task_query.is_some()
                || self.providers.embedding.task_passage.is_some()
                || self.providers.embedding.normalized.is_some())
        {
            anyhow::bail!(
                "providers.embedding.task_query/task_passage/normalized require providers.embedding.provider = 'openai-compatible'"
            );
        }

        let rerank_mode = self.providers.rerank.mode.trim();
        if rerank_mode != "none" && rerank_mode != "lightweight" && rerank_mode != "cross-encoder" {
            anyhow::bail!(
                "providers.rerank.mode must be one of: 'none', 'lightweight', 'cross-encoder'"
            );
        }
        let rerank_provider = self.providers.rerank.provider.trim();
        let rerank_provider_valid = matches!(
            rerank_provider,
            "jina" | "siliconflow" | "voyage" | "pinecone" | "vllm"
        );
        if !rerank_provider_valid {
            anyhow::bail!(
                "providers.rerank.provider must be one of: 'jina', 'siliconflow', 'voyage', 'pinecone', 'vllm'"
            );
        }
        if !(0.0..=1.0).contains(&self.providers.rerank.blend) {
            anyhow::bail!("providers.rerank.blend must be within [0, 1]");
        }
        if self.providers.rerank.timeout_ms == 0 || self.providers.rerank.timeout_ms > 120_000 {
            anyhow::bail!("providers.rerank.timeout_ms must be within [1, 120000]");
        }
        if self.retrieval.candidate_pool_size == 0 {
            anyhow::bail!("retrieval.candidate_pool_size must be > 0");
        }
        if !(0.0..=1.0).contains(&self.retrieval.vector_weight) {
            anyhow::bail!("retrieval.vector_weight must be within [0, 1]");
        }
        if !(0.0..=1.0).contains(&self.retrieval.bm25_weight) {
            anyhow::bail!("retrieval.bm25_weight must be within [0, 1]");
        }
        if self.retrieval.vector_weight + self.retrieval.bm25_weight <= 0.0 {
            anyhow::bail!("retrieval.vector_weight + retrieval.bm25_weight must be > 0");
        }
        if !(0.0..=1.0).contains(&self.retrieval.min_score) {
            anyhow::bail!("retrieval.min_score must be within [0, 1]");
        }
        if !(0.0..=1.0).contains(&self.retrieval.hard_min_score) {
            anyhow::bail!("retrieval.hard_min_score must be within [0, 1]");
        }
        if self.retrieval.hard_min_score < self.retrieval.min_score {
            anyhow::bail!("retrieval.hard_min_score cannot be smaller than retrieval.min_score");
        }
        if self.retrieval.recency_half_life_days <= 0.0 {
            anyhow::bail!("retrieval.recency_half_life_days must be > 0");
        }
        if !(0.0..=1.0).contains(&self.retrieval.recency_weight) {
            anyhow::bail!("retrieval.recency_weight must be within [0, 1]");
        }
        if self.retrieval.length_norm_anchor < 32 {
            anyhow::bail!("retrieval.length_norm_anchor must be >= 32");
        }
        if self.retrieval.time_decay_half_life_days <= 0.0 {
            anyhow::bail!("retrieval.time_decay_half_life_days must be > 0");
        }
        if !(0.0..=5.0).contains(&self.retrieval.reinforcement_factor) {
            anyhow::bail!("retrieval.reinforcement_factor must be within [0, 5]");
        }
        if !(1.0..=10.0).contains(&self.retrieval.max_half_life_multiplier) {
            anyhow::bail!("retrieval.max_half_life_multiplier must be within [1, 10]");
        }
        if !(0.0..=1.0).contains(&self.retrieval.mmr_similarity_threshold) {
            anyhow::bail!("retrieval.mmr_similarity_threshold must be within [0, 1]");
        }
        Ok(())
    }
}
