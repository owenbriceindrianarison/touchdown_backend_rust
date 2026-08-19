use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use pasetors::{
    Public,
    claims::{Claims, ClaimsValidationRules},
    keys::{AsymmetricKeyPair, AsymmetricPublicKey, AsymmetricSecretKey, Generate},
    public,
    token::UntrustedToken,
    version4::V4,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppError, config::PasetoConfig, ids::new_id, locale::Locale};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Customer,
    Staff,
    Admin,
}

impl Role {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "customer" => Some(Role::Customer),
            "staff" => Some(Role::Staff),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Customer => "customer",
            Role::Staff => "staff",
            Role::Admin => "admin",
        }
    }

    /// Staff inherits the customer and admin permissions for everything.
    /// This avoids having to list the roles for each access control check
    pub fn satisfies(&self, required: Role) -> bool {
        matches!(
            (self, required),
            (Role::Admin, _)
                | (Role::Staff, Role::Staff | Role::Customer)
                | (Role::Customer, Role::Customer)
        )
    }
}

#[derive(Debug, Clone)]
pub struct AccessClaims {
    pub user_id: Uuid,
    pub role: Role,
    pub locale: Locale,
    /// Token ID, used for targeted revocation on the Redis side.
    pub jti: Uuid,
    pub session_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct TokenIssuer {
    keypair_secret: AsymmetricSecretKey<V4>,
    issuer: String,
    audience: String,
    access_ttl: Duration,
}

#[derive(Clone)]
pub struct TokenVerifier {
    public_key: AsymmetricPublicKey<V4>,
    issuer: String,
    audience: String,
}

impl TokenIssuer {
    pub fn from_config(cfg: &PasetoConfig) -> Result<Self, AppError> {
        let secret = cfg.secret_key.as_ref().ok_or_else(|| {
            AppError::Config("PASETO SECRET KEY is required to issue tokens".into())
        })?;

        let keypair_secret = AsymmetricSecretKey::<V4>::from(secret)
            .map_err(|e| AppError::Config(format!("invalid PASETO secret key: {e}")))?;

        Ok(Self {
            keypair_secret,
            issuer: cfg.issuer.clone(),
            audience: cfg.audience.clone(),
            access_ttl: cfg.access_ttl,
        })
    }

    pub fn issue_access(
        &self,
        user_id: Uuid,
        role: Role,
        locale: Locale,
        session_id: Uuid,
    ) -> Result<(String, AccessClaims), AppError> {
        let jti = new_id();
        let expires_at =
            Utc::now() + chrono::Duration::from_std(self.access_ttl).unwrap_or_default();

        let mut claims = Claims::new_expires_in(&self.access_ttl)
            .map_err(|e| AppError::internal(anyhow::anyhow!("claims: {e}")))?;
        claims
            .subject(&user_id.to_string())
            .and_then(|_| claims.issuer(&self.issuer))
            .and_then(|_| claims.audience(&self.audience))
            .and_then(|_| claims.token_identifier(&jti.to_string()))
            .and_then(|_| claims.add_additional("role", role.as_str()))
            .and_then(|_| claims.add_additional("locale", locale.as_str()))
            .and_then(|_| claims.add_additional("sid", session_id.to_string()))
            .map_err(|e| AppError::internal(anyhow::anyhow!("claims: {e}")))?;

        let token = public::sign(&self.keypair_secret, &claims, None, None)
            .map_err(|e| AppError::internal(anyhow::anyhow!("paseto sign: {e}")))?;

        Ok((
            token,
            AccessClaims {
                user_id,
                role,
                locale,
                jti,
                session_id,
                expires_at,
            },
        ))
    }
}

impl TokenVerifier {
    pub fn from_config(cfg: &PasetoConfig) -> Result<Self, AppError> {
        let public_key = AsymmetricPublicKey::<V4>::from(&cfg.public_key)
            .map_err(|e| AppError::Config(format!("invalid PASETO public key: {e}")))?;

        Ok(Self {
            public_key,
            issuer: cfg.issuer.clone(),
            audience: cfg.audience.clone(),
        })
    }

    pub fn verify(&self, token: &str) -> Result<AccessClaims, AppError> {
        let untrusted = UntrustedToken::<Public, V4>::try_from(token)
            .map_err(|_| AppError::Unauthorized("malformed token".into()))?;

        let mut rules = ClaimsValidationRules::new();
        rules.validate_issuer_with(&self.issuer);
        rules.validate_audience_with(&self.audience);

        let trusted = public::verify(&self.public_key, &untrusted, &rules, None, None)
            .map_err(|_| AppError::Unauthorized("invalid or expired token".into()))?;

        let claims = trusted
            .payload_claims()
            .ok_or_else(|| AppError::Unauthorized("token has no claims".into()))?;

        let get = |k: &str| {
            claims
                .get_claim(k)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };

        Ok(AccessClaims {
            user_id: get("sub")
                .parse()
                .map_err(|_| AppError::Unauthorized("invalid subject".into()))?,
            role: Role::parse(&get("role"))
                .ok_or_else(|| AppError::Unauthorized("invalid role".into()))?,
            locale: get("locale").parse().unwrap_or_default(),
            jti: get("jti").parse().unwrap_or_else(|_| Uuid::nil()),
            session_id: get("sid").parse().unwrap_or_else(|_| Uuid::nil()),
            expires_at: DateTime::parse_from_rfc3339(&get("exp"))
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}

/// Generates an ed25519 key pair for development.
pub fn generate_keypair() -> Result<(String, String), AppError> {
    let kp = AsymmetricKeyPair::<V4>::generate()
        .map_err(|e| AppError::internal(anyhow::anyhow!("keygen: {e}")))?;
    Ok((
        hex::encode(kp.secret.as_bytes()),
        hex::encode(kp.public.as_bytes()),
    ))
}

/// Utility: time remaining until expiration, for setting a Redis TTL.
pub fn remaining(expires_at: DateTime<Utc>) -> Duration {
    let now: DateTime<Utc> = SystemTime::now().into();
    (expires_at - now).to_std().unwrap_or(Duration::ZERO)
}
