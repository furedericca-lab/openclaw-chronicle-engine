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
