
use super::*;

#[tokio::test]
async fn distill_job_enqueue_and_status_follow_frozen_contract() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-distill-session-source-{}",
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
    assert_eq!(body["result"]["artifactCount"], 1);
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
    assert_eq!(rows.len(), 1);
    assert!(rows
        .iter()
        .any(|(text, _)| text.contains("Cause:") && text.contains("Fix:")));
    assert!(rows
        .iter()
        .all(|(text, _)| !text.to_lowercase().contains("best practice")));
    assert!(rows.iter().all(|(_, evidence)| {
        evidence.contains("\"messageIds\":[2]") && evidence.contains("\"messageIds\":[3]")
    }));
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
        "chronicle-engine-rs-distill-complete-{}",
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
async fn distill_inline_messages_aggregates_multi_message_evidence_and_structured_summary() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-distill-aggregate-{}",
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
                { "role": "user", "text": "DNS broke on openclaw after a risky resolver change." },
                { "role": "assistant", "text": "Cause: systemd-resolved occupied port 53 and blocked mosdns." },
                { "role": "assistant", "text": "Fix: disable systemd-resolved, restart mosdns, and verify DNS resolution." },
                { "role": "assistant", "text": "Prevention: keep systemd-resolved disabled in this LXC baseline." }
            ]
        },
        "options": {
            "persistMode": "artifacts-only",
            "maxArtifacts": 5
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-aggregate-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");
    assert!(body["result"]["artifactCount"].as_u64().unwrap_or(0) >= 1);

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

    assert!(rows.iter().any(|(text, evidence)| {
        text.starts_with("Lesson:")
            && text.contains("Cause:")
            && text.contains("Fix:")
            && evidence.contains("\"messageIds\":[2]")
            && evidence.contains("\"messageIds\":[3]")
    }));
}

#[tokio::test]
async fn distill_session_lessons_single_hit_stays_lesson_for_stable_and_durable_labels() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-distill-evidence-gate-fallback-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("temp test path should be created");
    let app = setup_app_at(&tmp);

    for (idempotency_key, message) in [
        (
            "idem-distill-evidence-gate-durable-1",
            "Durable practice: always verify DNS resolution after a risky resolver edit.",
        ),
        (
            "idem-distill-evidence-gate-stable-1",
            "Stable decision: default gateway traffic to the HAProxy loopback proxy.",
        ),
    ] {
        let enqueue = json!({
            "actor": actor("u1", "main", "sess-1", "session-key-1"),
            "mode": "session-lessons",
            "source": {
                "kind": "inline-messages",
                "messages": [
                    { "role": "assistant", "text": message }
                ]
            },
            "options": {
                "persistMode": "artifacts-only",
                "maxArtifacts": 2
            }
        });

        let (status, body) = request_json(
            &app,
            Method::POST,
            "/v1/distill/jobs",
            Some(enqueue),
            Some(idempotency_key),
            Some(("u1", "main")),
            &[],
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);

        let job_id = body["jobId"].as_str().expect("jobId should exist");
        let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "completed");

        let conn =
            rusqlite::Connection::open(tmp.join("sqlite/jobs.db")).expect("sqlite should open");
        let mut stmt = conn
            .prepare("SELECT kind, subtype, text FROM distill_artifacts WHERE job_id = ?1 ORDER BY created_at ASC")
            .expect("distill artifact query should prepare");
        let rows = stmt
            .query_map(rusqlite::params![job_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .expect("artifact rows should query")
            .collect::<Result<Vec<_>, _>>()
            .expect("artifact rows should decode");

        assert_eq!(rows.len(), 1);
        assert!(rows.iter().all(|(kind, subtype, text)| {
            kind == "lesson"
                && subtype.is_none()
                && text.starts_with("Lesson:")
                && !text.starts_with("Lesson: Follow-up focus:")
                && !text.starts_with("Lesson: Next-turn guidance:")
        }));
        assert!(rows
            .iter()
            .all(|(_, _, text)| !text.starts_with("Durable practice:")));
        assert!(rows
            .iter()
            .all(|(_, _, text)| !text.starts_with("Stable decision:")));
    }
}

