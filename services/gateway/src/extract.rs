use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, request::Parts},
};
use shared::{
    AppError,
    ids::new_id,
    locale::Locale,
    nats::RequestContext,
    paseto::{AccessClaims, Role},
};

use crate::state::AppState;

/// Request context, optional authentication.
///
/// A MISSING token yields an anonymous context; a PRESENT but invalid token yields a 401.
/// Treating an expired token as anonymous would let the client believe it is authenticated while receiving public data.
pub struct ReqCtx(pub RequestContext);

/// Required authentication.
pub struct AuthCtx(pub RequestContext, pub AccessClaims);

/// Required authentication + admin role.
pub struct AdminCtx(pub RequestContext);

impl FromRequestParts<AppState> for ReqCtx {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = extract_claims(&parts.headers, state)?;
        Ok(ReqCtx(build_ctx(&parts.headers, claims.as_ref(), state)))
    }
}

impl FromRequestParts<AppState> for AuthCtx {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = extract_claims(&parts.headers, state)?
            .ok_or_else(|| AppError::Unauthorized("authentication required".into()))?;
        let ctx = build_ctx(&parts.headers, Some(&claims), state);
        Ok(AuthCtx(ctx, claims))
    }
}

impl FromRequestParts<AppState> for AdminCtx {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthCtx(ctx, claims) = AuthCtx::from_request_parts(parts, state).await?;
        if !claims.role.satisfies(Role::Admin) {
            return Err(AppError::Forbidden("admin role required".into()));
        }
        Ok(AdminCtx(ctx))
    }
}

fn extract_claims(headers: &HeaderMap, state: &AppState) -> Result<Option<AccessClaims>, AppError> {
    let Some(value) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Ok(None);
    };
    let raw = value
        .to_str()
        .map_err(|_| AppError::Unauthorized("malformed Authorization header".into()))?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::Unauthorized("expected `Bearer <token>`".into()))?;

    state.verifier.verify(token.trim()).map(Some)
}

fn build_ctx(
    headers: &HeaderMap,
    claims: Option<&AccessClaims>,
    state: &AppState,
) -> RequestContext {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };

    // The URL/header locale takes precedence over the profile locale:
    // a French-speaking user can browse in English without changing their account.
    let locale = header("accept-language")
        .map(|h| Locale::from_accept_language(&h, state.cfg.default_locale))
        .or_else(|| claims.map(|c| c.locale))
        .unwrap_or(state.cfg.default_locale);

    RequestContext {
        user_id: claims.map(|c| c.user_id),
        role: claims.map(|c| c.role),
        locale,
        request_id: header("x-request-id").unwrap_or_else(|| new_id().to_string()),
        traceparent: header("traceparent"),
        // Behind a reverse proxy, the real IP is in X-Forwarded-For.
        client_ip: header("x-forwarded-for")
            .and_then(|v| v.split(',').next().map(|s| s.trim().to_string()))
            .or_else(|| header("x-real-ip")),
        user_agent: header("user-agent"),
    }
}
