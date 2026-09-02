use contracts::{
    auth::{
        AddressDto, AddressIdRequest, CreateAddressRequest, OkResponse, SetDefaultAddressRequest,
        UpdateAddressRequest, events,
    },
    validation::validate,
};
use shared::{
    AppError,
    nats::{Actor, EventEnvelope, RequestContext},
    outbox,
};

use crate::{
    domain::ports::{AddressRepository, UpsertAddressCmd},
    state::AuthState,
};

pub async fn list(
    state: AuthState,
    ctx: RequestContext,
    _req: serde_json::Value,
) -> Result<Vec<AddressDto>, AppError> {
    let user_id = ctx.require_user()?;
    let addresses = AddressRepository::list(&*state.repo, user_id).await?;
    Ok(addresses.iter().map(|a| a.to_dto()).collect())
}

pub async fn get(
    state: AuthState,
    ctx: RequestContext,
    req: AddressIdRequest,
) -> Result<AddressDto, AppError> {
    let user_id = ctx.require_user()?;
    Ok(state.repo.get(user_id, req.address_id).await?.to_dto())
}

pub async fn create(
    state: AuthState,
    ctx: RequestContext,
    req: CreateAddressRequest,
) -> Result<AddressDto, AppError> {
    validate(&req)?;
    let user_id = ctx.require_user()?;
    let country = req.country.to_uppercase();

    let mut tx = state.pool.begin().await?;

    if req.is_default_shipping || req.is_default_billing {
        state
            .repo
            .clear_defaults(
                &mut tx,
                user_id,
                req.is_default_shipping,
                req.is_default_billing,
                None,
            )
            .await?;
    }

    let address = AddressRepository::create(
        &*state.repo,
        &mut tx,
        user_id,
        UpsertAddressCmd {
            label: req.label.as_deref(),
            kind: req.kind,
            first_name: &req.first_name,
            last_name: &req.last_name,
            company: req.company.as_deref(),
            line1: &req.line1,
            line2: req.line2.as_deref(),
            postal_code: &req.postal_code,
            city: &req.city,
            state: req.state.as_deref(),
            country: &country,
            phone: req.phone.as_deref(),
            is_default_shipping: req.is_default_shipping,
            is_default_billing: req.is_default_billing,
        },
    )
    .await?;

    let envelope = EventEnvelope::new(
        events::ADDRESS_CREATED,
        events::AddressChanged {
            user_id,
            address_id: address.id,
            country,
        },
        Actor::from(&ctx),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(address.to_dto())
}

pub async fn update(
    state: AuthState,
    ctx: RequestContext,
    req: UpdateAddressRequest,
) -> Result<AddressDto, AppError> {
    validate(&req.data)?;
    let user_id = ctx.require_user()?;
    let data = req.data;
    let country = data.country.to_uppercase();

    let mut tx = state.pool.begin().await?;

    if data.is_default_shipping || data.is_default_billing {
        state
            .repo
            .clear_defaults(
                &mut tx,
                user_id,
                data.is_default_shipping,
                data.is_default_billing,
                Some(req.address_id),
            )
            .await?;
    }

    let address = state
        .repo
        .update(
            &mut tx,
            req.address_id,
            user_id,
            UpsertAddressCmd {
                label: data.label.as_deref(),
                kind: data.kind,
                first_name: &data.first_name,
                last_name: &data.last_name,
                company: data.company.as_deref(),
                line1: &data.line1,
                line2: data.line2.as_deref(),
                postal_code: &data.postal_code,
                city: &data.city,
                state: data.state.as_deref(),
                country: &country,
                phone: data.phone.as_deref(),
                is_default_shipping: data.is_default_shipping,
                is_default_billing: data.is_default_billing,
            },
        )
        .await?;

    let envelope = EventEnvelope::new(
        events::ADDRESS_UPDATED,
        events::AddressChanged {
            user_id,
            address_id: address.id,
            country,
        },
        Actor::from(&ctx),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(address.to_dto())
}

pub async fn set_default(
    state: AuthState,
    ctx: RequestContext,
    req: SetDefaultAddressRequest,
) -> Result<AddressDto, AppError> {
    let user_id = ctx.require_user()?;
    let mut tx = state.pool.begin().await?;

    state
        .repo
        .clear_defaults(
            &mut tx,
            user_id,
            req.shipping,
            req.billing,
            Some(req.address_id),
        )
        .await?;

    let address = state
        .repo
        .set_default(&mut tx, req.address_id, user_id, req.shipping, req.billing)
        .await?;

    tx.commit().await?;
    Ok(address.to_dto())
}

pub async fn delete(
    state: AuthState,
    ctx: RequestContext,
    req: AddressIdRequest,
) -> Result<OkResponse, AppError> {
    let user_id = ctx.require_user()?;
    let mut tx = state.pool.begin().await?;

    let affected = state.repo.delete(&mut tx, user_id, req.address_id).await?;

    if affected == 0 {
        return Err(AppError::not_found("address", req.address_id));
    }

    let envelope = EventEnvelope::new(
        events::ADDRESS_DELETED,
        events::AddressChanged {
            user_id,
            address_id: req.address_id,
            country: String::new(),
        },
        Actor::from(&ctx),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(OkResponse { ok: true })
}
