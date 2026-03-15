pub mod config;
mod error;
pub mod models;
mod state;

pub use error::{AppError, AppResult};
pub use state::AppState;

use crate::{
    config::AppConfig,
    models::{
        validate_delete_request, validate_enqueue_reflection_job_request, validate_list_request,
        validate_recall_generic_request, validate_recall_reflection_request,
        validate_stats_request, validate_store_request, validate_update_request, Actor,
        DeleteRequest, EnqueueReflectionJobRequest, HealthResponse, ListRequest, Principal,
        RecallGenericRequest, RecallReflectionRequest, StatsRequest, StoreRequest, UpdateRequest,
    },
};
use axum::{
    body::Body,
    extract::{rejection::JsonRejection, Extension, Path, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::future::Future;

const AUTH_USER_ID_HEADER: &str = "x-auth-user-id";
const AUTH_AGENT_ID_HEADER: &str = "x-auth-agent-id";

#[derive(Clone, Debug)]
struct RuntimeAuthContext {
    principal: Principal,
}

pub fn build_app(config: AppConfig) -> anyhow::Result<Router> {
    config.validate()?;
    let state = AppState::new(config)?;

    let data_routes = Router::new()
        .route("/v1/recall/generic", post(recall_generic))
        .route("/v1/recall/reflection", post(recall_reflection))
        .route("/v1/memories/store", post(store_memories))
        .route("/v1/memories/update", post(update_memory))
        .route("/v1/memories/delete", post(delete_memories))
        .route("/v1/memories/list", post(list_memories))
        .route("/v1/memories/stats", post(memory_stats))
        .route("/v1/reflection/jobs", post(enqueue_reflection_job))
        .route(
            "/v1/reflection/jobs/:job_id",
            get(get_reflection_job_status),
        )
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            runtime_auth_middleware,
        ));

    Ok(Router::new()
        .route("/v1/health", get(health))
        .merge(data_routes))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "memory-backend",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn recall_generic(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    payload: Result<Json<RecallGenericRequest>, JsonRejection>,
) -> AppResult<Json<crate::models::RecallGenericResponse>> {
    let req = decode_json(payload)?;
    validate_recall_generic_request(&req)?;
    ensure_actor_matches_context(&req.actor, &auth)?;
    let rows = state.memory_repo.recall_generic(req).await?;
    Ok(Json(rows))
}

async fn recall_reflection(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    payload: Result<Json<RecallReflectionRequest>, JsonRejection>,
) -> AppResult<Json<crate::models::RecallReflectionResponse>> {
    let req = decode_json(payload)?;
    validate_recall_reflection_request(&req)?;
    ensure_actor_matches_context(&req.actor, &auth)?;
    let rows = state.memory_repo.recall_reflection(req).await?;
    Ok(Json(rows))
}

async fn store_memories(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    headers: HeaderMap,
    payload: Result<Json<StoreRequest>, JsonRejection>,
) -> AppResult<Json<crate::models::StoreResponse>> {
    let idempotency_key = require_idempotency_key(&headers)?.to_string();
    let req = decode_json(payload)?;
    validate_store_request(&req)?;
    ensure_actor_matches_context(req.actor(), &auth)?;
    let response = run_idempotent_operation(
        &state,
        &auth.principal,
        "POST /v1/memories/store",
        &idempotency_key,
        &fingerprint_request(&req)?,
        state.memory_repo.store(req),
    )
    .await?;
    Ok(Json(response))
}

async fn update_memory(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    headers: HeaderMap,
    payload: Result<Json<UpdateRequest>, JsonRejection>,
) -> AppResult<Json<crate::models::UpdateResponse>> {
    let idempotency_key = require_idempotency_key(&headers)?.to_string();
    let req = decode_json(payload)?;
    validate_update_request(&req)?;
    ensure_actor_matches_context(&req.actor, &auth)?;
    let response = run_idempotent_operation(
        &state,
        &auth.principal,
        "POST /v1/memories/update",
        &idempotency_key,
        &fingerprint_request(&req)?,
        state.memory_repo.update(req),
    )
    .await?;
    Ok(Json(response))
}

async fn delete_memories(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    headers: HeaderMap,
    payload: Result<Json<DeleteRequest>, JsonRejection>,
) -> AppResult<Json<crate::models::DeleteResponse>> {
    let idempotency_key = require_idempotency_key(&headers)?.to_string();
    let req = decode_json(payload)?;
    validate_delete_request(&req)?;
    ensure_actor_matches_context(&req.actor, &auth)?;
    let deleted = run_idempotent_operation(
        &state,
        &auth.principal,
        "POST /v1/memories/delete",
        &idempotency_key,
        &fingerprint_request(&req)?,
        state.memory_repo.delete(req),
    )
    .await?;
    Ok(Json(crate::models::DeleteResponse { deleted }))
}

async fn list_memories(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    payload: Result<Json<ListRequest>, JsonRejection>,
) -> AppResult<Json<crate::models::ListResponse>> {
    let req = decode_json(payload)?;
    validate_list_request(&req)?;
    ensure_actor_matches_context(&req.actor, &auth)?;
    let response = state.memory_repo.list(req).await?;
    Ok(Json(response))
}

async fn memory_stats(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    payload: Result<Json<StatsRequest>, JsonRejection>,
) -> AppResult<Json<crate::models::StatsResponse>> {
    let req = decode_json(payload)?;
    validate_stats_request(&req)?;
    ensure_actor_matches_context(&req.actor, &auth)?;
    let response = state.memory_repo.stats(&req.actor).await?;
    Ok(Json(response))
}

async fn enqueue_reflection_job(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    headers: HeaderMap,
    payload: Result<Json<EnqueueReflectionJobRequest>, JsonRejection>,
) -> AppResult<(
    StatusCode,
    Json<crate::models::EnqueueReflectionJobResponse>,
)> {
    let idempotency_key = require_idempotency_key(&headers)?.to_string();
    let req = decode_json(payload)?;
    validate_enqueue_reflection_job_request(&req)?;
    ensure_actor_matches_context(&req.actor, &auth)?;
    let response = run_idempotent_operation(
        &state,
        &auth.principal,
        "POST /v1/reflection/jobs",
        &idempotency_key,
        &fingerprint_request(&req)?,
        async { state.job_store.enqueue(&req.actor, req.trigger) },
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(response)))
}

