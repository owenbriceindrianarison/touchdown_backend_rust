use chrono::{Duration, Utc};
use contracts::auth::{
    AcceptedResponse, AuthSession, ForgotPasswordRequest, LoginRequest, LogoutRequest, OkResponse,
    RefreshRequest, RegisterRequest, ResetPasswordRequest, VerifyEmailRequest, events,
};
use contracts::validation::validate;
use shared::{
    AppError,
    nats::{Actor, EventEnvelope, RequestContext},
    outbox,
};

use crate::{
    application::session,
    domain::email::Email,
    domain::ports::{CreateUserCmd, UserRepository},
    password,
    state::AuthState,
    tokens,
};

pub async fn register(
    state: AuthState,
    ctx: RequestContext,
    req: RegisterRequest,
) -> Result<AuthSession, AppError> {
    validate(&req)?;
    let email = Email::parse(&req.email)?;
    let password_hash_str = password::hash(&req.password)?;
    let password_hash = crate::domain::user::PasswordHash::new(password_hash_str);

    let mut tx = state.pool.begin().await?;

    let user = UserRepository::create(
        &*state.repo,
        &mut tx,
        CreateUserCmd {
            email: email.as_str(),
            password_hash: &password_hash,
            first_name: req.first_name.as_deref(),
            last_name: req.last_name.as_deref(),
            locale: req.locale.as_str(),
        },
    )
    .await?;

    let verification_token = tokens::generate();
    state
        .repo
        .insert_single_use_token(
            &mut tx,
            "email_verification_tokens",
            user.id,
            &tokens::hash(&verification_token),
            Utc::now() + Duration::hours(AuthState::VERIFY_TOKEN_TTL_HOURS),
            Some(email.as_str()),
        )
        .await?;

    if req.marketing_consent {
        state
            .repo
            .record_marketing_consent(&mut tx, user.id, ctx.client_ip.as_deref())
            .await?;
    }

    let envelope = EventEnvelope::new(
        events::USER_REGISTERED,
        events::UserRegistered {
            user_id: user.id,
            email: email.to_string(),
            first_name: user.first_name.clone(),
            locale: user.locale,
            verification_token,
        },
        Actor::system(),
    )
    .with_trace(ctx.traceparent.clone());
    outbox::enqueue(&mut tx, &envelope).await?;

    let out = session::issue(&state, &mut tx, &user, None, None, &ctx).await?;
    tx.commit().await?;

    tracing::info!(user_id = %user.id, "user registered");
    Ok(out)
}

pub async fn login(
    state: AuthState,
    ctx: RequestContext,
    req: LoginRequest,
) -> Result<AuthSession, AppError> {
    validate(&req)?;
    let email = Email::parse(&req.email)?;

    let Some(user) = state.repo.find_by_email(email.as_str()).await? else {
        // Constant-time response even for unknown emails: prevents timing oracle.
        password::dummy_verify(&req.password);
        return Err(AppError::Unauthorized("invalid credentials".into()));
    };

    // Distinguish Unauthorized (wrong password) from Forbidden (locked/inactive).
    match user.authenticate(&req.password) {
        Ok(()) => {}
        Err(AppError::Unauthorized(_)) => {
            // Wrong password → increment the failure counter.
            state
                .repo
                .record_login_failure(
                    user.id,
                    AuthState::MAX_LOGIN_ATTEMPTS,
                    Duration::minutes(AuthState::LOCK_MINUTES),
                )
                .await?;
            return Err(AppError::Unauthorized("invalid credentials".into()));
        }
        Err(e) => return Err(e),
    }

    let mut tx = state.pool.begin().await?;
    let out = session::issue(&state, &mut tx, &user, None, None, &ctx).await?;

    let envelope = EventEnvelope::new(
        events::USER_LOGGED_IN,
        events::UserLoggedIn {
            user_id: user.id,
            session_id: shared::ids::new_id(),
            client_ip: ctx.client_ip.clone(),
            user_agent: ctx.user_agent.clone(),
        },
        Actor::system(),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    state.repo.record_login_success(user.id).await?;
    Ok(out)
}

/// Refresh token rotation with reuse detection.
/// A token presented twice signals likely theft: the entire family is revoked.
pub async fn refresh(
    state: AuthState,
    ctx: RequestContext,
    req: RefreshRequest,
) -> Result<AuthSession, AppError> {
    validate(&req)?;
    let hash = tokens::hash(&req.refresh_token);

    let Some(row) = state.repo.find_refresh_token(&hash).await? else {
        return Err(AppError::Unauthorized("invalid refresh token".into()));
    };

    if row.revoked_at.is_some() {
        let revoked = state
            .repo
            .revoke_family(row.family_id, "reuse_detected")
            .await?;
        tracing::warn!(
            user_id = %row.user_id,
            family_id = %row.family_id,
            revoked,
            "refresh token reuse detected, family revoked"
        );
        return Err(AppError::Unauthorized(
            "refresh token reuse detected".into(),
        ));
    }

    if row.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized("refresh token expired".into()));
    }

    let user = state
        .repo
        .find_by_id(row.user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("account no longer exists".into()))?;

    if !user.status.is_active() {
        return Err(AppError::Forbidden("account is not active".into()));
    }

    let mut tx = state.pool.begin().await?;
    state.repo.revoke_token(&mut tx, row.id, "rotated").await?;
    let out = session::issue(
        &state,
        &mut tx,
        &user,
        Some(row.family_id),
        Some(row.id),
        &ctx,
    )
    .await?;
    tx.commit().await?;

    Ok(out)
}

