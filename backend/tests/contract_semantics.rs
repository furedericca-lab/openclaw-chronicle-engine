use arrow_array::{
    Float64Array, Int64Array, RecordBatch, RecordBatchIterator, RecordBatchReader, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, Method, Request, StatusCode},
    routing::post,
    Json, Router,
};
use lancedb::{connect, index::IndexType};
use chronicle_engine_rs::{
    build_app,
    config::{
        AppConfig, AuthConfig, LoggingConfig, ProvidersConfig, RetrievalConfig, ServerConfig,
        StorageConfig, TokenConfig,
    },
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::net::TcpListener;
use tower::ServiceExt;
use uuid::Uuid;

const RUNTIME_TOKEN: &str = "runtime-token";
const AUTH_USER_ID_HEADER: &str = "x-auth-user-id";
const AUTH_AGENT_ID_HEADER: &str = "x-auth-agent-id";

struct RequestAuth<'a> {
    auth_context: Option<(&'a str, &'a str)>,
    bearer_token: &'a str,
}

fn make_config(tmp: &Path) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            bind: "127.0.0.1:0".to_string(),
        },
        storage: StorageConfig {
            lancedb_path: tmp.join("lancedb"),
            sqlite_path: tmp.join("sqlite/jobs.db"),
        },
        auth: AuthConfig {
            runtime: TokenConfig {
                token: RUNTIME_TOKEN.to_string(),
            },
            admin: TokenConfig {
                token: "admin-token".to_string(),
            },
        },
        logging: LoggingConfig {
            level: "info".to_string(),
        },
        providers: ProvidersConfig::default(),
        retrieval: RetrievalConfig::default(),
    }
}

fn actor(user_id: &str, agent_id: &str, session_id: &str, session_key: &str) -> Value {
    json!({
        "userId": user_id,
        "agentId": agent_id,
        "sessionId": session_id,
        "sessionKey": session_key,
    })
}

async fn request_json(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
    idempotency_key: Option<&str>,
    auth_context: Option<(&str, &str)>,
    extra_headers: &[(&str, &str)],
) -> (StatusCode, Value) {
    request_json_with_token(
        app,
        method,
        path,
        body,
        idempotency_key,
        RequestAuth {
            auth_context,
            bearer_token: RUNTIME_TOKEN,
        },
        extra_headers,
    )
    .await
}

async fn request_json_with_token(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
    idempotency_key: Option<&str>,
    auth: RequestAuth<'_>,
    extra_headers: &[(&str, &str)],
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", auth.bearer_token),
        )
        .header("x-request-id", "req-test-1");

    if let Some((auth_user, auth_agent)) = auth.auth_context {
        builder = builder.header(AUTH_USER_ID_HEADER, auth_user);
        builder = builder.header(AUTH_AGENT_ID_HEADER, auth_agent);
    }

    if let Some(key) = idempotency_key {
        builder = builder.header("idempotency-key", key);
    }

    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }

    let request = if let Some(payload) = body {
        builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(payload.to_string()))
            .expect("request should be built")
    } else {
        builder
            .body(Body::empty())
            .expect("request should be built")
    };

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should produce a response");

    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body should be readable");

    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({ "_raw": String::from_utf8_lossy(&bytes).to_string() }))
    };

    (status, value)
}

