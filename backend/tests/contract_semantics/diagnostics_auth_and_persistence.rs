
use super::*;

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
async fn debug_behavioral_recall_routes_report_behavioral_trace_without_leaking_extra_row_fields() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-debug-behavioral-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_with_at(&tmp, |cfg| {
        cfg.retrieval.diagnostics = true;
        cfg.retrieval.min_score = 0.0;
        cfg.retrieval.hard_min_score = 0.0;
    });

    seed_behavioral_memory(
        &app,
        &tmp,
        (
            "u1",
            "main",
            "sess-debug-behavioral-store",
            "session-key-debug-behavioral",
        ),
        "Always verify DNS and mount health after restart.",
        "idem-debug-behavioral-store",
    )
    .await;

    let recall = json!({
        "actor": actor("u1", "main", "sess-debug-behavioral-query", "session-key-debug-behavioral"),
        "query": "restart checks",
        "mode": "invariant-only",
        "limit": 3
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/debug/recall/behavioral",
        Some(recall.clone()),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;

    assert_eq!(
        status,
        StatusCode::OK,
        "debug behavioral recall failed: {body}"
    );
    assert_eq!(body["trace"]["kind"], "behavioral");
    assert_eq!(body["trace"]["mode"], "invariant-only");
    let rows = body["rows"].as_array().expect("rows should be an array");
    assert!(
        !rows.is_empty(),
        "behavioral debug recall should return rows"
    );
    assert!(rows[0].get("trace").is_none());
    assert!(rows[0].get("diagnostics").is_none());

    let (alias_status, _alias_body) = request_json(
        &app,
        Method::POST,
        "/v1/debug/recall/reflection",
        Some(recall),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(alias_status, StatusCode::NOT_FOUND);
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
async fn manual_behavioral_write_routes_are_rejected() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-manual-behavioral-write-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_at(&tmp);

    let store_behavioral = json!({
        "actor": actor("u1", "main", "sess-ref-store-reject", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Manual behavioral write should be rejected",
            "category": "behavioral"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store_behavioral),
        Some("idem-behavioral-store-reject"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_REQUEST");
    assert_eq!(
        body["error"]["message"],
        "behavioral-guidance rows are backend-managed and cannot be created or updated manually"
    );

    let store_fact = json!({
        "actor": actor("u1", "main", "sess-fact-store", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "Store a fact row first",
            "category": "fact"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store_fact),
        Some("idem-fact-store"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let fact_row_id = body["results"][0]["id"]
        .as_str()
        .expect("stored fact row id should exist");

    let update_to_behavioral = json!({
        "actor": actor("u1", "main", "sess-fact-update", "session-key-1"),
        "memoryId": fact_row_id,
        "patch": {
            "category": "behavioral"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/update",
        Some(update_to_behavioral),
        Some("idem-update-to-behavioral"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_REQUEST");
    assert_eq!(
        body["error"]["message"],
        "behavioral-guidance rows are backend-managed and cannot be created or updated manually"
    );

    let behavioral_row_id = seed_behavioral_memory(
        &app,
        &tmp,
        ("u1", "main", "sess-behavioral-row", "session-key-1"),
        "Seeded behavioral row for recall-only behavior",
        "idem-seeded-behavioral-row",
    )
    .await;
    let update_existing_behavioral = json!({
        "actor": actor("u1", "main", "sess-behavioral-update", "session-key-1"),
        "memoryId": behavioral_row_id,
        "patch": {
            "text": "Attempted manual edit to behavioral row"
        }
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/update",
        Some(update_existing_behavioral),
        Some("idem-update-behavioral-row"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "INVALID_REQUEST");
    assert_eq!(
        body["error"]["message"],
        "behavioral-guidance rows are backend-managed and cannot be created or updated manually"
    );
}

#[tokio::test]
async fn behavioral_recall_mode_honors_invariant_only_semantics_without_legacy_route_alias() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-behavioral-mode-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_at(&tmp);
    seed_behavioral_memory(
        &app,
        &tmp,
        ("u1", "main", "sess-behavioral-1", "session-key-1"),
        "Verify health checks before infra edits",
        "idem-behavioral-store",
    )
    .await;

    let invariant_only = json!({
        "actor": actor("u1", "main", "sess-behavioral-2", "session-key-1"),
        "query": "health",
        "mode": "invariant-only",
        "limit": 5
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/behavioral",
        Some(invariant_only.clone()),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["rows"][0]["kind"], "invariant");
    assert!(body["rows"][0]["strictKey"].is_string());
    assert!(
        body["rows"][0]["strictKey"]
            .as_str()
            .unwrap_or_default()
            .starts_with("behavioral:"),
        "behavioral strict keys should be normalized on the API surface"
    );

    let (alias_status, _alias_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/reflection",
        Some(invariant_only),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(alias_status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn public_reflection_request_aliases_are_removed() {
    let app = setup_app();

    let generic_with_removed_field = json!({
        "actor": actor("u1", "main", "sess-generic-removed-alias", "session-key-1"),
        "query": "health",
        "limit": 5,
        "excludeReflection": true
    });
    let (generic_status, generic_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(generic_with_removed_field),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(generic_status, StatusCode::BAD_REQUEST);
    assert_eq!(generic_body["error"]["code"], "INVALID_REQUEST");

    let store_with_removed_category = json!({
        "actor": actor("u1", "main", "sess-store-removed-category", "session-key-1"),
        "mode": "tool-store",
        "memory": {
            "text": "removed category alias",
            "category": "reflection"
        }
    });
    let (store_status, store_body) = request_json(
        &app,
        Method::POST,
        "/v1/memories/store",
        Some(store_with_removed_category),
        Some("idem-store-removed-category"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(store_status, StatusCode::BAD_REQUEST);
    assert_eq!(store_body["error"]["code"], "INVALID_REQUEST");

    let generic_with_removed_category = json!({
        "actor": actor("u1", "main", "sess-recall-removed-category", "session-key-1"),
        "query": "health",
        "limit": 5,
        "categories": ["reflection"]
    });
    let (recall_status, recall_body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/generic",
        Some(generic_with_removed_category),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(recall_status, StatusCode::BAD_REQUEST);
    assert_eq!(recall_body["error"]["code"], "INVALID_REQUEST");
}

#[tokio::test]
async fn generic_recall_applies_backend_owned_filter_fields() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-generic-filter-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_at(&tmp);

    for (idx, text) in [
        "Verify DNS and mount health after restart.",
        "Verify DNS and mount health after restart.",
    ]
    .iter()
    .enumerate()
    {
        let session_id = format!("sess-generic-filter-{idx}");
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
            Some(&format!("idem-generic-filter-{idx}")),
            Some(("u1", "main")),
            &[],
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    seed_behavioral_memory(
        &app,
        &tmp,
        (
            "u1",
            "main",
            "sess-generic-filter-behavioral",
            "session-key-1",
        ),
        "Behavioral memory should not pass generic backend filter.",
        "idem-generic-filter-behavioral",
    )
    .await;

    let recall = json!({
        "actor": actor("u1", "main", "sess-generic-filter-query", "session-key-1"),
        "query": "verify DNS and mount health",
        "limit": 5,
        "categories": ["fact", "behavioral"],
        "excludeBehavioral": true,
        "maxEntriesPerKey": 1
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
    assert_eq!(
        rows.len(),
        1,
        "backend should enforce duplicate/key filtering"
    );
    assert_eq!(rows[0]["category"], "fact");
    assert!(
        rows[0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("Verify DNS and mount health after restart."),
        "backend should keep the fact row after applying filters"
    );
}

#[tokio::test]
async fn behavioral_recall_applies_include_kinds_filter() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-test-behavioral-filter-{}",
        Uuid::new_v4()
    ));
    let app = setup_app_at(&tmp);
    seed_behavioral_memory(
        &app,
        &tmp,
        ("u1", "main", "sess-behavioral-filter-1", "session-key-1"),
        "Always verify health checks before infra edits",
        "idem-behavioral-filter-store",
    )
    .await;

    let derived_only = json!({
        "actor": actor("u1", "main", "sess-behavioral-filter-2", "session-key-1"),
        "query": "health",
        "mode": "invariant+derived",
        "limit": 5,
        "includeKinds": ["derived"],
        "minScore": 0.0
    });
    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/recall/behavioral",
        Some(derived_only),
        None,
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["rows"].as_array().expect("rows should be an array");
    assert_eq!(
        rows.len(),
        0,
        "backend should enforce behavioral kind filtering"
    );
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
        "chronicle-engine-rs-persist-{}",
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
        RequestAuth {
            auth_context: Some(("u1", "main")),
            bearer_token: "admin-token",
        },
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
        RequestAuth {
            auth_context: Some(("u1", "main")),
            bearer_token: "admin-token",
        },
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
