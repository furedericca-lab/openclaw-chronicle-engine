use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    InvalidRequest,
    NotFound,
    Conflict,
    BackendUnavailable,
    UpstreamEmbeddingError,
    UpstreamRerankError,
    UpstreamReflectionError,
    RateLimited,
    IdempotencyConflict,
    InternalError,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::BackendUnavailable => "BACKEND_UNAVAILABLE",
            Self::UpstreamEmbeddingError => "UPSTREAM_EMBEDDING_ERROR",
            Self::UpstreamRerankError => "UPSTREAM_RERANK_ERROR",
            Self::UpstreamReflectionError => "UPSTREAM_REFLECTION_ERROR",
            Self::RateLimited => "RATE_LIMITED",
            Self::IdempotencyConflict => "IDEMPOTENCY_CONFLICT",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: ErrorCode,
    message: String,
    retryable: bool,
}

impl AppError {
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthorized,
            false,
            message,
        )
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, ErrorCode::Forbidden, false, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            ErrorCode::InvalidRequest,
            false,
            message,
        )
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, ErrorCode::NotFound, false, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, ErrorCode::Conflict, false, message)
    }

    pub fn idempotency_conflict(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::CONFLICT,
            ErrorCode::IdempotencyConflict,
            false,
            message,
        )
    }

    pub fn backend_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::BackendUnavailable,
            true,
            message,
        )
    }

    pub fn upstream_embedding(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::UpstreamEmbeddingError,
            true,
            message,
        )
    }

    pub fn upstream_rerank(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::UpstreamRerankError,
            true,
            message,
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::InternalError,
            false,
            message,
        )
    }

    pub fn upstream_reflection(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::UpstreamReflectionError,
            true,
            message,
        )
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn new(
        status: StatusCode,
        code: ErrorCode,
        retryable: bool,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            retryable,
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal(err.to_string())
    }
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorPayload,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    code: &'static str,
    message: String,
    retryable: bool,
    details: serde_json::Value,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            error: ErrorPayload {
                code: self.code.as_str(),
                message: self.message,
                retryable: self.retryable,
                details: serde_json::json!({}),
            },
        };
        (self.status, Json(body)).into_response()
    }
}