async fn poll_distill_job(
    app: &Router,
    job_id: &str,
    auth_context: (&str, &str),
) -> (StatusCode, Value) {
    let path = format!("/v1/distill/jobs/{job_id}");
    for _ in 0..40 {
        let (status, body) =
            request_json(app, Method::GET, &path, None, None, Some(auth_context), &[]).await;
        if status != StatusCode::OK {
            return (status, body);
        }
        let state = body["status"].as_str().unwrap_or_default();
        if state == "completed" || state == "failed" {
            return (status, body);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    request_json(app, Method::GET, &path, None, None, Some(auth_context), &[]).await
}

async fn append_session_transcript(
    app: &Router,
    actor: Value,
    items: Value,
    idempotency_key: &str,
    auth_context: (&str, &str),
) -> (StatusCode, Value) {
    request_json(
        app,
        Method::POST,
        "/v1/session-transcripts/append",
        Some(json!({
            "actor": actor,
            "items": items,
        })),
        Some(idempotency_key),
        Some(auth_context),
        &[],
    )
    .await
}

fn setup_app() -> Router {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("temp test path should be created");
    let cfg = make_config(&tmp);
    build_app(cfg).expect("app should build")
}

fn setup_app_at(tmp: &Path) -> Router {
    std::fs::create_dir_all(tmp).expect("temp test path should be created");
    let cfg = make_config(tmp);
    build_app(cfg).expect("app should build")
}

fn setup_app_with(mutator: impl FnOnce(&mut AppConfig)) -> Router {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-custom-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("temp test path should be created");
    let mut cfg = make_config(&tmp);
    mutator(&mut cfg);
    build_app(cfg).expect("app should build")
}

fn setup_app_with_at(tmp: &Path, mutator: impl FnOnce(&mut AppConfig)) -> Router {
    std::fs::create_dir_all(tmp).expect("temp test path should be created");
    let mut cfg = make_config(tmp);
    mutator(&mut cfg);
    build_app(cfg).expect("app should build")
}

async fn set_row_temporal_access_metadata(
    tmp: &Path,
    row_id: &str,
    created_at: i64,
    updated_at: i64,
    access_count: i64,
    last_accessed_at: i64,
) {
    let db_path = tmp.join("lancedb");
    let conn = connect(db_path.to_string_lossy().as_ref())
        .execute()
        .await
        .expect("lancedb should connect for temporal metadata update");
    let table = conn
        .open_table("memories_v1")
        .execute()
        .await
        .expect("memories_v1 should open for temporal metadata update");
    let escaped_id = row_id.replace('\'', "''");
    table
        .update()
        .only_if(format!("id = '{escaped_id}'"))
        .column("created_at", created_at.to_string())
        .column("updated_at", updated_at.to_string())
        .column("access_count", access_count.to_string())
        .column("last_accessed_at", last_accessed_at.to_string())
        .execute()
        .await
        .expect("temporal metadata update should succeed");
}

async fn mark_row_as_behavioral(tmp: &Path, row_id: &str, strict_key: Option<&str>) {
    let db_path = tmp.join("lancedb");
    let conn = connect(db_path.to_string_lossy().as_ref())
        .execute()
        .await
        .expect("lancedb should connect for behavioral row update");
    let table = conn
        .open_table("memories_v1")
        .execute()
        .await
        .expect("memories_v1 should open for behavioral row update");
    let escaped_id = row_id.replace('\'', "''");
    let strict_key_expr = strict_key
        .map(|value| format!("'{}'", value.replace('\'', "''")))
        .unwrap_or_else(|| "NULL".to_string());
    let update_result = table
        .update()
        .only_if(format!("id = '{escaped_id}'"))
        .column("category", "'behavioral'")
        .column("behavioral_kind", "'invariant'")
        .column("strict_key", strict_key_expr)
        .execute()
        .await
        .expect("behavioral row update should succeed");
    assert_eq!(
        update_result.rows_updated, 1,
        "behavioral row update should affect exactly one row"
    );
}

async fn seed_behavioral_memory(
    app: &Router,
    tmp: &Path,
    actor_ctx: (&str, &str, &str, &str),
    text: &str,
    idempotency_key: &str,
) -> String {
    let (user_id, agent_id, session_id, session_key) = actor_ctx;
    let store = json!({
        "actor": actor(user_id, agent_id, session_id, session_key),
        "mode": "tool-store",
        "memory": {
            "text": text,
            "category": "fact"
        }
    });
    let (status, body) = request_json(
        app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some(idempotency_key),
        Some((user_id, agent_id)),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "behavioral seed store failed: {body}"
    );
    let row_id = body["results"][0]["id"]
        .as_str()
        .expect("seeded row id should exist")
        .to_string();
    mark_row_as_behavioral(tmp, &row_id, Some(&format!("behavioral:{row_id}"))).await;
    row_id
}

async fn seed_legacy_table_without_vector(tmp: &Path, rows: &[(&str, &str, &str, &str)]) {
    let schema = Arc::new(Schema::new(vec![
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
    ]));

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_millis() as i64;
    let ids = rows.iter().map(|(id, _, _, _)| *id).collect::<Vec<&str>>();
    let users = rows
        .iter()
        .map(|(_, user, _, _)| *user)
        .collect::<Vec<&str>>();
    let agents = rows
        .iter()
        .map(|(_, _, agent, _)| *agent)
        .collect::<Vec<&str>>();
    let texts = rows
        .iter()
        .map(|(_, _, _, text)| *text)
        .collect::<Vec<&str>>();
    let categories = vec!["fact"; rows.len()];
    let scopes = rows
        .iter()
        .map(|(_, _, agent, _)| format!("agent:{agent}"))
        .collect::<Vec<String>>();
    let importance = vec![0.7_f64; rows.len()];
    let created = vec![now; rows.len()];
    let updated = vec![now; rows.len()];
    let reflection_kind = vec![None::<&str>; rows.len()];
    let strict_key = vec![None::<&str>; rows.len()];

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(users)),
            Arc::new(StringArray::from(agents)),
            Arc::new(StringArray::from(texts)),
            Arc::new(StringArray::from(categories)),
            Arc::new(Float64Array::from(importance)),
            Arc::new(StringArray::from(scopes)),
            Arc::new(Int64Array::from(created)),
            Arc::new(Int64Array::from(updated)),
            Arc::new(StringArray::from(reflection_kind)),
            Arc::new(StringArray::from(strict_key)),
        ],
    )
    .expect("legacy batch should build");

    let db_path = tmp.join("lancedb");
    std::fs::create_dir_all(&db_path).expect("legacy lancedb path should be created");
    let conn = connect(db_path.to_string_lossy().as_ref())
        .execute()
        .await
        .expect("legacy lancedb should connect");
    let _legacy_table = conn
        .create_empty_table("memories_v1", schema.clone())
        .execute()
        .await
        .expect("legacy table should be created");
    let table = conn
        .open_table("memories_v1")
        .execute()
        .await
        .expect("legacy table should open");
    let reader: Box<dyn RecordBatchReader + Send> =
        Box::new(RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema));
    table
        .add(reader)
        .execute()
        .await
        .expect("legacy row should insert");
}

