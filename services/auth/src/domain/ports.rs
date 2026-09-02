use async_trait::async_trait;
use chrono::{DateTime, Duration};
use contracts::auth::AddressKind;
use shared::AppError;
use uuid::Uuid;

use super::address::Address;
use super::user::{PasswordHash, User};

pub type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

/// Command to create a user.
pub struct CreateUserCmd<'a> {
    pub email: &'a str,
    pub password_hash: &'a PasswordHash,
    pub first_name: Option<&'a str>,
    pub last_name: Option<&'a str>,
    pub locale: &'a str,
}

pub struct InsertRefreshTokenCmd<'a> {
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub token_hash: &'a [u8],
    pub expires_at: DateTime<chrono::Utc>,
    pub client_ip: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}

pub struct UpsertAddressCmd<'a> {
    pub label: Option<&'a str>,
    pub kind: AddressKind,
    pub first_name: &'a str,
    pub last_name: &'a str,
    pub company: Option<&'a str>,
    pub line1: &'a str,
    pub line2: Option<&'a str>,
    pub postal_code: &'a str,
    pub city: &'a str,
    pub state: Option<&'a str>,
    pub country: &'a str,
    pub phone: Option<&'a str>,
    pub is_default_shipping: bool,
    pub is_default_billing: bool,
}

/// Domain refresh token (without the hash, which is kept confidential).
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub family_id: Uuid,
    pub expires_at: DateTime<chrono::Utc>,
    pub revoked_at: Option<DateTime<chrono::Utc>>,
}

#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;
    async fn create(&self, tx: &mut Tx<'_>, cmd: CreateUserCmd<'_>) -> Result<User, AppError>;
    async fn update_profile(
        &self,
        user_id: Uuid,
        first_name: Option<&str>,
        last_name: Option<&str>,
        phone: Option<&str>,
        locale: Option<&str>,
    ) -> Result<User, AppError>;
    async fn set_password(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        hash: &PasswordHash,
    ) -> Result<(), AppError>;
    async fn record_login_success(&self, user_id: Uuid) -> Result<(), AppError>;
    async fn record_login_failure(
        &self,
        user_id: Uuid,
        max_attempts: i16,
        lock_duration: Duration,
    ) -> Result<(), AppError>;
    async fn list(
        &self,
        role: Option<&str>,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<User>, i64), AppError>;
}

#[async_trait]
pub trait TokenRepository: Send + Sync + 'static {
    async fn insert_refresh_token(
        &self,
        tx: &mut Tx<'_>,
        cmd: InsertRefreshTokenCmd<'_>,
    ) -> Result<Uuid, AppError>;
    async fn find_refresh_token(&self, token_hash: &[u8])
    -> Result<Option<RefreshToken>, AppError>;
    async fn revoke_token(&self, tx: &mut Tx<'_>, id: Uuid, reason: &str) -> Result<(), AppError>;
    async fn revoke_family(&self, family_id: Uuid, reason: &str) -> Result<u64, AppError>;
    async fn revoke_all_user_tokens(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError>;
    async fn insert_single_use_token(
        &self,
        tx: &mut Tx<'_>,
        table: &str,
        user_id: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<chrono::Utc>,
        email: Option<&str>,
    ) -> Result<(), AppError>;
    async fn consume_single_use_token(
        &self,
        tx: &mut Tx<'_>,
        table: &str,
        token_hash: &[u8],
    ) -> Result<Uuid, AppError>;
    async fn mark_email_verified(&self, tx: &mut Tx<'_>, user_id: Uuid) -> Result<(), AppError>;
    async fn record_marketing_consent(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        client_ip: Option<&str>,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait AddressRepository: Send + Sync + 'static {
    async fn list(&self, user_id: Uuid) -> Result<Vec<Address>, AppError>;
    async fn get(&self, user_id: Uuid, address_id: Uuid) -> Result<Address, AppError>;
    async fn clear_defaults(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        shipping: bool,
        billing: bool,
        except: Option<Uuid>,
    ) -> Result<(), AppError>;
    async fn create(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        cmd: UpsertAddressCmd<'_>,
    ) -> Result<Address, AppError>;
    async fn update(
        &self,
        tx: &mut Tx<'_>,
        address_id: Uuid,
        user_id: Uuid,
        cmd: UpsertAddressCmd<'_>,
    ) -> Result<Address, AppError>;
    async fn set_default(
        &self,
        tx: &mut Tx<'_>,
        address_id: Uuid,
        user_id: Uuid,
        shipping: bool,
        billing: bool,
    ) -> Result<Address, AppError>;
    async fn delete(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        address_id: Uuid,
    ) -> Result<u64, AppError>;
}

/// Supertrait aggregating the three repository ports, allowing `AuthState` to hold an
/// `Arc<dyn AuthRepository>` without a generic parameter.
pub trait AuthRepository: UserRepository + TokenRepository + AddressRepository {}

impl<T: UserRepository + TokenRepository + AddressRepository> AuthRepository for T {}
