use crate::{
    config::AppConfig,
    error::{AppError, AppResult},
    models::{
        clamped_limit, validate_non_empty, Actor, Category, DeleteRequest,
        EnqueueReflectionJobResponse, ListRequest, ListResponse, ListRow, MemoryAction,
        MemoryMutationResult, Principal, RecallGenericRequest, RecallGenericResponse,
        RecallGenericRow, RecallReflectionRequest, RecallReflectionResponse, ReflectionJobStatus,
        ReflectionJobStatusResponse, ReflectionKind, ReflectionMetadata, ReflectionRecallMode,
        ReflectionTrigger, RowMetadata, StatsResponse, StoreRequest, StoreResponse, UpdateRequest,
        UpdateResponse, DEFAULT_IMPORTANCE,
    },
};
use arrow_array::{
    Array, ArrayRef, Float64Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{ArrowError, DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::{
    connect,
    query::{ExecutableQuery, QueryBase},
    Connection as LanceConnection, Error as LanceError, Table as LanceTable,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const MEMORY_TABLE_NAME: &str = "memories_v1";

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
        let memory_repo = Arc::new(LanceMemoryRepo::new(config.storage.lancedb_path.clone())?);
        Ok(Self {
            config,
            memory_repo,
            job_store,
            idempotency_store,
        })
    }
}

#[derive(Clone)]
pub struct LanceMemoryRepo {
    db_path: PathBuf,
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
    reflection_kind: Option<ReflectionKind>,
    strict_key: Option<String>,
}

impl LanceMemoryRepo {
    pub fn new(db_path: PathBuf) -> anyhow::Result<Self> {
        fs::create_dir_all(&db_path)?;
        Ok(Self { db_path })
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
                    reflection_kind: None,
                    strict_key: None,
                };
                normalize_reflection_fields(&mut row);
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
                        reflection_kind: None,
                        strict_key: None,
                    };
                    results.push(to_mutation_result(&row, MemoryAction::Add));
                    rows.push(row);
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

        if let Some(text) = req.patch.text {
            row.text = text;
        }
        if let Some(category) = req.patch.category {
            row.category = category;
        }
        if let Some(importance) = req.patch.importance {
            row.importance = importance;
        }
        row.updated_at = now_millis();
        normalize_reflection_fields(&mut row);

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
        let limit = clamped_limit(req.limit) as usize;
        let lowered_query = req.query.to_lowercase();
        let table = self.open_or_create_table().await?;

        let mut rows: Vec<RecallGenericRow> = self
            .query_rows(&table, Some(principal_filter(&req.actor)))
            .await?
            .into_iter()
            .map(|row| {
                let score = if row.text.to_lowercase().contains(&lowered_query) {
                    0.95
                } else {
                    0.51
                };
                RecallGenericRow {
                    id: row.id,
                    text: row.text,
                    category: row.category,
                    scope: row.scope,
                    score,
                    metadata: RowMetadata {
                        created_at: row.created_at,
                        updated_at: row.updated_at,
                    },
                }
            })
            .collect();

        rows.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| b.metadata.created_at.cmp(&a.metadata.created_at))
                .then_with(|| a.id.cmp(&b.id))
        });
        rows.truncate(limit);

        Ok(RecallGenericResponse { rows })
    }

    pub async fn recall_reflection(
        &self,
        req: RecallReflectionRequest,
    ) -> AppResult<RecallReflectionResponse> {
        let mode = req.mode.unwrap_or(ReflectionRecallMode::InvariantDerived);
        let limit = clamped_limit(req.limit) as usize;
        let lowered_query = req.query.to_lowercase();
        let table = self.open_or_create_table().await?;

        let mut rows = self
            .query_rows(&table, Some(principal_filter(&req.actor)))
            .await?
            .into_iter()
            .filter(|row| row.category == Category::Reflection)
            .filter_map(|row| {
                let kind = row.reflection_kind.unwrap_or(ReflectionKind::Derived);
                if matches!(mode, ReflectionRecallMode::InvariantOnly)
                    && !matches!(kind, ReflectionKind::Invariant)
                {
                    return None;
                }

                let strict_key = if matches!(kind, ReflectionKind::Invariant) {
                    Some(
                        row.strict_key
                            .unwrap_or_else(|| default_strict_key(&row.id)),
                    )
                } else {
                    None
                };
                let score = if row.text.to_lowercase().contains(&lowered_query) {
                    0.92
                } else {
                    0.63
                };

                Some(crate::models::RecallReflectionRow {
                    id: row.id,
                    text: row.text,
                    kind,
                    strict_key,
                    scope: row.scope,
                    score,
                    metadata: ReflectionMetadata {
                        timestamp: row.created_at,
                    },
                })
            })
            .collect::<Vec<_>>();

        rows.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| b.metadata.timestamp.cmp(&a.metadata.timestamp))
                .then_with(|| a.id.cmp(&b.id))
        });
        rows.truncate(limit);

        Ok(RecallReflectionResponse { rows })
    }

    async fn insert_rows(&self, rows: &[MemoryRow]) -> AppResult<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let table = self.open_or_create_table().await?;
        let batch = rows_to_record_batch(rows)?;
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
            Ok(table) => Ok(table),
            Err(LanceError::TableNotFound { .. }) => self.create_table(&conn).await,
            Err(err) => Err(AppError::internal(format!(
                "failed to open LanceDB memory table: {err}"
            ))),
        }
    }

    async fn create_table(&self, conn: &LanceConnection) -> AppResult<LanceTable> {
        let schema = memory_table_schema();
        match conn
            .create_empty_table(MEMORY_TABLE_NAME, schema)
            .execute()
            .await
        {
            Ok(table) => Ok(table),
            Err(LanceError::TableAlreadyExists { .. }) => conn
                .open_table(MEMORY_TABLE_NAME)
                .execute()
                .await
                .map_err(|err| {
                    AppError::internal(format!(
                        "failed to open existing LanceDB memory table: {err}"
                    ))
                }),
            Err(err) => Err(AppError::internal(format!(
                "failed to create LanceDB memory table: {err}"
            ))),
        }
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

    pub fn enqueue(
        &self,
        actor: &Actor,
        trigger: ReflectionTrigger,
    ) -> AppResult<EnqueueReflectionJobResponse> {
        validate_non_empty("actor.userId", &actor.user_id)?;
        validate_non_empty("actor.agentId", &actor.agent_id)?;

        let status = ReflectionJobStatus::Queued;
        let now = now_millis();
        let job_id = format!("job_{}", Uuid::new_v4().simple());
        let conn = self.open_conn().map_err(AppError::from)?;
        conn.execute(
            "INSERT INTO reflection_jobs (
                job_id,
                user_id,
                agent_id,
                session_key,
                session_id,
                trigger,
                status,
                persisted,
                memory_count,
                error_code,
                error_message,
                error_retryable,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, NULL, NULL, NULL, ?8, ?8)",
            params![
                &job_id,
                &actor.user_id,
                &actor.agent_id,
                &actor.session_key,
                &actor.session_id,
                trigger_to_str(trigger),
                status_to_str(status),
                now,
            ],
        )
        .map_err(|err| AppError::internal(format!("failed to enqueue reflection job: {err}")))?;

        Ok(EnqueueReflectionJobResponse { job_id, status })
    }

    pub fn get_scoped(
        &self,
        job_id: &str,
        user_id: &str,
        agent_id: &str,
    ) -> AppResult<Option<ReflectionJobStatusResponse>> {
        let conn = self.open_conn().map_err(AppError::from)?;
        let mut stmt = conn
            .prepare(
                "SELECT
                    user_id,
                    agent_id,
                    status,
                    persisted,
                    memory_count,
                    error_code,
                    error_message,
                    error_retryable
                 FROM reflection_jobs
                 WHERE job_id = ?1",
            )
            .map_err(|err| AppError::internal(format!("failed to prepare query: {err}")))?;

        let mut rows = stmt
            .query(params![job_id])
            .map_err(|err| AppError::internal(format!("failed to query reflection job: {err}")))?;

        let Some(row) = rows.next().map_err(|err| {
            AppError::internal(format!("failed to fetch reflection job row: {err}"))
        })?
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

        let status_raw: String = row
            .get(2)
            .map_err(|err| AppError::internal(format!("failed to read status: {err}")))?;
        let status = parse_status(&status_raw)?;

        let persisted_raw: i64 = row
            .get(3)
            .map_err(|err| AppError::internal(format!("failed to read persisted: {err}")))?;
        let memory_count_raw: i64 = row
            .get(4)
            .map_err(|err| AppError::internal(format!("failed to read memory_count: {err}")))?;
        let error_code: Option<String> = row
            .get(5)
            .map_err(|err| AppError::internal(format!("failed to read error_code: {err}")))?;
        let error_message: Option<String> = row
            .get(6)
            .map_err(|err| AppError::internal(format!("failed to read error_message: {err}")))?;
        let error_retryable: Option<i64> = row
            .get(7)
            .map_err(|err| AppError::internal(format!("failed to read error_retryable: {err}")))?;

        let response = match status {
            ReflectionJobStatus::Queued | ReflectionJobStatus::Running => {
                ReflectionJobStatusResponse {
                    job_id: job_id.to_string(),
                    status,
                    persisted: None,
                    memory_count: None,
                    error: None,
                }
            }
            ReflectionJobStatus::Completed => ReflectionJobStatusResponse {
                job_id: job_id.to_string(),
                status,
                persisted: Some(persisted_raw != 0),
                memory_count: Some(memory_count_raw.max(0) as u64),
                error: None,
            },
            ReflectionJobStatus::Failed => ReflectionJobStatusResponse {
                job_id: job_id.to_string(),
                status,
                persisted: None,
                memory_count: None,
                error: Some(crate::models::JobStatusError {
                    code: error_code.unwrap_or_else(|| "UPSTREAM_REFLECTION_ERROR".to_string()),
                    message: error_message.unwrap_or_else(|| "reflection job failed".to_string()),
                    retryable: error_retryable.unwrap_or(0) != 0,
                    details: serde_json::json!({}),
                }),
            },
        };

        Ok(Some(response))
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.open_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS reflection_jobs (
                job_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                session_key TEXT NOT NULL,
                session_id TEXT NOT NULL,
                trigger TEXT NOT NULL,
                status TEXT NOT NULL,
                persisted INTEGER NOT NULL DEFAULT 0,
                memory_count INTEGER NOT NULL DEFAULT 0,
                error_code TEXT,
                error_message TEXT,
                error_retryable INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
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

fn rows_to_record_batch(rows: &[MemoryRow]) -> AppResult<RecordBatch> {
    let schema = memory_table_schema();
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
    let reflection_kind_values: Vec<Option<&str>> = rows
        .iter()
        .map(|row| row.reflection_kind.map(reflection_kind_to_str))
        .collect();
    let strict_key_values: Vec<Option<&str>> =
        rows.iter().map(|row| row.strict_key.as_deref()).collect();

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(id_values)),
        Arc::new(StringArray::from(principal_user_values)),
        Arc::new(StringArray::from(principal_agent_values)),
        Arc::new(StringArray::from(text_values)),
        Arc::new(StringArray::from(category_values)),
        Arc::new(Float64Array::from(importance_values)),
        Arc::new(StringArray::from(scope_values)),
        Arc::new(Int64Array::from(created_values)),
        Arc::new(Int64Array::from(updated_values)),
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
        let category_idx = schema_index(batch, "category")?;
        let importance_idx = schema_index(batch, "importance")?;
        let scope_idx = schema_index(batch, "scope")?;
        let created_idx = schema_index(batch, "created_at")?;
        let updated_idx = schema_index(batch, "updated_at")?;
        let reflection_kind_idx = schema_index(batch, "reflection_kind")?;
        let strict_key_idx = schema_index(batch, "strict_key")?;

        let id_col = as_string_array(batch.column(id_idx), "id")?;
        let principal_user_col =
            as_string_array(batch.column(principal_user_idx), "principal_user_id")?;
        let principal_agent_col =
            as_string_array(batch.column(principal_agent_idx), "principal_agent_id")?;
        let text_col = as_string_array(batch.column(text_idx), "text")?;
        let category_col = as_string_array(batch.column(category_idx), "category")?;
        let importance_col = as_f64_array(batch.column(importance_idx), "importance")?;
        let scope_col = as_string_array(batch.column(scope_idx), "scope")?;
        let created_col = as_i64_array(batch.column(created_idx), "created_at")?;
        let updated_col = as_i64_array(batch.column(updated_idx), "updated_at")?;
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
                reflection_kind,
                strict_key,
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

fn parse_status(raw: &str) -> AppResult<ReflectionJobStatus> {
    match raw {
        "queued" => Ok(ReflectionJobStatus::Queued),
        "running" => Ok(ReflectionJobStatus::Running),
        "completed" => Ok(ReflectionJobStatus::Completed),
        "failed" => Ok(ReflectionJobStatus::Failed),
        _ => Err(AppError::internal(format!(
            "unknown reflection job status persisted: {raw}"
        ))),
    }
}

fn status_to_str(status: ReflectionJobStatus) -> &'static str {
    match status {
        ReflectionJobStatus::Queued => "queued",
        ReflectionJobStatus::Running => "running",
        ReflectionJobStatus::Completed => "completed",
        ReflectionJobStatus::Failed => "failed",
    }
}

fn trigger_to_str(trigger: ReflectionTrigger) -> &'static str {
    match trigger {
        ReflectionTrigger::New => "new",
        ReflectionTrigger::Reset => "reset",
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

fn memory_table_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("principal_user_id", DataType::Utf8, false),
        Field::new("principal_agent_id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("category", DataType::Utf8, false),
        Field::new("importance", DataType::Float64, false),
        Field::new("scope", DataType::Utf8, false),
        Field::new("created_at", DataType::Int64, false),
        Field::new("updated_at", DataType::Int64, false),
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

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