#[derive(Clone)]
struct EmbeddingMockState {
    requests: Arc<Mutex<Vec<Value>>>,
    dimensions: usize,
}

async fn embedding_mock_handler(
    State(state): State<EmbeddingMockState>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    state
        .requests
        .lock()
        .expect("embedding requests lock should be available")
        .push(payload.clone());

    let mut inputs = Vec::new();
    match payload.get("input") {
        Some(Value::String(text)) => inputs.push(text.clone()),
        Some(Value::Array(rows)) => {
            for item in rows {
                if let Some(text) = item.as_str() {
                    inputs.push(text.to_string());
                }
            }
        }
        _ => {}
    }

    let data = inputs
        .iter()
        .enumerate()
        .map(|(idx, text)| {
            json!({
                "index": idx,
                "embedding": mock_embedding_vector(text, state.dimensions),
            })
        })
        .collect::<Vec<_>>();

    Json(json!({
        "object": "list",
        "data": data,
    }))
}

fn mock_embedding_vector(text: &str, dimensions: usize) -> Vec<f32> {
    let mut vector = vec![0.0_f32; dimensions];
    let lowered = text.to_lowercase();
    let axis = if lowered.contains("stellar")
        || lowered.contains("nebula")
        || lowered.contains("orion")
        || lowered.contains("galaxy")
    {
        0
    } else if lowered.contains("ledger")
        || lowered.contains("finance")
        || lowered.contains("budget")
        || lowered.contains("reconciliation")
    {
        1
    } else {
        2
    };
    vector[axis] = 1.0;
    vector
}

async fn spawn_embedding_mock_server(dimensions: usize) -> (String, Arc<Mutex<Vec<Value>>>) {
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock embedding server should bind");
    let base_url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("mock embedding server should have local addr")
    );
    let state = EmbeddingMockState {
        requests: requests.clone(),
        dimensions,
    };
    let app = Router::new()
        .route("/embeddings", post(embedding_mock_handler))
        .with_state(state);
    let _server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock embedding server should run");
    });
    (base_url, requests)
}

#[derive(Clone)]
struct AuthEmbeddingMockRequest {
    authorization: Option<String>,
    body: Value,
}

#[derive(Clone)]
struct AuthEmbeddingMockState {
    requests: Arc<Mutex<Vec<AuthEmbeddingMockRequest>>>,
    dimensions: usize,
    accepted_key: String,
}

