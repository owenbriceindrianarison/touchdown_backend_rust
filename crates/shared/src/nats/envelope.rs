use async_nats::HeaderMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ids::new_id, locale::Locale, paseto::Role};

pub const HDR_USER_ID: &str = "X-User-Id";
pub const HDR_ROLE: &str = "X-User-Role";
pub const HDR_LOCALE: &str = "X-Locale";
pub const HDR_REQUEST_ID: &str = "X-Request-Id";
pub const HDR_TRACEPARENT: &str = "traceparent";
pub const HDR_MSG_ID: &str = "Nats-Msg-Id";

/// Call context, carried in the NATS HEADERS and not in the body.
///
/// Intended result: the body of an RPC message is exactly the same as the API's public JSON.
/// A DTO is serialized identically in both HTTP and NATS
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub user_id: Option<Uuid>,
    pub role: Option<Role>,
    pub locale: Locale,
    pub request_id: String,
    pub traceparent: Option<String>,
}

impl RequestContext {
    pub fn system() -> Self {
        Self {
            request_id: new_id().to_string(),
            ..Default::default()
        }
    }

    pub fn require_user(&self) -> Result<Uuid, crate::AppError> {
        self.user_id
            .ok_or_else(|| crate::AppError::Unauthorized("authentication required".into()))
    }

    pub fn require_role(&self, required: Role) -> Result<(), crate::AppError> {
        match self.role {
            Some(r) if r.satisfies(required) => Ok(()),
            Some(_) => Err(crate::AppError::Forbidden("insufficient role".into())),
            None => Err(crate::AppError::Unauthorized(
                "authentication required".into(),
            )),
        }
    }

    pub fn to_headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(uuid) = self.user_id {
            h.insert(HDR_USER_ID, uuid.to_string().as_str());
        }
        if let Some(role) = self.role {
            h.insert(HDR_ROLE, role.as_str());
        }
        h.insert(HDR_LOCALE, self.locale.as_str());
        h.insert(HDR_REQUEST_ID, self.request_id.as_str());
        h
    }

    pub fn from_headers(headers: Option<&HeaderMap>, default_locale: Locale) -> Self {
        let get = |k: &str| {
            headers
                .and_then(|h| h.get(k))
                .map(|v| v.as_str().to_string())
        };
        Self {
            user_id: get(HDR_USER_ID).and_then(|v| v.parse().ok()),
            role: get(HDR_ROLE).and_then(|v| Role::parse(&v)),
            locale: get(HDR_LOCALE)
                .and_then(|v| v.parse().ok())
                .unwrap_or(default_locale),
            request_id: get(HDR_REQUEST_ID).unwrap_or_else(|| new_id().to_string()),
            traceparent: get(HDR_TRACEPARENT),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Actor {
    pub user_id: Option<Uuid>,
    pub role: String,
}

impl Actor {
    pub fn system() -> Self {
        Self {
            user_id: None,
            role: "system".into(),
        }
    }
}

impl From<&RequestContext> for Actor {
    fn from(ctx: &RequestContext) -> Self {
        Self {
            user_id: ctx.user_id,
            role: ctx
                .role
                .map(|r| r.as_str().to_string())
                .unwrap_or_else(|| "system".into()),
        }
    }
}

/// Common wrapper for ALL JetStream events.
/// `event_id` serves as the `Nats-Msg-Id`: native server-side deduplication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<T> {
    pub event_id: Uuid,
    pub event_type: String,
    pub occured_at: DateTime<Utc>,
    pub version: u16,
    pub actor: Actor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    pub payload: T,
}

impl<T> EventEnvelope<T> {
    pub fn new(event_type: impl Into<String>, payload: T, actor: Actor) -> Self {
        Self {
            event_id: new_id(),
            event_type: event_type.into(),
            occured_at: Utc::now(),
            version: 1,
            actor,
            trace_id: None,
            payload,
        }
    }

    pub fn with_trace(mut self, trace_id: Option<String>) -> Self {
        self.trace_id = trace_id;
        self
    }
}