pub async fn logout(
    state: AuthState,
    _ctx: RequestContext,
    req: LogoutRequest,
) -> Result<OkResponse, AppError> {
    let hash = tokens::hash(&req.refresh_token);
    // Same response whether the token exists or not: prevents validity oracle.
    if let Some(row) = state.repo.find_refresh_token(&hash).await? {
        state.repo.revoke_family(row.family_id, "logout").await?;
    }
    Ok(OkResponse { ok: true })
}

pub async fn request_password_reset(
    state: AuthState,
    ctx: RequestContext,
    req: ForgotPasswordRequest,
) -> Result<AcceptedResponse, AppError> {
    validate(&req)?;
    let email = Email::parse(&req.email)?;

    // Always 202, even for unknown emails: prevents account enumeration.
    let Some(user) = state.repo.find_by_email(email.as_str()).await? else {
        tracing::debug!("password reset requested for unknown email");
        return Ok(AcceptedResponse { accepted: true });
    };

    let reset_token = tokens::generate();
    let mut tx = state.pool.begin().await?;
    state
        .repo
        .insert_single_use_token(
            &mut tx,
            "password_reset_tokens",
            user.id,
            &tokens::hash(&reset_token),
            Utc::now() + Duration::minutes(AuthState::RESET_TOKEN_TTL_MINUTES),
            None,
        )
        .await?;

    let envelope = EventEnvelope::new(
        events::USER_PASSWORD_RESET_REQUESTED,
        events::PasswordResetRequested {
            user_id: user.id,
            email: email.to_string(),
            locale: user.locale,
            reset_token,
        },
        Actor::system(),
    )
    .with_trace(ctx.traceparent.clone());
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(AcceptedResponse { accepted: true })
}

pub async fn confirm_password_reset(
    state: AuthState,
    _ctx: RequestContext,
    req: ResetPasswordRequest,
) -> Result<OkResponse, AppError> {
    validate(&req)?;

    let mut tx = state.pool.begin().await?;
    let user_id = state
        .repo
        .consume_single_use_token(&mut tx, "password_reset_tokens", &tokens::hash(&req.token))
        .await?;

    let hash_str = password::hash(&req.new_password)?;
    let hash = crate::domain::user::PasswordHash::new(hash_str);
    state.repo.set_password(&mut tx, user_id, &hash).await?;
    // Invalidates all sessions: the only way to regain control if the account was compromised.
    state
        .repo
        .revoke_all_user_tokens(&mut tx, user_id, "password_changed")
        .await?;

    // Read outside tx: we only need email/locale for the event payload.
    let user = state
        .repo
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::not_found("user", user_id))?;

    let envelope = EventEnvelope::new(
        events::USER_PASSWORD_CHANGED,
        events::UserSimpleEvent {
            user_id,
            email: user.email.as_str().to_owned(),
            locale: user.locale,
        },
        Actor::system(),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(OkResponse { ok: true })
}

pub async fn confirm_email(
    state: AuthState,
    _ctx: RequestContext,
    req: VerifyEmailRequest,
) -> Result<OkResponse, AppError> {
    validate(&req)?;

    let mut tx = state.pool.begin().await?;
    let user_id = state
        .repo
        .consume_single_use_token(
            &mut tx,
            "email_verification_tokens",
            &tokens::hash(&req.token),
        )
        .await?;
    state.repo.mark_email_verified(&mut tx, user_id).await?;

    let user = state
        .repo
        .find_by_id(user_id)
        .await?
        .ok_or_else(|| AppError::not_found("user", user_id))?;

    let envelope = EventEnvelope::new(
        events::USER_EMAIL_VERIFIED,
        events::UserSimpleEvent {
            user_id,
            email: user.email.as_str().to_owned(),
            locale: user.locale,
        },
        Actor::system(),
    );
    outbox::enqueue(&mut tx, &envelope).await?;
    tx.commit().await?;

    Ok(OkResponse { ok: true })
}
