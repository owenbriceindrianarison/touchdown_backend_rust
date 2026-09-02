use contracts::{
    auth::{
        ChangePasswordRequest, GetUserRequest, ListUsersRequest, ListUsersResponse, OkResponse,
        UpdateProfileRequest, UserDto, events,
    },
    validation::validate,
};
use shared::{
    AppError,
    nats::{Actor, EventEnvelope, RequestContext},
    outbox,
    pagination::{Page, PageParams},
    paseto::Role,
};

use crate::{
    domain::ports::UserRepository, domain::user::PasswordHash, password, state::AuthState,
};

pub async fn get(
    state: AuthState,
    ctx: RequestContext,
    req: GetUserRequest,
) -> Result<UserDto, AppError> {
    let me = ctx.require_user()?;
    let target = req.user_id.unwrap_or(me);

    // Reading another user's account requires the staff role.
    if target != me {
        ctx.require_role(Role::Staff)?;
    }

    state
        .repo
        .find_by_id(target)
        .await?
        .map(|u| u.to_dto())
        .ok_or_else(|| AppError::not_found("user", target))
}

pub async fn update(
    state: AuthState,
    ctx: RequestContext,
    req: UpdateProfileRequest,
) -> Result<UserDto, AppError> {
    validate(&req)?;
    let user_id = ctx.require_user()?;

    let user = state
        .repo
        .update_profile(
            user_id,
            req.first_name.as_deref(),
            req.last_name.as_deref(),
            req.phone.as_deref(),
            req.locale.map(|l| l.as_str()),
        )
        .await?;

    let mut tx = state.pool.begin().await?;
    let envelope = EventEnvelope::new(
        events::USER_UPDATED,
        events::UserSimpleEvent {
            user_id,
            email: user.email.as_str().to_owned(),
            locale: user.locale,
        },
        Actor::from(&ctx),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(user.to_dto())
}

pub async fn change_password(
    state: AuthState,
    ctx: RequestContext,
    req: ChangePasswordRequest,
) -> Result<OkResponse, AppError> {
    validate(&req)?;
    let user_id = ctx.require_user()?;

    let user = state
        .repo
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::not_found("user", user_id))?;

    if !password::verify(&req.current_password, user.password_hash.as_str()) {
        return Err(AppError::Unauthorized(
            "current password is incorrect".into(),
        ));
    }

    let hash_str = password::hash(&req.new_password)?;
    let hash = PasswordHash::new(hash_str);
    let mut tx = state.pool.begin().await?;
    state.repo.set_password(&mut tx, user_id, &hash).await?;
    state
        .repo
        .revoke_all_user_tokens(&mut tx, user_id, "password_changed")
        .await?;

    let envelope = EventEnvelope::new(
        events::USER_PASSWORD_CHANGED,
        events::UserSimpleEvent {
            user_id,
            email: user.email.as_str().to_owned(),
            locale: user.locale,
        },
        Actor::from(&ctx),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(OkResponse { ok: true })
}

pub async fn list(
    state: AuthState,
    ctx: RequestContext,
    req: ListUsersRequest,
) -> Result<ListUsersResponse, AppError> {
    ctx.require_role(Role::Staff)?;

    let params = PageParams {
        page: req.page.unwrap_or(1),
        per_page: req.per_page.unwrap_or(20),
    }
    .normalized();

    let (users, total) = UserRepository::list(
        &*state.repo,
        req.role.map(|r| r.as_str()),
        req.search.as_deref(),
        params.limit(),
        params.offset(),
    )
    .await?;

    Ok(Page::new(
        users.iter().map(|u| u.to_dto()).collect(),
        total,
        params,
    ))
}
