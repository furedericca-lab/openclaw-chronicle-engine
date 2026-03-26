
use super::*;

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
async fn legacy_table_without_vector_column_is_rejected_as_unsupported() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-legacy-schema-{}",
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
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "legacy stats should be rejected: {stats_body}"
    );
    assert_eq!(stats_body["error"]["code"], "BACKEND_UNAVAILABLE");
    assert!(
        stats_body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unsupported legacy LanceDB memory schema"),
        "legacy schema error should explain manual migration/reset requirement: {stats_body}"
    );
}

#[tokio::test]
async fn lancedb_search_indices_are_explicitly_ensured() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-index-lifecycle-{}",
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
        "chronicle-engine-rs-access-reinforcement-{}",
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
        "chronicle-engine-rs-access-bound-{}",
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
