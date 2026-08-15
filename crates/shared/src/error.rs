use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed")]
    Validation {
        fields: BTreeMap<String, Vec<String>>,
    },

    #[error("bas request: {0}")]
    BadRequest(String),

    #[error("{resource} not found")]
    NotFound { resource: String, id: String },

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("rate limited")]
    RateLimited { retry_after_secs: u64 },

    #[error("upstream timeout: {0}")]
    Timeout(String),

    #[error("upstream unavailable: {0}")]
    Upstream(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("remote error: {}", body.message)]
    Remote { status: u16, body: ErrorBody },

    #[error("internal error")]
    Internal(#[source] anyhow::Error),
}

impl AppError {
    pub fn internal(e: impl Into<anyhow::Error>) -> Self {
        Self::Internal(e.into())
    }

    pub fn not_found(resource: impl Into<String>, id: impl std::fmt::Display) -> Self {
        Self::NotFound {
            resource: resource.into(),
            id: id.to_string(),
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::Validation { .. } => "validation_failed",

            Self::BadRequest(_) => "bad_request",

            Self::NotFound { .. } => "not_found",

            Self::Conflict(_) => "conflict",

            Self::Unauthorized(_) => "unauthorized",

            Self::Forbidden(_) => "forbidden",

            Self::RateLimited { .. } => "rate_limited",

            Self::Timeout(_) => "upstream_timeout",

            Self::Upstream(_) => "upstream_unavailable",

            Self::Config(_) => "configuration_error",

            Self::Remote { body, .. } => &body.code,

            Self::Internal(_) => "internal_error",
        }
    }

    pub fn status(&self) -> u16 {
        match self {
            Self::Validation { .. } | Self::BadRequest(_) => 400,

            Self::Unauthorized(_) => 401,

            Self::Forbidden(_) => 403,

            Self::NotFound { .. } => 404,

            Self::Conflict(_) => 409,

            Self::RateLimited { .. } => 429,

            Self::Timeout(_) => 504,

            Self::Upstream(_) => 503,

            Self::Config(_) | Self::Internal(_) => 500,

            Self::Remote { status, .. } => *status,
        }
    }

    pub fn to_body(&self) -> ErrorBody {
        match self {
            Self::Remote { body, .. } => body.clone(),

            Self::Validation { fields } => ErrorBody {
                code: self.code().to_string(),
                message: "Validation failed".to_string(),
                details: Some(fields.clone()),
                trace_id: None,
            },

            Self::Internal(cause) => {
                tracing::error!(error = ?cause, "internal error");
                ErrorBody {
                    code: self.code().to_string(),
                    message: "An internal error occured".to_string(),
                    details: None,
                    trace_id: None,
                }
            }

            other => ErrorBody {
                code: other.code().to_string(),
                message: other.to_string(),
                details: None,
                trace_id: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,

    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, Vec<String>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                AppError::Conflict(db.constraint().unwrap_or("unique").to_string())
            }

            sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
                AppError::BadRequest(db.constraint().unwrap_or("foreign key").to_string())
            }

            _ => AppError::Internal(e.into()),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::BadRequest(format!("invalid payload: {e}"))
    }
}

impl From<async_nats::RequestError> for AppError {
    fn from(e: async_nats::RequestError) -> Self {
        match e.kind() {
            async_nats::RequestErrorKind::TimedOut => AppError::Timeout(e.to_string()),

            async_nats::RequestErrorKind::NoResponders => AppError::Upstream(e.to_string()),

            _ => AppError::Upstream(e.to_string()),
        }
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = axum::http::StatusCode::from_u16(self.status())
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        let mut response = (status, axum::Json(self.to_body())).into_response();

        if let AppError::RateLimited { retry_after_secs } = self {
            if let Ok(v) = axum::http::HeaderValue::from_str(&retry_after_secs.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, v);
            }
        }

        response
    }
}