#[tokio::test]
async fn distill_session_lessons_promote_repeated_stable_decision_signals() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-distill-evidence-gate-stable-{}",
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
                { "role": "assistant", "text": "Decision: default OpenClaw gateway traffic to the HAProxy loopback proxy at 127.0.0.1:17890." },
                { "role": "assistant", "text": "Prefer the loopback proxy as the standard path instead of dialing upstream proxies directly." }
            ]
        },
        "options": {
            "persistMode": "artifacts-only",
            "maxArtifacts": 3
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-evidence-gate-stable-2"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");

    let conn = rusqlite::Connection::open(tmp.join("sqlite/jobs.db")).expect("sqlite should open");
    let mut stmt = conn
        .prepare("SELECT kind, subtype, text, evidence_json FROM distill_artifacts WHERE job_id = ?1 ORDER BY created_at ASC")
        .expect("distill artifact query should prepare");
    let rows = stmt
        .query_map(rusqlite::params![job_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .expect("artifact rows should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("artifact rows should decode");

    assert!(rows.iter().any(|(kind, subtype, text, evidence)| {
        kind == "lesson"
            && subtype.is_none()
            && text.starts_with("Stable decision:")
            && evidence.contains("\"messageIds\":[1]")
            && evidence.contains("\"messageIds\":[2]")
    }));
}

#[tokio::test]
async fn distill_session_lessons_emit_durable_practice_and_follow_up_subtypes() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-distill-subtypes-{}",
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
                { "role": "user", "text": "DNS broke on openclaw after a risky resolver change." },
                { "role": "assistant", "text": "Cause: systemd-resolved occupied port 53 and blocked mosdns." },
                { "role": "assistant", "text": "Fix: disable systemd-resolved, restart mosdns, and verify DNS resolution." },
                { "role": "assistant", "text": "Prevention: keep systemd-resolved disabled as a durable practice on openclaw." },
                { "role": "assistant", "text": "Open loop: verify the Azure mount reconnects cleanly after DNS recovery." },
                { "role": "assistant", "text": "Next turn: ask the user for the latest rclone log if /mnt/azure still looks stale." }
            ]
        },
        "options": {
            "persistMode": "artifacts-only",
            "maxArtifacts": 6
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-subtypes-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");

    let conn = rusqlite::Connection::open(tmp.join("sqlite/jobs.db")).expect("sqlite should open");
    let mut stmt = conn
        .prepare("SELECT kind, subtype, text, evidence_json FROM distill_artifacts WHERE job_id = ?1 ORDER BY created_at ASC")
        .expect("distill artifact query should prepare");
    let rows = stmt
        .query_map(rusqlite::params![job_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .expect("artifact rows should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("artifact rows should decode");

    assert!(rows.iter().any(|(kind, subtype, text, evidence)| {
        kind == "lesson"
            && subtype.is_none()
            && text.starts_with("Durable practice:")
            && evidence.contains("\"messageIds\":[1]")
            && evidence.contains("\"messageIds\":[2]")
            && evidence.contains("\"messageIds\":[3]")
    }));
    assert!(rows.iter().any(|(kind, subtype, text, _)| {
        kind == "lesson"
            && subtype.as_deref() == Some("follow-up-focus")
            && text.starts_with("Follow-up focus:")
    }));
    assert!(rows.iter().any(|(kind, subtype, text, _)| {
        kind == "lesson"
            && subtype.as_deref() == Some("next-turn-guidance")
            && text.starts_with("Next-turn guidance:")
    }));
}

#[tokio::test]
async fn governance_candidates_emit_skill_and_promotion_labels() {
    let tmp = std::env::temp_dir().join(format!(
        "chronicle-engine-rs-distill-governance-labels-{}",
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&tmp).expect("temp test path should be created");
    let app = setup_app_at(&tmp);

    let enqueue = json!({
        "actor": actor("u1", "main", "sess-1", "session-key-1"),
        "mode": "governance-candidates",
        "source": {
            "kind": "inline-messages",
            "messages": [
                { "role": "assistant", "text": "This DNS recovery workflow is reusable enough to extract as a skill." },
                { "role": "assistant", "text": "Promote the resolver guardrail into AGENTS.md so future sessions keep systemd-resolved disabled." }
            ]
        },
        "options": {
            "persistMode": "artifacts-only",
            "maxArtifacts": 4
        }
    });

    let (status, body) = request_json(
        &app,
        Method::POST,
        "/v1/distill/jobs",
        Some(enqueue),
        Some("idem-distill-governance-1"),
        Some(("u1", "main")),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);

    let job_id = body["jobId"].as_str().expect("jobId should exist");
    let (status, body) = poll_distill_job(&app, job_id, ("u1", "main")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");

    let conn = rusqlite::Connection::open(tmp.join("sqlite/jobs.db")).expect("sqlite should open");
    let mut stmt = conn
        .prepare(
            "SELECT kind, text FROM distill_artifacts WHERE job_id = ?1 ORDER BY created_at ASC",
        )
        .expect("distill artifact query should prepare");
    let rows = stmt
        .query_map(rusqlite::params![job_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .expect("artifact rows should query")
        .collect::<Result<Vec<_>, _>>()
        .expect("artifact rows should decode");

    assert!(rows.iter().all(|(kind, _)| kind == "governance-candidate"));
    assert!(rows
        .iter()
        .any(|(_, text)| { text.contains("Governance candidate: Skill extraction candidate:") }));
    assert!(rows.iter().any(|(_, text)| {
        text.contains("Governance candidate: AGENTS/SOUL/TOOLS promotion candidate:")
    }));
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
