use contracts::auth::{
    AddressDto, AddressIdRequest, CreateAddressRequest, OkResponse, SetDefaultAddressRequest,
    UpdateAddressRequest,
};
use shared::{AppError, nats::RequestContext};

use crate::{application, state::AuthState};

pub async fn list(
    state: AuthState,
    ctx: RequestContext,
    req: serde_json::Value,
) -> Result<Vec<AddressDto>, AppError> {
    application::address::list(state, ctx, req).await
}

pub async fn get(
    state: AuthState,
    ctx: RequestContext,
    req: AddressIdRequest,
) -> Result<AddressDto, AppError> {
    application::address::get(state, ctx, req).await
}

pub async fn create(
    state: AuthState,
    ctx: RequestContext,
    req: CreateAddressRequest,
) -> Result<AddressDto, AppError> {
    application::address::create(state, ctx, req).await
}

pub async fn update(
    state: AuthState,
    ctx: RequestContext,
    req: UpdateAddressRequest,
) -> Result<AddressDto, AppError> {
    application::address::update(state, ctx, req).await
}

pub async fn set_default(
    state: AuthState,
    ctx: RequestContext,
    req: SetDefaultAddressRequest,
) -> Result<AddressDto, AppError> {
    application::address::set_default(state, ctx, req).await
}

pub async fn delete(
    state: AuthState,
    ctx: RequestContext,
    req: AddressIdRequest,
) -> Result<OkResponse, AppError> {
    application::address::delete(state, ctx, req).await
}
