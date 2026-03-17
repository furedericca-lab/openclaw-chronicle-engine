use arrow_array::{Float64Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{DataType, Field, Schema};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{header, Method, Request, StatusCode},
    routing::post,
    Json, Router,
};
use lancedb::{connect, index::IndexType};
use memory_lancedb_pro_backend::{
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
        auth_context,
        RUNTIME_TOKEN,
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
    auth_context: Option<(&str, &str)>,
    bearer_token: &str,
    extra_headers: &[(&str, &str)],
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header(header::AUTHORIZATION, format!("Bearer {bearer_token}"))
        .header("x-request-id", "req-test-1");

    if let Some((auth_user, auth_agent)) = auth_context {
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
        "memory-lancedb-pro-backend-test-{}",
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
        "memory-lancedb-pro-backend-test-custom-{}",
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
    let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
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
        "memory-lancedb-pro-backend-test-invalid-config-{}",
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

#[tokio::test]
async fn openai_compatible_embedding_provider_is_used_for_recall() {
    let (embedding_base_url, requests) = spawn_embedding_mock_server(64).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "mock-embedding-64".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let relevant_store = json!({
        "actor": actor("u1", "main", "sess-embed-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Orion nebula observation notes for telescope calibration."
        }
    });
    let (status, first_store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(relevant_store),
        Some("idem-openai-embed-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "first store request failed: {first_store_body}"
    );

    let irrelevant_store = json!({
        "actor": actor("u1", "main", "sess-embed-2", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Ledger reconciliation worksheet for quarterly finance close."
        }
    });
    let (status, second_store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(irrelevant_store),
        Some("idem-openai-embed-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "second store request failed: {second_store_body}"
    );

    let recall = json!({
        "actor": actor("u1", "main", "sess-embed-3", "session-key-1"),
        "query": "stellar cloud telescope",
        "limit": 2
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"].as_array().expect("rows should be an array");
    assert!(!rows.is_empty(), "recall should return at least one row");
    let top_text = rows[0]["text"].as_str().unwrap_or_default().to_lowercase();
    assert!(
        top_text.contains("orion") || top_text.contains("nebula"),
        "openai-compatible embedding path should rank the astronomy memory first"
    );

    let captured = requests
        .lock()
        .expect("embedding requests lock should be readable")
        .clone();
    assert!(
        captured.len() >= 3,
        "embedding provider should be called for both writes and recall query"
    );
    assert!(captured.iter().all(|payload| {
        payload.get("model").and_then(|value| value.as_str()) == Some("mock-embedding-64")
            && payload.get("dimensions").and_then(|value| value.as_u64()) == Some(64)
    }));
    let query_seen = captured.iter().any(|payload| match payload.get("input") {
        Some(Value::String(text)) => text == "stellar cloud telescope",
        Some(Value::Array(rows)) => rows
            .iter()
            .any(|item| item.as_str() == Some("stellar cloud telescope")),
        _ => false,
    });
    assert!(
        query_seen,
        "embedding provider should receive recall query input"
    );
}

#[tokio::test]
async fn openai_compatible_embedding_provider_failure_returns_upstream_error() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failing embedding server should bind");
    let base_url = format!(
        "http://{}",
        listener
            .local_addr()
            .expect("failing embedding server should have local addr")
    );
    let app_server = Router::new().route(
        "/embeddings",
        post(|| async {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"embedding provider unavailable"})),
            )
        }),
    );
    let _server = tokio::spawn(async move {
        axum::serve(listener, app_server)
            .await
            .expect("failing embedding server should run");
    });

    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "mock-embedding-64".to_string();
        cfg.providers.embedding.base_url = Some(base_url.clone());
        cfg.providers.embedding.dimensions = 64;
    });

    let store = json!({
        "actor": actor("u1", "main", "sess-embed-fail-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "This write should fail because embedding provider returns 500."
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-openai-embed-fail-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "UPSTREAM_EMBEDDING_ERROR");
}

#[tokio::test]
async fn openai_compatible_embedding_context_limit_recovers_with_chunking() {
    let max_chars = 1200usize;
    let (embedding_base_url, requests) =
        spawn_context_limit_embedding_mock_server(64, max_chars, false).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "all-MiniLM-L6-v2".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let long_text = "Long embedding input segment for context recovery validation. ".repeat(40);
    let store = json!({
        "actor": actor("u1", "main", "sess-embed-chunk-1", "session-key-embed-chunk"),
        "mode": "tool-store",
        "memory": {
            "text": long_text
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-openai-embed-chunk-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "store should recover via chunking: {body}"
    );

    let captured = requests
        .lock()
        .expect("context limit embedding requests should be readable")
        .clone();
    assert!(
        captured.len() >= 2,
        "context-limit request should trigger at least one recovery attempt"
    );
    assert!(
        captured.iter().any(|req| req.max_input_chars > max_chars),
        "initial request should include an over-limit input"
    );
    assert!(
        captured
            .iter()
            .any(|req| req.input_count > 1 && req.max_input_chars <= max_chars),
        "chunk recovery should submit bounded chunk batch inputs"
    );
}

#[tokio::test]
async fn openai_compatible_embedding_context_limit_recovery_failure_returns_upstream_error() {
    let (embedding_base_url, requests) =
        spawn_context_limit_embedding_mock_server(64, 80, true).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "all-MiniLM-L6-v2".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
    });

    let long_text =
        "Context overflow should remain failing when provider rejects every chunk. ".repeat(30);
    let store = json!({
        "actor": actor("u1", "main", "sess-embed-chunk-fail-1", "session-key-embed-chunk-fail"),
        "mode": "tool-store",
        "memory": {
            "text": long_text
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-openai-embed-chunk-fail-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "store should fail when chunk recovery cannot succeed"
    );
    assert_eq!(body["error"]["code"], "UPSTREAM_EMBEDDING_ERROR");

    let captured = requests
        .lock()
        .expect("context limit embedding requests should be readable")
        .clone();
    assert!(
        captured.len() >= 2,
        "failing context-limit path should attempt chunk recovery before failing"
    );
}

#[tokio::test]
async fn cross_encoder_rerank_provider_can_reorder_candidates() {
    let (rerank_base_url, rerank_requests) = spawn_rerank_mock_server().await;
    let app = setup_app_with(|cfg| {
        cfg.providers.rerank.enabled = true;
        cfg.providers.rerank.mode = "cross-encoder".to_string();
        cfg.providers.rerank.provider = "jina".to_string();
        cfg.providers.rerank.base_url = Some(format!("{rerank_base_url}/rerank"));
        cfg.providers.rerank.api_key = Some("rerank-test-key".to_string());
        cfg.providers.rerank.blend = 1.0;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let primary_store = json!({
        "actor": actor("u1", "main", "sess-rerank-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Rollout checklist for deployment guardrails and staged release validation."
        }
    });
    let (status, primary_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(primary_store),
        Some("idem-rerank-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let primary_id = primary_body["results"][0]["id"]
        .as_str()
        .expect("primary memory id should exist")
        .to_string();

    let secondary_store = json!({
        "actor": actor("u1", "main", "sess-rerank-2", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Rollout lunch menu notes with tea and snacks for the operations team."
        }
    });
    let (status, secondary_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(secondary_store),
        Some("idem-rerank-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let secondary_id = secondary_body["results"][0]["id"]
        .as_str()
        .expect("secondary memory id should exist")
        .to_string();

    let recall = json!({
        "actor": actor("u1", "main", "sess-rerank-3", "session-key-1"),
        "query": "deployment rollout guardrails checklist",
        "limit": 2
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"].as_array().expect("rows should be an array");
    assert!(rows.len() >= 2, "rerank test should return both candidates");
    assert_eq!(
        rows[0]["id"].as_str().expect("top row id should exist"),
        secondary_id,
        "cross-encoder rerank should reorder candidates based on provider signal"
    );
    assert_ne!(primary_id, secondary_id);

    let captured = rerank_requests
        .lock()
        .expect("rerank requests lock should be readable")
        .clone();
    assert!(!captured.is_empty(), "rerank provider should be called");
    let first = &captured[0];
    assert_eq!(
        first.authorization.as_deref(),
        Some("Bearer rerank-test-key")
    );
    assert_eq!(
        first.body.get("query").and_then(|value| value.as_str()),
        Some("deployment rollout guardrails checklist")
    );
    let doc_count = first
        .body
        .get("documents")
        .and_then(|value| value.as_array())
        .map(|docs| docs.len())
        .unwrap_or(0);
    assert!(
        doc_count >= 2,
        "rerank provider should receive candidate documents"
    );
}

#[tokio::test]
async fn rerank_provider_rotates_keys_and_fails_over_on_auth_error() {
    let (rerank_base_url, rerank_requests) =
        spawn_auth_rerank_mock_server(Some("good-rerank-key"), false).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.rerank.enabled = true;
        cfg.providers.rerank.mode = "cross-encoder".to_string();
        cfg.providers.rerank.provider = "jina".to_string();
        cfg.providers.rerank.base_url = Some(format!("{rerank_base_url}/rerank"));
        cfg.providers.rerank.api_key = Some("bad-rerank-key,good-rerank-key".to_string());
        cfg.providers.rerank.blend = 1.0;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let primary_store = json!({
        "actor": actor("u1", "main", "sess-rerank-rotate-1", "session-key-rerank-rotate"),
        "mode": "tool-store",
        "memory": {
            "text": "Production deploy guardrail checklist for staged release rollout."
        }
    });
    let (status, primary_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(primary_store),
        Some("idem-rerank-rotate-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "primary store failed: {primary_body}"
    );
    let primary_id = primary_body["results"][0]["id"]
        .as_str()
        .expect("primary memory id should exist")
        .to_string();

    let secondary_store = json!({
        "actor": actor("u1", "main", "sess-rerank-rotate-2", "session-key-rerank-rotate"),
        "mode": "tool-store",
        "memory": {
            "text": "Rollout tea and snack notes for operations social planning."
        }
    });
    let (status, secondary_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(secondary_store),
        Some("idem-rerank-rotate-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "secondary store failed: {secondary_body}"
    );
    let secondary_id = secondary_body["results"][0]["id"]
        .as_str()
        .expect("secondary memory id should exist")
        .to_string();

    let recall = json!({
        "actor": actor("u1", "main", "sess-rerank-rotate-3", "session-key-rerank-rotate"),
        "query": "staged deploy rollout guardrail checklist",
        "limit": 2
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "recall failed: {body}");
    let rows = body["rows"].as_array().expect("rows should be array");
    assert!(rows.len() >= 2, "rerank recall should return both memories");
    assert_eq!(
        rows[0]["id"].as_str().expect("top row id should exist"),
        secondary_id,
        "successful failover rerank should still apply provider ordering"
    );
    assert_ne!(primary_id, secondary_id);

    let captured = rerank_requests
        .lock()
        .expect("auth rerank requests should be readable")
        .clone();
    assert_eq!(
        captured.len(),
        2,
        "rerank provider should retry once with backup key after auth failure"
    );
    assert_eq!(
        captured[0].authorization.as_deref(),
        Some("Bearer bad-rerank-key"),
        "first rerank attempt should use first configured key"
    );
    assert_eq!(
        captured[1].authorization.as_deref(),
        Some("Bearer good-rerank-key"),
        "second rerank attempt should rotate to backup key"
    );
}

#[tokio::test]
async fn rerank_provider_does_not_rotate_on_non_retryable_error() {
    let (rerank_base_url, rerank_requests) = spawn_auth_rerank_mock_server(None, true).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.rerank.enabled = true;
        cfg.providers.rerank.mode = "cross-encoder".to_string();
        cfg.providers.rerank.provider = "jina".to_string();
        cfg.providers.rerank.base_url = Some(format!("{rerank_base_url}/rerank"));
        cfg.providers.rerank.api_key = Some("first-key,second-key".to_string());
        cfg.providers.rerank.blend = 1.0;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let store_a = json!({
        "actor": actor("u1", "main", "sess-rerank-nonretry-1", "session-key-rerank-nonretry"),
        "mode": "tool-store",
        "memory": {
            "text": "Deployment rollout hardening checklist for production incidents."
        }
    });
    let (status, store_a_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store_a),
        Some("idem-rerank-nonretry-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store A failed: {store_a_body}");

    let store_b = json!({
        "actor": actor("u1", "main", "sess-rerank-nonretry-2", "session-key-rerank-nonretry"),
        "mode": "tool-store",
        "memory": {
            "text": "Social lunch planning note for the release team."
        }
    });
    let (status, store_b_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store_b),
        Some("idem-rerank-nonretry-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store B failed: {store_b_body}");

    let recall = json!({
        "actor": actor("u1", "main", "sess-rerank-nonretry-3", "session-key-rerank-nonretry"),
        "query": "production rollout checklist",
        "limit": 2
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "recall should fall back to lightweight: {body}"
    );
    assert!(
        body["rows"]
            .as_array()
            .map(|rows| !rows.is_empty())
            .unwrap_or(false),
        "recall should still return rows after rerank fallback"
    );

    let captured = rerank_requests
        .lock()
        .expect("auth rerank requests should be readable")
        .clone();
    assert_eq!(
        captured.len(),
        1,
        "non-retryable rerank failure should not rotate through backup keys"
    );
    assert_eq!(
        captured[0].authorization.as_deref(),
        Some("Bearer first-key"),
        "only the first key should be attempted on non-retryable failure"
    );
}

#[tokio::test]
async fn embedding_provider_cache_reuses_vectors_across_write_and_recall() {
    let (embedding_base_url, requests) = spawn_embedding_mock_server(64).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "mock-embedding-64".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
        cfg.providers.embedding.cache_max_entries = 512;
        cfg.providers.embedding.cache_ttl_ms = 60_000;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let text = "Cache probe memory about deployment readiness checks.";
    let store = json!({
        "actor": actor("u1", "main", "sess-cache-1", "session-key-cache"),
        "mode": "tool-store",
        "memory": {
            "text": text
        }
    });
    let (status, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-cache-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {store_body}");

    let recall = json!({
        "actor": actor("u1", "main", "sess-cache-2", "session-key-cache"),
        "query": text,
        "limit": 1
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "recall failed: {body}");
    assert_eq!(body["rows"][0]["text"], text);

    let captured = requests
        .lock()
        .expect("embedding requests lock should be readable")
        .clone();
    assert_eq!(
        captured.len(),
        1,
        "embedding cache should reuse vector across write+recall for identical text"
    );
}

#[tokio::test]
async fn embedding_provider_rotates_keys_and_fails_over_on_auth_error() {
    let (embedding_base_url, requests) = spawn_auth_embedding_mock_server(64, "good-key").await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "mock-embedding-64".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
        cfg.providers.embedding.api_key = Some("bad-key,good-key".to_string());
    });

    let store = json!({
        "actor": actor("u1", "main", "sess-rotate-1", "session-key-rotate"),
        "mode": "tool-store",
        "memory": {
            "text": "Failover key rotation should recover from bad credentials."
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-rotate-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {body}");

    let captured = requests
        .lock()
        .expect("auth embedding requests lock should be readable")
        .clone();
    assert_eq!(
        captured.len(),
        2,
        "embedding provider should retry with next configured key"
    );
    assert_eq!(
        captured[0].authorization.as_deref(),
        Some("Bearer bad-key"),
        "first request should use first key"
    );
    assert_eq!(
        captured[1].authorization.as_deref(),
        Some("Bearer good-key"),
        "second request should rotate to backup key"
    );
    assert_eq!(
        captured[1]
            .body
            .get("model")
            .and_then(|value| value.as_str()),
        Some("mock-embedding-64")
    );
}

#[tokio::test]
async fn legacy_table_without_vector_column_is_migrated_without_data_loss() {
    let tmp = std::env::temp_dir().join(format!(
        "memory-lancedb-pro-backend-legacy-migration-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("legacy temp path should be created");
    seed_legacy_table_without_vector(
        &tmp,
        &[(
            "legacy_mem_1",
            "u1",
            "main",
            "Legacy memory row created before vector column existed.",
        )],
    )
    .await;

    let app = setup_app_at(&tmp);
    let stats_payload = json!({
        "actor": actor("u1", "main", "sess-legacy-1", "session-key-legacy")
    });
    let (status, stats_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/stats",
        Some(stats_payload),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "stats failed: {stats_body}");
    assert_eq!(stats_body["memoryCount"], 1);

    let store = json!({
        "actor": actor("u1", "main", "sess-legacy-2", "session-key-legacy"),
        "mode": "tool-store",
        "memory": {
            "text": "New memory after migration should still be writable."
        }
    });
    let (status, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-legacy-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {store_body}");

    let stats_payload = json!({
        "actor": actor("u1", "main", "sess-legacy-3", "session-key-legacy")
    });
    let (status, stats_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/stats",
        Some(stats_payload),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "stats failed: {stats_body}");
    assert_eq!(stats_body["memoryCount"], 2);

    let db_path = tmp.join("lancedb");
    let conn = connect(db_path.to_string_lossy().as_ref())
        .execute()
        .await
        .expect("lancedb should connect for verification");
    let table = conn
        .open_table("memories_v1")
        .execute()
        .await
        .expect("memories_v1 should exist after migration");
    let schema = table
        .schema()
        .await
        .expect("schema should be readable after migration");
    assert!(
        schema.field_with_name("vector").is_ok(),
        "migrated table should include vector column"
    );
    let table_names = conn
        .table_names()
        .execute()
        .await
        .expect("table names should be readable");
    assert!(
        table_names
            .iter()
            .any(|name| name.starts_with("memories_v1_legacy_backup_")),
        "legacy migration should preserve an auditable backup table"
    );
}

#[tokio::test]
async fn lancedb_search_indices_are_explicitly_ensured() {
    let tmp = std::env::temp_dir().join(format!(
        "memory-lancedb-pro-backend-index-lifecycle-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_at(&tmp);

    let store = json!({
        "actor": actor("u1", "main", "sess-index-1", "session-key-index"),
        "mode": "tool-store",
        "memory": {
            "text": "Vector and lexical index lifecycle validation memory row."
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-index-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {body}");

    let db_path = tmp.join("lancedb");
    let conn = connect(db_path.to_string_lossy().as_ref())
        .execute()
        .await
        .expect("lancedb should connect for index checks");
    let table = conn
        .open_table("memories_v1")
        .execute()
        .await
        .expect("memories_v1 should open");
    let indices = table
        .list_indices()
        .await
        .expect("list_indices should succeed");

    let has_fts = indices.iter().any(|index| {
        index.index_type == IndexType::FTS
            && index.columns.iter().any(|column| column.as_str() == "text")
    });
    assert!(has_fts, "text FTS index should be explicitly ensured");

    let has_vector = indices.iter().any(|index| {
        index
            .columns
            .iter()
            .any(|column| column.as_str() == "vector")
            && matches!(
                index.index_type,
                IndexType::IvfFlat
                    | IndexType::IvfSq
                    | IndexType::IvfPq
                    | IndexType::IvfRq
                    | IndexType::IvfHnswPq
                    | IndexType::IvfHnswSq
            )
    });
    assert!(has_vector, "vector ANN index should be explicitly ensured");
}

#[tokio::test]
async fn query_expansion_and_noise_filtering_improve_generic_recall() {
    let app = setup_app_with(|cfg| {
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let useful = json!({
        "actor": actor("u1", "main", "sess-expand-1", "session-key-expand"),
        "mode": "tool-store",
        "memory": {
            "text": "Timeout remediation playbook for API gateway rollout incidents."
        }
    });
    let (status, useful_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(useful),
        Some("idem-expand-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "useful store failed: {useful_body}");

    let noise = json!({
        "actor": actor("u1", "main", "sess-expand-2", "session-key-expand"),
        "mode": "tool-store",
        "memory": {
            "text": "I don't recall any relevant memories found for this request."
        }
    });
    let (status, noise_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(noise),
        Some("idem-expand-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "noise store failed: {noise_body}");

    let recall = json!({
        "actor": actor("u1", "main", "sess-expand-3", "session-key-expand"),
        "query": "service hung during deploy",
        "limit": 5
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "recall failed: {body}");
    let rows = body["rows"].as_array().expect("rows should be array");
    assert!(
        rows.iter().any(|row| {
            row["text"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase()
                .contains("timeout")
        }),
        "expanded retrieval should surface timeout-oriented memory for 'hung' query"
    );
    assert!(
        rows.iter().all(|row| {
            !row["text"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase()
                .contains("don't recall any relevant memories found")
        }),
        "noise-filtered recall should remove denial-like memory rows"
    );
}

#[tokio::test]
async fn access_reinforcement_extends_time_decay_for_old_memories() {
    let tmp = std::env::temp_dir().join(format!(
        "memory-lancedb-pro-backend-access-reinforcement-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_with_at(&tmp, |cfg| {
        cfg.providers.rerank.enabled = false;
        cfg.providers.rerank.mode = "none".to_string();
        cfg.retrieval.query_expansion = false;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
        cfg.retrieval.recency_weight = 0.0;
        cfg.retrieval.time_decay_half_life_days = 30.0;
        cfg.retrieval.reinforcement_factor = 0.8;
        cfg.retrieval.max_half_life_multiplier = 3.0;
    });

    let text = "Orion deploy rollback checklist for gateway release guardrails.";
    let first_store = json!({
        "actor": actor("u1", "main", "sess-access-decay-1", "session-key-access-decay"),
        "mode": "tool-store",
        "memory": { "text": text }
    });
    let (status, first_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(first_store),
        Some("idem-access-decay-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "first store failed: {first_body}");
    let reinforced_id = first_body["results"][0]["id"]
        .as_str()
        .expect("reinforced row id should exist")
        .to_string();

    let second_store = json!({
        "actor": actor("u1", "main", "sess-access-decay-2", "session-key-access-decay"),
        "mode": "tool-store",
        "memory": { "text": text }
    });
    let (status, second_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(second_store),
        Some("idem-access-decay-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "second store failed: {second_body}");
    let stale_id = second_body["results"][0]["id"]
        .as_str()
        .expect("stale row id should exist")
        .to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_millis() as i64;
    let stale_ts = now - 45 * 86_400_000;
    set_row_temporal_access_metadata(&tmp, &reinforced_id, stale_ts, stale_ts, 4_000, now).await;
    set_row_temporal_access_metadata(&tmp, &stale_id, stale_ts, stale_ts, 0, 0).await;

    let recall = json!({
        "actor": actor("u1", "main", "sess-access-decay-3", "session-key-access-decay"),
        "query": text,
        "limit": 2
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "recall failed: {body}");

    let rows = body["rows"].as_array().expect("rows should be array");
    assert_eq!(rows.len(), 2, "recall should return both duplicate rows");
    let reinforced_score = rows
        .iter()
        .find(|row| row["id"].as_str() == Some(reinforced_id.as_str()))
        .and_then(|row| row["score"].as_f64())
        .expect("reinforced row score should exist");
    let stale_score = rows
        .iter()
        .find(|row| row["id"].as_str() == Some(stale_id.as_str()))
        .and_then(|row| row["score"].as_f64())
        .expect("stale row score should exist");
    assert!(
        reinforced_score > stale_score,
        "recently-accessed stale memory should decay slower and score higher"
    );
}

#[tokio::test]
async fn access_reinforcement_respects_max_half_life_multiplier_bound() {
    let tmp = std::env::temp_dir().join(format!(
        "memory-lancedb-pro-backend-access-bound-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_with_at(&tmp, |cfg| {
        cfg.providers.rerank.enabled = false;
        cfg.providers.rerank.mode = "none".to_string();
        cfg.retrieval.query_expansion = false;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
        cfg.retrieval.recency_weight = 0.0;
        cfg.retrieval.time_decay_half_life_days = 30.0;
        cfg.retrieval.reinforcement_factor = 1.0;
        cfg.retrieval.max_half_life_multiplier = 1.0;
    });

    let text = "Orion deploy rollback checklist for bounded reinforcement validation.";
    let first_store = json!({
        "actor": actor("u1", "main", "sess-access-bound-1", "session-key-access-bound"),
        "mode": "tool-store",
        "memory": { "text": text }
    });
    let (status, first_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(first_store),
        Some("idem-access-bound-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "first store failed: {first_body}");
    let reinforced_id = first_body["results"][0]["id"]
        .as_str()
        .expect("reinforced row id should exist")
        .to_string();

    let second_store = json!({
        "actor": actor("u1", "main", "sess-access-bound-2", "session-key-access-bound"),
        "mode": "tool-store",
        "memory": { "text": text }
    });
    let (status, second_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(second_store),
        Some("idem-access-bound-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "second store failed: {second_body}");
    let stale_id = second_body["results"][0]["id"]
        .as_str()
        .expect("stale row id should exist")
        .to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be valid")
        .as_millis() as i64;
    let stale_ts = now - 45 * 86_400_000;
    set_row_temporal_access_metadata(&tmp, &reinforced_id, stale_ts, stale_ts, 9_999, now).await;
    set_row_temporal_access_metadata(&tmp, &stale_id, stale_ts, stale_ts, 0, 0).await;

    let recall = json!({
        "actor": actor("u1", "main", "sess-access-bound-3", "session-key-access-bound"),
        "query": text,
        "limit": 2
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "recall failed: {body}");

    let rows = body["rows"].as_array().expect("rows should be array");
    assert_eq!(rows.len(), 2, "recall should return both duplicate rows");
    let reinforced_score = rows
        .iter()
        .find(|row| row["id"].as_str() == Some(reinforced_id.as_str()))
        .and_then(|row| row["score"].as_f64())
        .expect("reinforced row score should exist");
    let stale_score = rows
        .iter()
        .find(|row| row["id"].as_str() == Some(stale_id.as_str()))
        .and_then(|row| row["score"].as_f64())
        .expect("stale row score should exist");
    assert!(
        (reinforced_score - stale_score).abs() <= 1e-6,
        "maxHalfLifeMultiplier=1 should prevent any reinforcement uplift"
    );
}

#[tokio::test]
async fn mmr_diversity_reduces_duplicate_topk_deterministically() {
    let (embedding_base_url, _) = spawn_embedding_mock_server(64).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "mock-embedding-64".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
        cfg.providers.rerank.enabled = false;
        cfg.providers.rerank.mode = "none".to_string();
        cfg.retrieval.query_expansion = false;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
        cfg.retrieval.reinforcement_factor = 0.0;
        cfg.retrieval.mmr_diversity = true;
        cfg.retrieval.mmr_similarity_threshold = 0.85;
    });

    let duplicate_a = json!({
        "actor": actor("u1", "main", "sess-mmr-1", "session-key-mmr"),
        "mode": "tool-store",
        "memory": {
            "text": "Orion deploy rollback checklist for gateway release guardrails."
        }
    });
    let (status, duplicate_a_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(duplicate_a),
        Some("idem-mmr-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "duplicate A store failed: {duplicate_a_body}"
    );
    let duplicate_a_id = duplicate_a_body["results"][0]["id"]
        .as_str()
        .expect("duplicate A id should exist")
        .to_string();

    let duplicate_b = json!({
        "actor": actor("u1", "main", "sess-mmr-2", "session-key-mmr"),
        "mode": "tool-store",
        "memory": {
            "text": "Orion deploy rollback checklist duplicate note for gateway release guardrails."
        }
    });
    let (status, duplicate_b_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(duplicate_b),
        Some("idem-mmr-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "duplicate B store failed: {duplicate_b_body}"
    );
    let duplicate_b_id = duplicate_b_body["results"][0]["id"]
        .as_str()
        .expect("duplicate B id should exist")
        .to_string();

    let diverse = json!({
        "actor": actor("u1", "main", "sess-mmr-3", "session-key-mmr"),
        "mode": "tool-store",
        "memory": {
            "text": "Deploy rollback checklist handbook for gateway release guardrails."
        }
    });
    let (status, diverse_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(diverse),
        Some("idem-mmr-store-3"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "diverse store failed: {diverse_body}"
    );
    let diverse_id = diverse_body["results"][0]["id"]
        .as_str()
        .expect("diverse id should exist")
        .to_string();

    let recall = json!({
        "actor": actor("u1", "main", "sess-mmr-4", "session-key-mmr"),
        "query": "orion deploy rollback checklist",
        "limit": 2
    });
    let (status, first_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall.clone()),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "first recall failed: {first_body}");
    let first_rows = first_body["rows"].as_array().expect("rows should be array");
    assert_eq!(
        first_rows.len(),
        2,
        "limit=2 should return exactly two rows"
    );
    let first_ids = first_rows
        .iter()
        .map(|row| row["id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(
        first_ids.contains(&diverse_id),
        "MMR should keep one diverse result in top-k"
    );
    let duplicate_count = first_ids
        .iter()
        .filter(|id| *id == &duplicate_a_id || *id == &duplicate_b_id)
        .count();
    assert_eq!(
        duplicate_count, 1,
        "MMR top-k should include only one of the near-duplicate pair"
    );

    let (status, second_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "second recall failed: {second_body}"
    );
    let second_ids = second_body["rows"]
        .as_array()
        .expect("rows should be array")
        .iter()
        .map(|row| row["id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        first_ids, second_ids,
        "MMR output ordering should remain deterministic across repeated recalls"
    );
}

#[tokio::test]
async fn embedding_tuning_knobs_are_sent_for_compatible_provider_assumptions() {
    let (embedding_base_url, requests) = spawn_embedding_mock_server(64).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "jina-embeddings-v5-text-small".to_string();
        cfg.providers.embedding.api = "jina".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
        cfg.providers.embedding.task_query = Some("retrieval.query".to_string());
        cfg.providers.embedding.task_passage = Some("retrieval.passage".to_string());
        cfg.providers.embedding.normalized = Some(true);
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let passage_text = "Embedding passage knob payload verification for provider-compatible mode.";
    let store = json!({
        "actor": actor("u1", "main", "sess-embed-knob-1", "session-key-embed-knob"),
        "mode": "tool-store",
        "memory": { "text": passage_text }
    });
    let (status, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-embed-knob-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {store_body}");

    let query_text = "provider-compatible query knob verification";
    let recall = json!({
        "actor": actor("u1", "main", "sess-embed-knob-2", "session-key-embed-knob"),
        "query": query_text,
        "limit": 1
    });
    let (status, recall_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "recall failed: {recall_body}");

    let captured = requests
        .lock()
        .expect("embedding requests lock should be readable")
        .clone();
    assert!(
        captured.len() >= 2,
        "store + recall should both call embedding provider"
    );

    let passage_payload = captured
        .iter()
        .find(|payload| payload.get("input").and_then(|v| v.as_str()) == Some(passage_text))
        .expect("passage embedding payload should exist");
    assert_eq!(
        passage_payload.get("task").and_then(|value| value.as_str()),
        Some("retrieval.passage")
    );
    assert_eq!(
        passage_payload
            .get("normalized")
            .and_then(|value| value.as_bool()),
        Some(true)
    );

    let query_payload = captured
        .iter()
        .find(|payload| payload.get("input").and_then(|v| v.as_str()) == Some(query_text))
        .expect("query embedding payload should exist");
    assert_eq!(
        query_payload.get("task").and_then(|value| value.as_str()),
        Some("retrieval.query")
    );
    assert_eq!(
        query_payload
            .get("normalized")
            .and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[tokio::test]
async fn embedding_tuning_knobs_are_omitted_when_provider_contract_is_not_compatible() {
    let (embedding_base_url, requests) = spawn_embedding_mock_server(64).await;
    let app = setup_app_with(|cfg| {
        cfg.providers.embedding.provider = "openai-compatible".to_string();
        cfg.providers.embedding.model = "text-embedding-3-small".to_string();
        cfg.providers.embedding.api = "openai".to_string();
        cfg.providers.embedding.base_url = Some(embedding_base_url.clone());
        cfg.providers.embedding.dimensions = 64;
        cfg.providers.embedding.task_query = Some("retrieval.query".to_string());
        cfg.providers.embedding.task_passage = Some("retrieval.passage".to_string());
        cfg.providers.embedding.normalized = Some(true);
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let store = json!({
        "actor": actor("u1", "main", "sess-embed-knob-safe-1", "session-key-embed-knob-safe"),
        "mode": "tool-store",
        "memory": { "text": "Safe knob omission store probe." }
    });
    let (status, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-embed-knob-safe-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {store_body}");

    let recall = json!({
        "actor": actor("u1", "main", "sess-embed-knob-safe-2", "session-key-embed-knob-safe"),
        "query": "safe knob omission query",
        "limit": 1
    });
    let (status, recall_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "recall failed: {recall_body}");

    let captured = requests
        .lock()
        .expect("embedding requests lock should be readable")
        .clone();
    assert!(
        !captured.is_empty(),
        "embedding provider should be called in safe omission test"
    );
    assert!(
        captured.iter().all(|payload| {
            payload.get("task").is_none() && payload.get("normalized").is_none()
        }),
        "non-compatible provider assumptions should omit task/normalized fields"
    );
}

#[tokio::test]
async fn store_supports_only_two_modes_and_preserves_tool_values() {
    let app = setup_app();

    let tool_store = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "User prefers Neovim",
            "category": "preference",
            "importance": 0.82
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(tool_store),
        Some("idem-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["results"][0]["category"], "preference");
    assert_eq!(body["results"][0]["importance"], 0.82);

    let auto_capture = json!({
        "actor": actor("u1", "main", "sess-2", "session-key-1"),
        "mode": "auto-capture",
        "items": [
            { "role": "user", "text": "I use tmux" }
        ]
    });

    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(auto_capture),
        Some("idem-store-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let invalid_mode = json!({
        "actor": actor("u1", "main", "sess-3", "session-key-1"),
        "mode": "manual",
        "memory": { "text": "x" }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(invalid_mode),
        Some("idem-store-3"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let auto_with_forbidden_fields = json!({
        "actor": actor("u1", "main", "sess-4", "session-key-1"),
        "mode": "auto-capture",
        "category": "preference",
        "items": [
            { "role": "user", "text": "hello" }
        ]
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(auto_with_forbidden_fields),
        Some("idem-store-4"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn write_payloads_forbid_scope_fields() {
    let app = setup_app();

    let tool_store_with_scope = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "User prefers fish",
            "scope": "agent:evil"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(tool_store_with_scope),
        Some("idem-store-scope"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let valid_store = json!({
        "actor": actor("u1", "main", "sess-2", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "User prefers zsh",
            "category": "preference"
        }
    });

    let (_, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(valid_store),
        Some("idem-store-ok"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    let memory_id = store_body["results"][0]["id"]
        .as_str()
        .expect("memory id should be present");

    let update_with_scope = json!({
        "actor": actor("u1", "main", "sess-3", "session-key-1"),
        "memoryId": memory_id,
        "patch": {
            "scope": "agent:forbidden"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/update",
        Some(update_with_scope),
        Some("idem-update-scope"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_endpoint_exists_and_uses_backend_scope_derivation() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "User prefers tmux",
            "category": "preference",
            "importance": 0.4
        }
    });
    let (_, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-update-base-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    let memory_id = store_body["results"][0]["id"]
        .as_str()
        .expect("memory id should exist");

    let update = json!({
        "actor": actor("u1", "main", "sess-2", "session-key-1"),
        "memoryId": memory_id,
        "patch": {
            "text": "User prefers tmux with vim bindings",
            "importance": 0.9
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/update",
        Some(update),
        Some("idem-update-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"]["action"], "UPDATE");
    assert_eq!(body["result"]["scope"], "agent:main");
    assert_eq!(body["result"]["importance"], 0.9);
}

#[tokio::test]
async fn stats_route_is_post_only_and_session_id_is_ephemeral() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-original", "session-key-stable"),
        "mode": "tool-store",
        "memory": {
            "text": "Persist beyond runtime session"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-stats-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let stats_payload = json!({
        "actor": actor("u1", "main", "sess-new-runtime", "session-key-stable")
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/stats",
        Some(stats_payload),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["memoryCount"], 1);

    let (status, _) = request_json(
        &app,
        Method::GET,
        "/v1/memories/stats",
        None,
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn stats_actor_envelope_must_match_authenticated_principal() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-stats-store", "session-key-stats"),
        "mode": "tool-store",
        "memory": {
            "text": "stats envelope verification"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-stats-envelope-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let mismatched_stats = json!({
        "actor": actor("u1", "main", "sess-stats-mismatch", "session-key-stats")
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/stats",
        Some(mismatched_stats),
        None,
        Some(("u2", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "FORBIDDEN");
}

#[tokio::test]
async fn list_default_order_and_final_page_next_offset_null() {
    let app = setup_app();

    for (idx, text) in ["first", "second", "third"].into_iter().enumerate() {
        let session_id = format!("sess-{idx}");
        let idem_key = format!("idem-list-{idx}");
        let store = json!({
            "actor": actor("u1", "main", &session_id, "session-key-1"),
            "mode": "tool-store",
            "memory": {
                "text": text,
                "category": "fact"
            }
        });
        let (status, _) = request_json(
            &app,
            Method::POST,
            "/v1/memories/store",
            Some(store),
            Some(&idem_key),
            Some(("u1", "main")),
            &[],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        tokio::time::sleep(Duration::from_millis(2)).await;
    }

    let first_page = json!({
        "actor": actor("u1", "main", "sess-list-1", "session-key-1"),
        "limit": 2,
        "offset": 0,
        "category": "fact"
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/list",
        Some(first_page),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rows"][0]["text"], "third");
    assert_eq!(body["nextOffset"], 2);

    let last_page = json!({
        "actor": actor("u1", "main", "sess-list-2", "session-key-1"),
        "limit": 2,
        "offset": 2,
        "category": "fact"
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/list",
        Some(last_page),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["nextOffset"].is_null());
}

#[tokio::test]
async fn frozen_category_enum_is_enforced() {
    let app = setup_app();

    let store_invalid_category = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "x",
            "category": "not-a-category"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store_invalid_category),
        Some("idem-bad-category"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let list_invalid_category = json!({
        "actor": actor("u1", "main", "sess-2", "session-key-1"),
        "limit": 10,
        "offset": 0,
        "category": "not-a-category"
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/list",
        Some(list_invalid_category),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reflection_job_status_is_scoped_to_user_and_agent() {
    let app = setup_app();

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "trigger": "reset",
        "messages": [
            { "role": "user", "text": "summarize session" }
        ]
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/reflection/jobs",
        Some(enqueue),
        Some("idem-job-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let status_path = format!("/v1/reflection/jobs/{job_id}");

    let (status, _) = request_json(
        &app,
        Method::GET,
        &status_path,
        None,
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = request_json(
        &app,
        Method::GET,
        &status_path,
        None,
        None,
        Some(("u2", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn distill_job_enqueue_and_status_follow_frozen_contract() {
    let tmp = std::env::temp_dir().join(format!(
        "memory-lancedb-pro-backend-distill-session-source-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("temp test path should be created");
    let app = setup_app_at(&tmp);
    let (status, body) = append_session_transcript(
        &app,
        actor("u1", "main", "sess-1", "session-key-1"),
        json!([
            { "role": "user", "text": "/note skip this command" },
            { "role": "assistant", "text": "Conversation info (untrusted metadata):\nRestart mosdns after disabling systemd-resolved." },
            { "role": "assistant", "text": "<relevant-memories>ignore this injected block</relevant-memories>\nCause: systemd-resolved occupied port 53. Fix: disable it and restart mosdns." },
            { "role": "assistant", "text": "Best practice: be careful." }
        ]),
        "idem-session-transcript-append-1",
        ("u1", "main"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["appended"], 4);

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "session-lessons",
        "source": {
            "kind": "session-transcript",
            "sessionKey": "session-key-1",
            "sessionId": "sess-1"
        },
        "options": {
            "maxMessages": 400,
            "chunkChars": 12000,
            "chunkOverlapMessages": 10,
            "maxArtifacts": 20,
            "persistMode": "artifacts-only"
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["status"], "queued");
    let job_id = body["jobId"].as_str().expect("jobId should exist");

    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jobId"], job_id);
    assert_eq!(body["status"], "completed");
    assert_eq!(body["mode"], "session-lessons");
    assert_eq!(body["sourceKind"], "session-transcript");
    assert!(body["createdAt"].is_number());
    assert!(body["updatedAt"].is_number());
    assert_eq!(body["result"]["persistedMemoryCount"], 0);
    assert_eq!(body["result"]["artifactCount"], 2);
    let conn = rusqlite::Connection::open(tmp.join("sqlite/jobs.db")).expect("sqlite should open");
    let mut stmt = conn
        .prepare("SELECT text, evidence_json FROM distill_artifacts WHERE job_id = ?1 ORDER BY created_at ASC")
        .expect("distill artifact query should prepare");
    let rows = stmt
        .query_map(rusqlite::params![job_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("artifact rows should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("artifact rows should decode");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|(text, _)| text.contains("restart mosdns")));
    assert!(rows.iter().all(|(text, _)| !text.to_lowercase().contains("best practice")));
    assert!(rows
        .iter()
        .all(|(_, evidence)| evidence.contains("\"messageIds\":[2]") || evidence.contains("\"messageIds\":[3]")));
}

#[tokio::test]
async fn distill_job_status_is_scoped_to_user_and_agent() {
    let app = setup_app();

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "session-lessons",
        "source": {
            "kind": "inline-messages",
            "messages": [
                { "role": "user", "text": "extract lessons from this session" }
            ]
        },
        "options": {
            "persistMode": "artifacts-only"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::ACCEPTED);
    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let status_path = format!("/v1/distill/jobs/{job_id}");

    let (status, _) = request_json(
        &app,
        Method::GET,
        &status_path,
        None,
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = request_json(
        &app,
        Method::GET,
        &status_path,
        None,
        None,
        Some(("u2", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn distill_inline_messages_job_completes_and_persists_artifacts_and_memories() {
    let tmp = std::env::temp_dir().join(format!(
        "memory-lancedb-pro-backend-distill-complete-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("temp test path should be created");
    let app = setup_app_at(&tmp);

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "session-lessons",
        "source": {
            "kind": "inline-messages",
            "messages": [
                { "role": "assistant", "text": "Fix: restart mosdns after disabling systemd-resolved to recover DNS on openclaw." },
                { "role": "assistant", "text": "User prefers Neovim for quick config edits." }
            ]
        },
        "options": {
            "persistMode": "persist-memory-rows",
            "maxArtifacts": 5
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-complete-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");
    assert_eq!(body["result"]["artifactCount"], 2);
    assert_eq!(body["result"]["persistedMemoryCount"], 2);

    let recall = json!({
        "actor": actor("u1", "main", "sess-2", "session-key-1"),
        "query": "mosdns restart",
        "limit": 5
    });
    let (status, recall_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(recall_body["rows"]
        .as_array()
        .expect("rows should be an array")
        .iter()
        .any(|row| row["text"]
            .as_str()
            .unwrap_or_default()
            .contains("restart mosdns")));

    let conn = rusqlite::Connection::open(tmp.join("sqlite/jobs.db")).expect("sqlite should open");
    let artifact_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM distill_artifacts WHERE job_id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .expect("artifact rows should be queryable");
    assert_eq!(artifact_count, 2);
}

#[tokio::test]
async fn distill_session_transcript_job_fails_when_requested_source_has_no_persisted_messages() {
    let app = setup_app();

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "session-lessons",
        "source": {
            "kind": "session-transcript",
            "sessionKey": "session-key-1",
            "sessionId": "sess-1"
        },
        "options": {
            "persistMode": "artifacts-only"
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-source-missing-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error"]["code"], "DISTILL_SOURCE_UNAVAILABLE");
}

#[tokio::test]
async fn distill_inline_messages_filters_slash_noise_to_zero_artifacts() {
    let app = setup_app();

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "session-lessons",
        "source": {
            "kind": "inline-messages",
            "messages": [
                { "role": "user", "text": "/note remember this" },
                { "role": "assistant", "text": "✅ New session started" },
                { "role": "assistant", "text": "NO_REPLY" }
            ]
        },
        "options": {
            "persistMode": "artifacts-only"
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-noise-only-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");
    assert_eq!(body["result"]["artifactCount"], 0);
    assert_eq!(body["result"]["persistedMemoryCount"], 0);
    assert!(body["result"]["warnings"]
        .as_array()
        .expect("warnings should be an array")
        .iter()
        .any(|warning| warning
            .as_str()
            .unwrap_or_default()
            .contains("filtered as noise")));
}

#[tokio::test]
async fn distill_job_rejects_empty_inline_messages() {
    let app = setup_app();

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "session-lessons",
        "source": {
            "kind": "inline-messages",
            "messages": []
        },
        "options": {
            "persistMode": "artifacts-only"
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-invalid-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .expect("error message should exist")
        .contains("source.messages must be non-empty"));
}

#[tokio::test]
async fn distill_job_rejects_persist_memory_rows_for_governance_candidates() {
    let app = setup_app();

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "governance-candidates",
        "source": {
            "kind": "session-transcript",
            "sessionKey": "session-key-1"
        },
        "options": {
            "persistMode": "persist-memory-rows"
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-invalid-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"]["message"]
        .as_str()
        .expect("error message should exist")
        .contains("persistMemoryRows is only allowed for mode=session-lessons"));
}

#[tokio::test]
async fn generic_recall_dto_does_not_expose_scoring_breakdown_internals() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "User prefers Neovim"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-recall-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let recall = json!({
        "actor": actor("u1", "main", "sess-2", "session-key-1"),
        "query": "Neovim",
        "limit": 3
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let row = &body["rows"][0];
    assert!(row.get("score").is_some());
    assert!(row.get("vectorScore").is_none());
    assert!(row.get("bm25Score").is_none());
    assert!(row.get("rerankScore").is_none());
}

#[tokio::test]
async fn retrieval_diagnostics_enabled_does_not_leak_internal_fields_to_v1_rows() {
    let app = setup_app_with(|cfg| {
        cfg.retrieval.diagnostics = true;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let store = json!({
        "actor": actor("u1", "main", "sess-diagnostics-1", "session-key-diagnostics"),
        "mode": "tool-store",
        "memory": {
            "text": "Release runbook for deployment rollback and postmortem tracking."
        }
    });
    let (status, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-diagnostics-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {store_body}");

    let recall = json!({
        "actor": actor("u1", "main", "sess-diagnostics-2", "session-key-diagnostics"),
        "query": "deployment rollback runbook",
        "limit": 3
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK, "recall failed: {body}");
    let row = &body["rows"][0];
    assert!(row.get("diagnostics").is_none());
    assert!(row.get("trace").is_none());
    assert!(row.get("stageCounts").is_none());
}

#[tokio::test]
async fn debug_generic_recall_route_returns_structured_trace_without_mutating_v1_rows() {
    let app = setup_app_with(|cfg| {
        cfg.retrieval.diagnostics = true;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let store = json!({
        "actor": actor("u1", "main", "sess-debug-trace-1", "session-key-debug-trace"),
        "mode": "tool-store",
        "memory": {
            "text": "Document rollback drill steps and verify post-check evidence."
        }
    });
    let (status, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-debug-trace-store-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "store failed: {store_body}");

    let recall = json!({
        "actor": actor("u1", "main", "sess-debug-trace-2", "session-key-debug-trace"),
        "query": "rollback drill evidence",
        "limit": 3
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/debug/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK, "debug recall failed: {body}");
    let rows = body["rows"].as_array().expect("rows should be present");
    assert!(!rows.is_empty(), "debug recall should return rows");
    assert!(
        rows[0].get("trace").is_none(),
        "trace must not leak into row payloads"
    );
    assert!(rows[0].get("diagnostics").is_none());

    assert_eq!(body["trace"]["kind"], "generic");
    let stages = body["trace"]["stages"]
        .as_array()
        .expect("trace stages should be an array");
    assert!(
        stages.iter().any(|stage| stage["name"] == "seed.merge"),
        "trace should record merged seed stage"
    );
    assert!(
        stages.iter().any(|stage| stage["name"] == "rank.finalize"),
        "trace should record finalization stage"
    );
    assert!(
        stages.iter().any(|stage| stage["name"] == "access-update"),
        "trace should record access metadata update stage"
    );
    let final_row_ids = body["trace"]["finalRowIds"]
        .as_array()
        .expect("trace should include final row ids");
    assert!(
        !final_row_ids.is_empty(),
        "trace should include final row ids"
    );
}

#[tokio::test]
async fn debug_recall_route_enforces_authenticated_actor_principal_boundary() {
    let app = setup_app();
    let recall = json!({
        "actor": actor("u2", "other-agent", "sess-debug-boundary", "session-key-boundary"),
        "query": "forbidden trace inspection",
        "limit": 3
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/debug/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        body["error"]["message"],
        "actor principal does not match authenticated request context"
    );
}

#[tokio::test]
async fn debug_recall_trace_records_rerank_fallback_reason_without_exposing_runtime_dto_fields() {
    let (rerank_base_url, _requests) = spawn_auth_rerank_mock_server(None, true).await;
    let app = setup_app_with(|cfg| {
        cfg.retrieval.diagnostics = true;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
        cfg.providers.rerank.enabled = true;
        cfg.providers.rerank.mode = "cross-encoder".to_string();
        cfg.providers.rerank.provider = "jina".to_string();
        cfg.providers.rerank.base_url = Some(format!("{rerank_base_url}/rerank"));
        cfg.providers.rerank.api_key = Some("first-key,second-key".to_string());
        cfg.providers.rerank.blend = 1.0;
    });

    for (idx, text) in [
        "Primary rollback sequence for service unit changes.",
        "Post-check evidence collection after rollback sequence.",
    ]
    .iter()
    .enumerate()
    {
        let session_id = format!("sess-rerank-trace-store-{idx}");
        let idem_key = format!("idem-rerank-trace-store-{idx}");
        let store = json!({
            "actor": actor("u1", "main", &session_id, "session-key-rerank-trace"),
            "mode": "tool-store",
            "memory": {
                "text": text
            }
        });
        let (status, body) = request_json(
            &app,
            Method::POST,
            "/v1/memories/store",
            Some(store),
            Some(&idem_key),
            Some(("u1", "main")),
            &[],
        )
        .await;
        assert_eq!(status, StatusCode::OK, "store failed: {body}");
    }

    let recall = json!({
        "actor": actor("u1", "main", "sess-rerank-trace-1", "session-key-rerank-trace"),
        "query": "rollback sequence",
        "limit": 3
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/debug/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK, "debug recall failed: {body}");
    let rerank_stage = body["trace"]["stages"]
        .as_array()
        .expect("trace stages should be present")
        .iter()
        .find(|stage| stage["name"] == "rerank")
        .cloned()
        .expect("rerank stage should be recorded");
    assert_eq!(rerank_stage["status"], "fallback");
    assert_eq!(rerank_stage["fallbackTo"], "lightweight");
    assert_eq!(rerank_stage["metrics"]["appliedMode"], "lightweight");
    let reason = rerank_stage["reason"]
        .as_str()
        .expect("rerank fallback reason should be recorded");
    assert!(
        reason.len() <= 240,
        "rerank fallback reason should be bounded, got {} chars",
        reason.len()
    );
}

#[tokio::test]
async fn debug_reflection_recall_route_reports_mode_and_trace_without_leaking_extra_row_fields() {
    let app = setup_app_with(|cfg| {
        cfg.retrieval.diagnostics = true;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    let store = json!({
        "actor": actor("u1", "main", "sess-debug-reflection-store", "session-key-debug-reflection"),
        "mode": "tool-store",
        "memory": {
            "text": "Always verify DNS and mount health after restart.",
            "category": "reflection"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-debug-reflection-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "reflection store failed: {body}");

    let recall = json!({
        "actor": actor("u1", "main", "sess-debug-reflection-query", "session-key-debug-reflection"),
        "query": "restart checks",
        "mode": "invariant-only",
        "limit": 3
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/debug/recall/reflection",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "debug reflection recall failed: {body}"
    );
    assert_eq!(body["trace"]["kind"], "reflection");
    assert_eq!(body["trace"]["mode"], "invariant-only");
    let rows = body["rows"].as_array().expect("rows should be an array");
    assert!(
        !rows.is_empty(),
        "reflection debug recall should return rows"
    );
    assert!(rows[0].get("trace").is_none());
    assert!(rows[0].get("diagnostics").is_none());
}

#[tokio::test]
async fn generic_recall_prefers_real_signal_over_placeholder_ordering() {
    let app = setup_app();

    let relevant_store = json!({
        "actor": actor("u1", "main", "sess-rag-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Use tmux to keep terminal sessions alive while developing."
        }
    });
    let (status, relevant_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(relevant_store),
        Some("idem-rag-real-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let relevant_id = relevant_body["results"][0]["id"]
        .as_str()
        .expect("relevant memory id should exist")
        .to_string();

    std::thread::sleep(Duration::from_millis(10));

    let irrelevant_store = json!({
        "actor": actor("u1", "main", "sess-rag-2", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Quarterly finance budget review and tax planning notes."
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(irrelevant_store),
        Some("idem-rag-real-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let recall = json!({
        "actor": actor("u1", "main", "sess-rag-3", "session-key-1"),
        "query": "tmux sessions",
        "limit": 2
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"].as_array().expect("rows should be an array");
    assert!(!rows.is_empty(), "recall rows should not be empty");

    let top = &rows[0];
    assert_eq!(
        top["id"].as_str().expect("top row id should be string"),
        relevant_id
    );
    assert!(
        top["text"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains("tmux"),
        "top row should be the tmux memory"
    );

    if rows.len() > 1 {
        let top_score = top["score"].as_f64().unwrap_or(0.0);
        let second_score = rows[1]["score"].as_f64().unwrap_or(0.0);
        assert!(
            top_score > second_score,
            "top score should outrank the irrelevant row"
        );
    }
}

#[tokio::test]
async fn actor_principal_must_match_authenticated_request_context() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "auth-bound memory"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-auth-mismatch"),
        Some(("u2", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_authenticated_identity_headers_are_rejected() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-auth-missing", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "missing auth headers"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-auth-missing"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_REQUEST");
    let message = body["error"]["message"]
        .as_str()
        .expect("error message should be string");
    assert!(
        message.contains(AUTH_USER_ID_HEADER),
        "missing auth user header should be named in error message"
    );
}

#[tokio::test]
async fn reflection_recall_mode_honors_invariant_only_semantics() {
    let app = setup_app();

    let store_reflection = json!({
        "actor": actor("u1", "main", "sess-ref-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Verify health checks before infra edits",
            "category": "reflection"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store_reflection),
        Some("idem-ref-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let invariant_only = json!({
        "actor": actor("u1", "main", "sess-ref-2", "session-key-1"),
        "query": "health",
        "mode": "invariant-only",
        "limit": 5
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/reflection",
        Some(invariant_only),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rows"][0]["kind"], "invariant");
    assert!(body["rows"][0]["strictKey"].is_string());
}

#[tokio::test]
async fn idempotency_reuse_returns_conflict() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-idem-1", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "idempotent write"
        }
    });
    let (status, _) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store.clone()),
        Some("idem-repeat-key"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-repeat-key"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "IDEMPOTENCY_CONFLICT");
}

#[tokio::test]
async fn idempotency_key_can_retry_after_failed_operation() {
    let app = setup_app();

    let missing_update = json!({
        "actor": actor("u1", "main", "sess-idem-failed-1", "session-key-1"),
        "memoryId": "mem_missing",
        "patch": {
            "text": "retry after failed operation"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/update",
        Some(missing_update.clone()),
        Some("idem-failed-retry"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/update",
        Some(missing_update),
        Some("idem-failed-retry"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "NOT_FOUND");

    let mismatched_payload = json!({
        "actor": actor("u1", "main", "sess-idem-failed-2", "session-key-1"),
        "memoryId": "mem_missing_other",
        "patch": {
            "text": "different payload should conflict"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/update",
        Some(mismatched_payload),
        Some("idem-failed-retry"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "IDEMPOTENCY_CONFLICT");
}

#[tokio::test]
async fn lancedb_memory_persists_across_app_restart() {
    let tmp = std::env::temp_dir().join(format!(
        "memory-lancedb-pro-backend-persist-{}",
        Uuid::new_v4()
    ));

    let app_a = setup_app_at(&tmp);
    let store = json!({
        "actor": actor("u1", "main", "sess-persist-1", "session-key-persist"),
        "mode": "tool-store",
        "memory": {
            "text": "persisted in lancedb"
        }
    });
    let (status, _) = request_json(
        &app_a,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-persist-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let app_b = setup_app_at(&tmp);
    let stats_payload = json!({
        "actor": actor("u1", "main", "sess-persist-2", "session-key-persist")
    });
    let (status, body) = request_json(
        &app_b,
        Method::POST,
        "/v1/memories/stats",
        Some(stats_payload),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["memoryCount"], 1);
}

#[tokio::test]
async fn admin_token_cannot_bypass_data_plane_and_admin_routes_are_not_exposed() {
    let app = setup_app();

    let store = json!({
        "actor": actor("u1", "main", "sess-admin-1", "session-key-admin"),
        "mode": "tool-store",
        "memory": {
            "text": "admin token bypass isolation check"
        }
    });
    let (status, body) = request_json_with_token(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store),
        Some("idem-admin-bypass"),
        Some(("u1", "main")),
        "admin-token",
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"]["code"], "UNAUTHORIZED");
    let message = body["error"]["message"]
        .as_str()
        .expect("error message should be string");
    assert!(!message.contains("admin-token"));

    let (status, _) = request_json_with_token(
        &app,
        Method::GET,
        "/v1/admin/health",
        None,
        None,
        Some(("u1", "main")),
        "admin-token",
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
