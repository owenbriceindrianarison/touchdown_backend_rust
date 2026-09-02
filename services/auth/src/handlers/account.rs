use contracts::auth::{
    AcceptedResponse, AuthSession, ForgotPasswordRequest, LoginRequest, LogoutRequest, OkResponse,
    RefreshRequest, RegisterRequest, ResetPasswordRequest, VerifyEmailRequest,
};
use shared::{AppError, nats::RequestContext};

use crate::{application, state::AuthState};

pub async fn register(
    state: AuthState,
    ctx: RequestContext,
    req: RegisterRequest,
) -> Result<AuthSession, AppError> {
    application::account::register(state, ctx, req).await
}

pub async fn login(
    state: AuthState,
    ctx: RequestContext,
    req: LoginRequest,
) -> Result<AuthSession, AppError> {
    application::account::login(state, ctx, req).await
}

pub async fn refresh(
    state: AuthState,
    ctx: RequestContext,
    req: RefreshRequest,
) -> Result<AuthSession, AppError> {
    application::account::refresh(state, ctx, req).await
}

pub async fn logout(
    state: AuthState,
    ctx: RequestContext,
    req: LogoutRequest,
) -> Result<OkResponse, AppError> {
    application::account::logout(state, ctx, req).await
}

pub async fn request_password_reset(
    state: AuthState,
    ctx: RequestContext,
    req: ForgotPasswordRequest,
) -> Result<AcceptedResponse, AppError> {
    application::account::request_password_reset(state, ctx, req).await
}

pub async fn confirm_password_reset(
    state: AuthState,
    ctx: RequestContext,
    req: ResetPasswordRequest,
) -> Result<OkResponse, AppError> {
    application::account::confirm_password_reset(state, ctx, req).await
}

pub async fn confirm_email(
    state: AuthState,
    ctx: RequestContext,
    req: VerifyEmailRequest,
) -> Result<OkResponse, AppError> {
    application::account::confirm_email(state, ctx, req).await
}