async fn get_reflection_job_status(
    State(state): State<AppState>,
    Extension(auth): Extension<RuntimeAuthContext>,
    Path(job_id): Path<String>,
) -> AppResult<Json<crate::models::ReflectionJobStatusResponse>> {
    let status = state
        .job_store
        .get_scoped(&job_id, &auth.principal.user_id, &auth.principal.agent_id)?
        .ok_or_else(|| AppError::not_found("reflection job not found"))?;

    Ok(Json(status))
}

pub async fn runtime_auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    require_request_id(request.headers())?;
    let token = bearer_token(request.headers())?;
    if token != state.config.auth.runtime.token {
        return Err(AppError::unauthorized("invalid runtime bearer token"));
    }

    let principal = Principal {
        user_id: required_header(request.headers(), AUTH_USER_ID_HEADER)?.to_string(),
        agent_id: required_header(request.headers(), AUTH_AGENT_ID_HEADER)?.to_string(),
    };
    request
        .extensions_mut()
        .insert(RuntimeAuthContext { principal });

    Ok(next.run(request).await)
}

fn require_request_id(headers: &HeaderMap) -> AppResult<()> {
    let _ = required_header(headers, "x-request-id")?;
    Ok(())
}

fn require_idempotency_key<'a>(headers: &'a HeaderMap) -> AppResult<&'a str> {
    required_header(headers, "idempotency-key")
}

fn required_header<'a>(headers: &'a HeaderMap, name: &str) -> AppResult<&'a str> {
    let value = headers
        .get(name)
        .ok_or_else(|| AppError::invalid_request(format!("missing required header: {name}")))?;
    let text = value
        .to_str()
        .map_err(|_| AppError::invalid_request(format!("invalid header value for {name}")))?;
    if text.trim().is_empty() {
        return Err(AppError::invalid_request(format!(
            "header {name} cannot be empty"
        )));
    }
    Ok(text)
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("missing Authorization header"))?
        .to_str()
        .map_err(|_| AppError::unauthorized("invalid Authorization header encoding"))?;
    let prefix = "Bearer ";
    if !value.starts_with(prefix) {
        return Err(AppError::unauthorized(
            "Authorization header must use Bearer scheme",
        ));
    }
    let token = value[prefix.len()..].trim();
    if token.is_empty() {
        return Err(AppError::unauthorized("Bearer token cannot be empty"));
    }
    Ok(token.to_string())
}

fn decode_json<T>(payload: Result<Json<T>, JsonRejection>) -> AppResult<T> {
    payload
        .map(|Json(value)| value)
        .map_err(|err| AppError::invalid_request(format!("invalid JSON request body: {err}")))
}

fn ensure_actor_matches_context(actor: &Actor, auth: &RuntimeAuthContext) -> AppResult<()> {
    if actor.user_id != auth.principal.user_id || actor.agent_id != auth.principal.agent_id {
        return Err(AppError::forbidden(
            "actor principal does not match authenticated request context",
        ));
    }
    Ok(())
}

async fn run_idempotent_operation<T, F>(
    state: &AppState,
    principal: &Principal,
    operation: &str,
    idempotency_key: &str,
    request_fingerprint: &str,
    action: F,
) -> AppResult<T>
where
    F: Future<Output = AppResult<T>>,
{
    let reservation = state.idempotency_store.reserve(
        principal,
        operation,
        idempotency_key,
        request_fingerprint,
    )?;
    match action.await {
        Ok(value) => {
            reservation.mark_completed()?;
            Ok(value)
        }
        Err(err) => {
            if let Err(mark_err) = reservation.mark_failed() {
                return Err(AppError::internal(format!(
                    "protected operation failed and idempotency state could not be marked failed; operation_error={err:?}; transition_error={mark_err:?}",
                )));
            }
            Err(err)
        }
    }
}

fn fingerprint_request<T: Serialize>(request: &T) -> AppResult<String> {
    serde_json::to_string(request)
        .map_err(|err| AppError::internal(format!("failed to fingerprint request payload: {err}")))
}