async fn auth_embedding_mock_handler(
    State(state): State<AuthEmbeddingMockState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    state
        .requests
        .lock()
        .expect("auth embedding requests lock should be available")
        .push(AuthEmbeddingMockRequest {
            authorization: authorization.clone(),
            body: payload.clone(),
        });

    if authorization.as_deref() != Some(&format!("Bearer {}", state.accepted_key)) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid embedding key" })),
        );
    }

    let mut inputs = Vec::new();
    match payload.get("input") {
        Some(Value::String(text)) => inputs.push(text.clone()),
        Some(Value::Array(rows)) => {
            for item in rows {
                if let Some(text) = item.as_str() {
                    inputs.push(text.to_string());
                }
            }
        }
        _ => {}
    }

    let data = inputs
        .iter()
        .enumerate()
        .map(|(idx, text)| {
            json!({
                "index": idx,
                "embedding": mock_embedding_vector(text, state.dimensions),
            })
        })
        .collect::<Vec<_>>();
    (
        StatusCode::OK,
        Json(json!({
            "object": "list",
            "data": data,
        })),
    )
}

async fn spawn_auth_embedding_mock_server(
    dimensions: usize,
    accepted_key: &str,
) -> (String, Arc<Mutex<Vec<AuthEmbeddingMockRequest>>>) {
    let requests = Arc::new(Mutex::new(Vec::<AuthEmbeddingMockRequest>::new()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("auth embedding server should bind");
    let base_url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("auth embedding server should have local addr")
    );
    let state = AuthEmbeddingMockState {
        requests: requests.clone(),
        dimensions,
        accepted_key: accepted_key.to_string(),
    };
    let app = Router::new()
        .route("/embeddings", post(auth_embedding_mock_handler))
        .with_state(state);
    let _server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("auth embedding server should run");
    });
    (base_url, requests)
}

#[derive(Clone)]
struct ContextLimitEmbeddingMockRequest {
    input_count: usize,
    max_input_chars: usize,
}

#[derive(Clone)]
struct ContextLimitEmbeddingMockState {
    requests: Arc<Mutex<Vec<ContextLimitEmbeddingMockRequest>>>,
    dimensions: usize,
    max_chars: usize,
    fail_always: bool,
}

async fn context_limit_embedding_mock_handler(
    State(state): State<ContextLimitEmbeddingMockState>,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let mut inputs = Vec::new();
    match payload.get("input") {
        Some(Value::String(text)) => inputs.push(text.clone()),
        Some(Value::Array(rows)) => {
            for item in rows {
                if let Some(text) = item.as_str() {
                    inputs.push(text.to_string());
                }
            }
        }
        _ => {}
    }
    let max_input_chars = inputs
        .iter()
        .map(|text| text.chars().count())
        .max()
        .unwrap_or(0);

    state
        .requests
        .lock()
        .expect("context limit embedding requests lock should be available")
        .push(ContextLimitEmbeddingMockRequest {
            input_count: inputs.len(),
            max_input_chars,
        });

    if state.fail_always || max_input_chars > state.max_chars {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "message": "maximum context length exceeded"
                }
            })),
        );
    }

    let data = inputs
        .iter()
        .enumerate()
        .map(|(idx, text)| {
            json!({
                "index": idx,
                "embedding": mock_embedding_vector(text, state.dimensions),
            })
        })
        .collect::<Vec<_>>();

    (
        StatusCode::OK,
        Json(json!({
            "object": "list",
            "data": data,
        })),
    )
}

async fn spawn_context_limit_embedding_mock_server(
    dimensions: usize,
    max_chars: usize,
    fail_always: bool,
) -> (String, Arc<Mutex<Vec<ContextLimitEmbeddingMockRequest>>>) {
    let requests = Arc::new(Mutex::new(Vec::<ContextLimitEmbeddingMockRequest>::new()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("context limit embedding server should bind");
    let base_url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("context limit embedding server should have local addr")
    );
    let state = ContextLimitEmbeddingMockState {
        requests: requests.clone(),
        dimensions,
        max_chars,
        fail_always,
    };
    let app = Router::new()
        .route("/embeddings", post(context_limit_embedding_mock_handler))
        .with_state(state);
    let _server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("context limit embedding server should run");
    });
    (base_url, requests)
}

#[derive(Clone)]
struct RerankMockRequest {
    authorization: Option<String>,
    body: Value,
}

