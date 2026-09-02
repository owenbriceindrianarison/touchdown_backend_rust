use chrono::{Duration, Utc};
use contracts::auth::{AuthSession, AuthTokens};
use shared::{AppError, ids::new_id, nats::RequestContext};
use uuid::Uuid;

use crate::domain::ports::InsertRefreshTokenCmd;
use crate::domain::user::User;
use crate::state::AuthState;
use crate::tokens;

/// Issues an access + refresh token pair and persists the refresh token in its family.
///
/// `family_id` identifies the session: it is carried over unchanged on rotations,
/// which allows revoking an entire session at once and detecting token theft.
pub async fn issue(
    state: &AuthState,
    tx: &mut crate::domain::ports::Tx<'_>,
    user: &User,
    family_id: Option<Uuid>,
    parent_id: Option<Uuid>,
    ctx: &RequestContext,
) -> Result<AuthSession, AppError> {
    let family_id = family_id.unwrap_or_else(new_id);

    let (access_token, claims) =
        state
            .issuer
            .issue_access(user.id, user.role, user.locale, family_id)?;

    let refresh_token = tokens::generate();
    let refresh_hash = tokens::hash(&refresh_token);
    let expires_at = Utc::now()
        + Duration::from_std(state.cfg.paseto.refresh_ttl).unwrap_or_else(|_| Duration::days(30));

    state
        .repo
        .insert_refresh_token(
            tx,
            InsertRefreshTokenCmd {
                user_id: user.id,
                family_id,
                parent_id,
                token_hash: &refresh_hash,
                expires_at,
                client_ip: ctx.client_ip.as_deref(),
                user_agent: ctx.user_agent.as_deref(),
            },
        )
        .await?;

    Ok(AuthSession {
        user: user.to_dto(),
        tokens: AuthTokens {
            access_token,
            refresh_token,
            expires_in: (claims.expires_at - Utc::now()).num_seconds().max(0) as u64,
            token_type: "Bearer".into(),
        },
    })
}
