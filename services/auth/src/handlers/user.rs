use contracts::auth::{
    ChangePasswordRequest, GetUserRequest, ListUsersRequest, ListUsersResponse, OkResponse,
    UpdateProfileRequest, UserDto,
};
use shared::{AppError, nats::RequestContext};

use crate::{application, state::AuthState};

pub async fn get(
    state: AuthState,
    ctx: RequestContext,
    req: GetUserRequest,
) -> Result<UserDto, AppError> {
    application::user::get(state, ctx, req).await
}

pub async fn update(
    state: AuthState,
    ctx: RequestContext,
    req: UpdateProfileRequest,
) -> Result<UserDto, AppError> {
    application::user::update(state, ctx, req).await
}

pub async fn change_password(
    state: AuthState,
    ctx: RequestContext,
    req: ChangePasswordRequest,
) -> Result<OkResponse, AppError> {
    application::user::change_password(state, ctx, req).await
}

pub async fn list(
    state: AuthState,
    ctx: RequestContext,
    req: ListUsersRequest,
) -> Result<ListUsersResponse, AppError> {
    application::user::list(state, ctx, req).await
}
