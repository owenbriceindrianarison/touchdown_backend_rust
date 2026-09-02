use chrono::{DateTime, Duration, Utc};
use contracts::auth::{UserDto, UserStatus as DtoUserStatus};
use shared::{AppError, locale::Locale, paseto::Role};
use uuid::Uuid;

use super::email::Email;
use crate::password;

/// Opaque password hash. Prevents direct access to the raw value from outside the module.
pub struct PasswordHash(pub(crate) String);

impl PasswordHash {
    pub fn new(raw_hash: String) -> Self {
        Self(raw_hash)
    }

    pub fn verify(&self, password: &str) -> bool {
        password::verify(password, &self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Account status in the domain (distinct from the contracts::auth::UserStatus DTO).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Locked,
    Disabled,
    Anonymized,
}

impl UserStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, UserStatus::Active)
    }

    pub(crate) fn from_db(s: &str) -> Self {
        match s {
            "locked" => UserStatus::Locked,
            "disabled" => UserStatus::Disabled,
            "anonymized" => UserStatus::Anonymized,
            _ => UserStatus::Active,
        }
    }

    fn to_dto(&self) -> DtoUserStatus {
        match self {
            UserStatus::Active => DtoUserStatus::Active,
            UserStatus::Locked => DtoUserStatus::Locked,
            UserStatus::Disabled => DtoUserStatus::Disabled,
            UserStatus::Anonymized => DtoUserStatus::Anonymized,
        }
    }
}

/// User aggregate root.
pub struct User {
    pub id: Uuid,
    pub email: Email,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub(crate) password_hash: PasswordHash,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub locale: Locale,
    pub role: Role,
    pub status: UserStatus,
    pub(crate) failed_attempts: i16,
    pub(crate) locked_until: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_locked(&self) -> bool {
        self.locked_until.is_some_and(|until| until > Utc::now())
    }

    /// Verifies the password and account state.
    /// Returns `Unauthorized` if the password is wrong,
    /// `Forbidden` if the account is locked or inactive.
    pub fn authenticate(&self, password: &str) -> Result<(), AppError> {
        if self.is_locked() {
            return Err(AppError::Forbidden(
                "account temporarily locked after too many failed attempts".into(),
            ));
        }
        if !self.status.is_active() {
            return Err(AppError::Forbidden("account is not active".into()));
        }
        if !self.password_hash.verify(password) {
            return Err(AppError::Unauthorized("invalid credentials".into()));
        }
        Ok(())
    }

    /// Increments the failure counter and locks the account if the threshold is reached.
    pub fn record_failed_attempt(&mut self, max_attempts: i16, lock_for: Duration) {
        self.failed_attempts += 1;
        if self.failed_attempts >= max_attempts {
            self.locked_until = Some(Utc::now() + lock_for);
        }
    }

    pub fn to_dto(&self) -> UserDto {
        UserDto {
            id: self.id,
            email: self.email.as_str().to_owned(),
            email_verified: self.email_verified_at.is_some(),
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            phone: self.phone.clone(),
            locale: self.locale,
            role: self.role,
            status: self.status.to_dto(),
            last_login_at: self.last_login_at,
            created_at: self.created_at,
        }
    }
}
