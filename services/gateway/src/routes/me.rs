use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use contracts::auth::{
    AddressDto, AddressIdRequest, ChangePasswordRequest, CreateAddressRequest, GetUserRequest,
    ListUsersRequest, ListUsersResponse, OkResponse, SetDefaultAddressRequest,
    UpdateAddressRequest, UpdateProfileRequest, UserDto, subjects,
};
use shared::AppError;
use uuid::Uuid;

use crate::{
    extract::{AdminCtx, AuthCtx},
    state::AppState,
};

#[utoipa::path(
    get, path = "/me", tag = "account",
    security(("paseto" = [])),
    responses(
        (status = 200, description = "Current profile", body = UserDto),
        (status = 401, description = "Unauthenticated"),
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
) -> Result<Json<UserDto>, AppError> {
    let req = GetUserRequest { user_id: None };
    Ok(Json(
        state.rpc.request(subjects::GET_USER, &ctx, &req).await?,
    ))
}

#[utoipa::path(
    patch, path = "/me", tag = "account",
    security(("paseto" = [])),
    request_body = UpdateProfileRequest,
    responses((status = 200, description = "Profile updated", body = UserDto))
)]
pub async fn update_me(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
    Json(body): Json<UpdateProfileRequest>,
) -> Result<Json<UserDto>, AppError> {
    Ok(Json(
        state
            .rpc
            .request(subjects::UPDATE_USER, &ctx, &body)
            .await?,
    ))
}

#[utoipa::path(
    post, path = "/me/password", tag = "account",
    security(("paseto" = [])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed, sessions revoked", body = OkResponse),
        (status = 401, description = "Current password incorrect"),
    )
)]
pub async fn change_password(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<OkResponse>, AppError> {
    Ok(Json(
        state
            .rpc
            .request(subjects::CHANGE_PASSWORD, &ctx, &body)
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/me/addresses", tag = "account",
    security(("paseto" = [])),
    responses((status = 200, description = "Account addresses", body = Vec<AddressDto>))
)]
pub async fn list_addresses(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
) -> Result<Json<Vec<AddressDto>>, AppError> {
    let empty = serde_json::json!({});
    Ok(Json(
        state
            .rpc
            .request(subjects::ADDRESS_LIST, &ctx, &empty)
            .await?,
    ))
}

#[utoipa::path(
    post, path = "/me/addresses", tag = "account",
    security(("paseto" = [])),
    request_body = CreateAddressRequest,
    responses((status = 201, description = "Address created", body = AddressDto))
)]
pub async fn create_address(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
    Json(body): Json<CreateAddressRequest>,
) -> Result<(StatusCode, Json<AddressDto>), AppError> {
    let res = state
        .rpc
        .request(subjects::ADDRESS_CREATE, &ctx, &body)
        .await?;
    Ok((StatusCode::CREATED, Json(res)))
}

#[utoipa::path(
    get, path = "/me/addresses/{id}", tag = "account",
    security(("paseto" = [])),
    params(("id" = Uuid, Path, description = "Address ID")),
    responses(
        (status = 200, body = AddressDto),
        (status = 404, description = "Address not found"),
    )
)]
pub async fn get_address(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
    Path(id): Path<Uuid>,
) -> Result<Json<AddressDto>, AppError> {
    let req = AddressIdRequest { address_id: id };
    Ok(Json(
        state.rpc.request(subjects::ADDRESS_GET, &ctx, &req).await?,
    ))
}

#[utoipa::path(
    put, path = "/me/addresses/{id}", tag = "account",
    security(("paseto" = [])),
    params(("id" = Uuid, Path)),
    request_body = CreateAddressRequest,
    responses((status = 200, body = AddressDto), (status = 404))
)]
pub async fn update_address(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateAddressRequest>,
) -> Result<Json<AddressDto>, AppError> {
    let req = UpdateAddressRequest {
        address_id: id,
        data: body,
    };
    Ok(Json(
        state
            .rpc
            .request(subjects::ADDRESS_UPDATE, &ctx, &req)
            .await?,
    ))
}

#[utoipa::path(
    post, path = "/me/addresses/{id}/default", tag = "account",
    security(("paseto" = [])),
    params(("id" = Uuid, Path)),
    request_body = SetDefaultAddressRequest,
    responses((status = 200, body = AddressDto), (status = 404))
)]
pub async fn set_default_address(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
    Path(id): Path<Uuid>,
    Json(mut body): Json<SetDefaultAddressRequest>,
) -> Result<Json<AddressDto>, AppError> {
    body.address_id = id;
    Ok(Json(
        state
            .rpc
            .request(subjects::ADDRESS_SET_DEFAULT, &ctx, &body)
            .await?,
    ))
}

#[utoipa::path(
    delete, path = "/me/addresses/{id}", tag = "account",
    security(("paseto" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = OkResponse), (status = 404))
)]
pub async fn delete_address(
    State(state): State<AppState>,
    AuthCtx(ctx, _): AuthCtx,
    Path(id): Path<Uuid>,
) -> Result<Json<OkResponse>, AppError> {
    let req = AddressIdRequest { address_id: id };
    Ok(Json(
        state
            .rpc
            .request(subjects::ADDRESS_DELETE, &ctx, &req)
            .await?,
    ))
}

#[utoipa::path(
    get, path = "/admin/users", tag = "admin",
    security(("paseto" = [])),
    params(ListUsersRequest),
    responses(
        (status = 200, description = "Paginated list of accounts", body = ListUsersResponse),
        (status = 403, description = "Insufficient role"),
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    AdminCtx(ctx): AdminCtx,
    Query(query): Query<ListUsersRequest>,
) -> Result<Json<ListUsersResponse>, AppError> {
    Ok(Json(
        state
            .rpc
            .request(subjects::LIST_USERS, &ctx, &query)
            .await?,
    ))
}