#[derive(Clone)]
struct RerankMockState {
    requests: Arc<Mutex<Vec<RerankMockRequest>>>,
}

async fn rerank_mock_handler(
    State(state): State<RerankMockState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    state
        .requests
        .lock()
        .expect("rerank requests lock should be available")
        .push(RerankMockRequest {
            authorization,
            body: payload.clone(),
        });

    let doc_count = payload
        .get("documents")
        .and_then(|docs| docs.as_array())
        .map(|docs| docs.len())
        .unwrap_or(0);
    let mut results = Vec::new();
    for (rank, idx) in (0..doc_count).rev().enumerate() {
        let score = (1.0_f64 - rank as f64 * 0.1).max(0.0);
        results.push(json!({
            "index": idx,
            "relevance_score": score,
        }));
    }
    Json(json!({ "results": results }))
}

async fn spawn_rerank_mock_server() -> (String, Arc<Mutex<Vec<RerankMockRequest>>>) {
    let requests = Arc::new(Mutex::new(Vec::<RerankMockRequest>::new()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock rerank server should bind");
    let base_url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("mock rerank server should have local addr")
    );
    let state = RerankMockState {
        requests: requests.clone(),
    };
    let app = Router::new()
        .route("/rerank", post(rerank_mock_handler))
        .with_state(state);
    let _server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock rerank server should run");
    });
    (base_url, requests)
}

#[derive(Clone)]
struct AuthRerankMockState {
    requests: Arc<Mutex<Vec<RerankMockRequest>>>,
    accepted_key: Option<String>,
    non_retryable_bad_request: bool,
}

async fn auth_rerank_mock_handler(
    State(state): State<AuthRerankMockState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    state
        .requests
        .lock()
        .expect("auth rerank requests lock should be available")
        .push(RerankMockRequest {
            authorization: authorization.clone(),
            body: payload.clone(),
        });

    if state.non_retryable_bad_request {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "non-retryable rerank payload error" })),
        );
    }

    if let Some(expected) = &state.accepted_key {
        if authorization.as_deref() != Some(&format!("Bearer {expected}")) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "invalid rerank key" })),
            );
        }
    }

    let doc_count = payload
        .get("documents")
        .and_then(|docs| docs.as_array())
        .map(|docs| docs.len())
        .unwrap_or(0);
    let mut results = Vec::new();
    for (rank, idx) in (0..doc_count).rev().enumerate() {
        let score = (1.0_f64 - rank as f64 * 0.1).max(0.0);
        results.push(json!({
            "index": idx,
            "relevance_score": score,
        }));
    }
    (StatusCode::OK, Json(json!({ "results": results })))
}

async fn spawn_auth_rerank_mock_server(
    accepted_key: Option<&str>,
    non_retryable_bad_request: bool,
) -> (String, Arc<Mutex<Vec<RerankMockRequest>>>) {
    let requests = Arc::new(Mutex::new(Vec::<RerankMockRequest>::new()));
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("auth rerank server should bind");
    let base_url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("auth rerank server should have local addr")
    );
    let state = AuthRerankMockState {
        requests: requests.clone(),
        accepted_key: accepted_key.map(|value| value.to_string()),
        non_retryable_bad_request,
    };
    let app = Router::new()
        .route("/rerank", post(auth_rerank_mock_handler))
        .with_state(state);
    let _server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("auth rerank server should run");
    });
    (base_url, requests)
}

#[test]
fn invalid_embedding_dimensions_config_is_rejected() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-invalid-config-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("temp test path should be created");
    let mut cfg = make_config(&tmp);
    cfg.providers.embedding.dimensions = 8;

    let err = build_app(cfg).expect_err("invalid config must fail app build");
    let message = format!("{err:#}");
    assert!(
        message.contains("providers.embedding.dimensions"),
        "validation error should point to embedding dimensions, got: {message}"
    );
}

#[path = "contract_semantics/diagnostics_auth_and_persistence.rs"]
mod diagnostics_auth_and_persistence;
#[path = "contract_semantics/distill_contracts.rs"]
mod distill_contracts;
#[path = "contract_semantics/memory_contracts.rs"]
mod memory_contracts;
#[path = "contract_semantics/provider_and_retrieval.rs"]
mod provider_and_retrieval;
