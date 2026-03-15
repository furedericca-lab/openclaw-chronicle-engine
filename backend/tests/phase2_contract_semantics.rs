use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use memory_lancedb_pro_backend::{
    build_app,
    config::{AppConfig, AuthConfig, LoggingConfig, ServerConfig, StorageConfig, TokenConfig},
};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
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
