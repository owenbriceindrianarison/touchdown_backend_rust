use axum::{Json, extract::State, http::StatusCode};
use contracts::auth::{
    AcceptedResponse, AuthSession, ForgotPasswordRequest, LoginRequest, LogoutRequest, OkResponse,
    RefreshRequest, RegisterRequest, ResetPasswordRequest, VerifyEmailRequest, subjects,
};
use shared::AppError;

use crate::{extract::ReqCtx, state::AppState};

#[utoipa::path(
    post, path = "/auth/register", tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Account created, session opened", body = AuthSession),
        (status = 400, description = "Validation failed"),
        (status = 409, description = "Email already in use"),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    ReqCtx(ctx): ReqCtx,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthSession>), AppError> {
    let res: AuthSession = state.rpc.request(subjects::REGISTER, &ctx, &body).await?;
    Ok((StatusCode::CREATED, Json(res)))
}

#[utoipa::path(
    post, path = "/auth/login", tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Session opened", body = AuthSession),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Account locked or inactive"),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    ReqCtx(ctx): ReqCtx,
    Json(body): Json<LoginRequest>,
) -> Result<Json<AuthSession>, AppError> {
    Ok(Json(state.rpc.request(subjects::LOGIN, &ctx, &body).await?))
}

#[utoipa::path(
    post, path = "/auth/refresh", tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "New token pair issued", body = AuthSession),
        (status = 401, description = "Token invalid, expired, or reuse detected"),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    ReqCtx(ctx): ReqCtx,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<AuthSession>, AppError> {
    Ok(Json(
        state.rpc.request(subjects::REFRESH, &ctx, &body).await?,
    ))
}

#[utoipa::path(
    post, path = "/auth/logout", tag = "auth",
    request_body = LogoutRequest,
    responses((status = 200, description = "Session revoked", body = OkResponse))
)]
pub async fn logout(
    State(state): State<AppState>,
    ReqCtx(ctx): ReqCtx,
    Json(body): Json<LogoutRequest>,
) -> Result<Json<OkResponse>, AppError> {
    Ok(Json(
        state.rpc.request(subjects::LOGOUT, &ctx, &body).await?,
    ))
}

#[utoipa::path(
    post, path = "/auth/password/forgot", tag = "auth",
    request_body = ForgotPasswordRequest,
    responses((status = 202, description = "Request accepted (same response if email is unknown)", body = AcceptedResponse))
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    ReqCtx(ctx): ReqCtx,
    Json(body): Json<ForgotPasswordRequest>,
) -> Result<(StatusCode, Json<AcceptedResponse>), AppError> {
    let res = state
        .rpc
        .request(subjects::REQUEST_RESET, &ctx, &body)
        .await?;
    Ok((StatusCode::ACCEPTED, Json(res)))
}

#[utoipa::path(
    post, path = "/auth/password/reset", tag = "auth",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Password changed, all sessions revoked", body = OkResponse),
        (status = 400, description = "Token invalid or expired"),
    )
)]
pub async fn reset_password(
    State(state): State<AppState>,
    ReqCtx(ctx): ReqCtx,
    Json(body): Json<ResetPasswordRequest>,
) -> Result<Json<OkResponse>, AppError> {
    Ok(Json(
        state
            .rpc
            .request(subjects::CONFIRM_RESET, &ctx, &body)
            .await?,
    ))
}

#[utoipa::path(
    post, path = "/auth/email/verify", tag = "auth",
    request_body = VerifyEmailRequest,
    responses(
        (status = 200, description = "Email verified", body = OkResponse),
        (status = 400, description = "Token invalid or expired"),
    )
)]
pub async fn verify_email(
    State(state): State<AppState>,
    ReqCtx(ctx): ReqCtx,
    Json(body): Json<VerifyEmailRequest>,
) -> Result<Json<OkResponse>, AppError> {
    Ok(Json(
        state
            .rpc
            .request(subjects::CONFIRM_EMAIL, &ctx, &body)
            .await?,
    ))
}
