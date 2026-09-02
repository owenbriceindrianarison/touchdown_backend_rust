use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use contracts::auth::AddressKind;
use shared::{AppError, db::PgPool, ids::new_id};
use uuid::Uuid;

use crate::domain::address::Address;
use crate::domain::email::Email;
use crate::domain::ports::{
    AddressRepository, CreateUserCmd, InsertRefreshTokenCmd, RefreshToken, TokenRepository, Tx,
    UpsertAddressCmd, UserRepository,
};
use crate::domain::user::{PasswordHash, User, UserStatus};

// ===== Internal SQL constants =====

const USER_COLUMNS: &str = "id, email::text AS email, email_verified_at, password_hash, \
     first_name, last_name, phone, locale::text AS locale, role::text AS role, \
     status::text AS status, failed_login_attempts, locked_until, last_login_at, created_at";

const ADDRESS_COLUMNS: &str = "id, user_id, label, kind::text AS kind, first_name, last_name, \
     company, line1, line2, postal_code, city, state, country::text AS country, phone, \
     is_default_shipping, is_default_billing, created_at, updated_at";

// ===== Internal helpers =====

fn address_kind_from_str(s: &str) -> AddressKind {
    match s {
        "shipping" => AddressKind::Shipping,
        "billing" => AddressKind::Billing,
        _ => AddressKind::Both,
    }
}

fn address_kind_to_str(k: AddressKind) -> &'static str {
    match k {
        AddressKind::Shipping => "shipping",
        AddressKind::Billing => "billing",
        AddressKind::Both => "both",
    }
}

// ===== Internal row structs =====

#[derive(Debug, sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    email_verified_at: Option<DateTime<Utc>>,
    password_hash: String,
    first_name: Option<String>,
    last_name: Option<String>,
    phone: Option<String>,
    locale: String,
    role: String,
    status: String,
    failed_login_attempts: i16,
    locked_until: Option<DateTime<Utc>>,
    last_login_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl UserRow {
    fn into_domain(self) -> Result<User, AppError> {
        let email = Email::parse(&self.email)?;
        let locale = self.locale.parse().unwrap_or_default();
        let role =
            shared::paseto::Role::parse(&self.role).unwrap_or(shared::paseto::Role::Customer);
        let status = UserStatus::from_db(&self.status);
        Ok(User {
            id: self.id,
            email,
            email_verified_at: self.email_verified_at,
            password_hash: PasswordHash::new(self.password_hash),
            first_name: self.first_name,
            last_name: self.last_name,
            phone: self.phone,
            locale,
            role,
            status,
            failed_attempts: self.failed_login_attempts,
            locked_until: self.locked_until,
            last_login_at: self.last_login_at,
            created_at: self.created_at,
        })
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AddressRow {
    id: Uuid,
    user_id: Uuid,
    label: Option<String>,
    kind: String,
    first_name: String,
    last_name: String,
    company: Option<String>,
    line1: String,
    line2: Option<String>,
    postal_code: String,
    city: String,
    state: Option<String>,
    country: String,
    phone: Option<String>,
    is_default_shipping: bool,
    is_default_billing: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AddressRow {
    fn into_domain(self) -> Address {
        Address {
            id: self.id,
            user_id: self.user_id,
            label: self.label,
            kind: address_kind_from_str(&self.kind),
            first_name: self.first_name,
            last_name: self.last_name,
            company: self.company,
            line1: self.line1,
            line2: self.line2,
            postal_code: self.postal_code,
            city: self.city,
            state: self.state,
            country: self.country,
            phone: self.phone,
            is_default_shipping: self.is_default_shipping,
            is_default_billing: self.is_default_billing,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RefreshTokenRow {
    id: Uuid,
    user_id: Uuid,
    family_id: Uuid,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl RefreshTokenRow {
    fn into_domain(self) -> RefreshToken {
        RefreshToken {
            id: self.id,
            user_id: self.user_id,
            family_id: self.family_id,
            expires_at: self.expires_at,
            revoked_at: self.revoked_at,
        }
    }
}

// ===== Concrete repository =====

#[derive(Clone)]
pub struct PgRepository {
    pool: PgPool,
}

impl PgRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ===== UserRepository impl =====

#[async_trait]
impl UserRepository for PgRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let sql = format!(
            "SELECT {USER_COLUMNS} FROM users WHERE email = $1::citext AND deleted_at IS NULL"
        );
        let row = sqlx::query_as::<_, UserRow>(sqlx::AssertSqlSafe(sql))
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1 AND deleted_at IS NULL");
        let row = sqlx::query_as::<_, UserRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn create(&self, tx: &mut Tx<'_>, cmd: CreateUserCmd<'_>) -> Result<User, AppError> {
        let sql = format!(
            r#"
            INSERT INTO users (id, email, password_hash, first_name, last_name, locale, tos_accepted_at)
            VALUES ($1, $2::citext, $3, $4, $5, $6::locale_code, now())
            RETURNING {USER_COLUMNS}
            "#
        );
        let row = sqlx::query_as::<_, UserRow>(sqlx::AssertSqlSafe(sql))
            .bind(new_id())
            .bind(cmd.email)
            .bind(cmd.password_hash.as_str())
            .bind(cmd.first_name)
            .bind(cmd.last_name)
            .bind(cmd.locale)
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| match AppError::from(e) {
                AppError::Conflict(_) => AppError::Conflict("email already registered".into()),
                other => other,
            })?;
        row.into_domain()
    }

    async fn update_profile(
        &self,
        user_id: Uuid,
        first_name: Option<&str>,
        last_name: Option<&str>,
        phone: Option<&str>,
        locale: Option<&str>,
    ) -> Result<User, AppError> {
        let sql = format!(
            r#"
            UPDATE users
               SET first_name = COALESCE($2, first_name),
                   last_name  = COALESCE($3, last_name),
                   phone      = COALESCE($4, phone),
                   locale     = COALESCE($5::locale_code, locale),
                   version    = version + 1
             WHERE id = $1 AND deleted_at IS NULL
            RETURNING {USER_COLUMNS}
            "#
        );
        let row = sqlx::query_as::<_, UserRow>(sqlx::AssertSqlSafe(sql))
            .bind(user_id)
            .bind(first_name)
            .bind(last_name)
            .bind(phone)
            .bind(locale)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("user", user_id))?;
        row.into_domain()
    }

    async fn set_password(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        hash: &PasswordHash,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET password_hash = $2, failed_login_attempts = 0, locked_until = NULL, \
             version = version + 1 WHERE id = $1",
        )
        .bind(user_id)
        .bind(hash.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn record_login_success(&self, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, last_login_at = now() \
             WHERE id = $1",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn record_login_failure(
        &self,
        user_id: Uuid,
        max_attempts: i16,
        lock_duration: Duration,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE users
               SET failed_login_attempts = failed_login_attempts + 1,
                   locked_until = CASE
                       WHEN failed_login_attempts + 1 >= $2 THEN now() + $3::interval
                       ELSE locked_until
                   END
             WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(max_attempts)
        .bind(format!("{} seconds", lock_duration.num_seconds()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list(
        &self,
        role: Option<&str>,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<User>, i64), AppError> {
        let pattern = search.map(|s| format!("%{s}%"));
        let sql = format!(
            r#"
            SELECT {USER_COLUMNS} FROM users
             WHERE deleted_at IS NULL
               AND ($1::text IS NULL OR role::text = $1)
               AND ($2::text IS NULL OR email::text ILIKE $2)
             ORDER BY created_at DESC
             LIMIT $3 OFFSET $4
            "#
        );
        let rows = sqlx::query_as::<_, UserRow>(sqlx::AssertSqlSafe(sql))
            .bind(role)
            .bind(pattern.as_deref())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*) FROM users
             WHERE deleted_at IS NULL
               AND ($1::text IS NULL OR role::text = $1)
               AND ($2::text IS NULL OR email::text ILIKE $2)
            "#,
        )
        .bind(role)
        .bind(pattern.as_deref())
        .fetch_one(&self.pool)
        .await?;

        let users = rows
            .into_iter()
            .map(UserRow::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((users, total))
    }
}

// ===== TokenRepository impl =====

#[async_trait]
impl TokenRepository for PgRepository {
    async fn insert_refresh_token(
        &self,
        tx: &mut Tx<'_>,
        cmd: InsertRefreshTokenCmd<'_>,
    ) -> Result<Uuid, AppError> {
        let id = new_id();
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens
                (id, user_id, family_id, parent_id, token_hash, expires_at, user_agent, ip)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet)
            "#,
        )
        .bind(id)
        .bind(cmd.user_id)
        .bind(cmd.family_id)
        .bind(cmd.parent_id)
        .bind(cmd.token_hash)
        .bind(cmd.expires_at)
        .bind(cmd.user_agent)
        .bind(cmd.client_ip)
        .execute(&mut **tx)
        .await?;
        Ok(id)
    }

    async fn find_refresh_token(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<RefreshToken>, AppError> {
        let row = sqlx::query_as::<_, RefreshTokenRow>(
            "SELECT id, user_id, family_id, expires_at, revoked_at FROM refresh_tokens \
             WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(RefreshTokenRow::into_domain))
    }

    async fn revoke_token(&self, tx: &mut Tx<'_>, id: Uuid, reason: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now(), revoked_reason = $2 \
             WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn revoke_family(&self, family_id: Uuid, reason: &str) -> Result<u64, AppError> {
        let res = sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now(), revoked_reason = $2 \
             WHERE family_id = $1 AND revoked_at IS NULL",
        )
        .bind(family_id)
        .bind(reason)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    async fn revoke_all_user_tokens(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        reason: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now(), revoked_reason = $2 \
             WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn insert_single_use_token(
        &self,
        tx: &mut Tx<'_>,
        table: &str,
        user_id: Uuid,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
        email: Option<&str>,
    ) -> Result<(), AppError> {
        let sql = match table {
            "password_reset_tokens" => {
                "INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at) \
                 VALUES ($1, $2, $3, $4)"
            }
            "email_verification_tokens" => {
                "INSERT INTO email_verification_tokens (id, user_id, email, token_hash, expires_at) \
                 VALUES ($1, $2, $5::citext, $3, $4)"
            }
            _ => return Err(AppError::internal(anyhow::anyhow!("unknown token table"))),
        };
        sqlx::query(sql)
            .bind(new_id())
            .bind(user_id)
            .bind(token_hash)
            .bind(expires_at)
            .bind(email)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    async fn consume_single_use_token(
        &self,
        tx: &mut Tx<'_>,
        table: &str,
        token_hash: &[u8],
    ) -> Result<Uuid, AppError> {
        let sql = format!(
            "UPDATE {table} SET used_at = now() \
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now() \
             RETURNING user_id"
        );
        sqlx::query_scalar::<_, Uuid>(sqlx::AssertSqlSafe(sql))
            .bind(token_hash)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| AppError::BadRequest("invalid or expired token".into()))
    }

    async fn mark_email_verified(&self, tx: &mut Tx<'_>, user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET email_verified_at = now() WHERE id = $1 AND email_verified_at IS NULL",
        )
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn record_marketing_consent(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        client_ip: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO user_consents (id, user_id, purpose, granted, source, ip) \
             VALUES ($1, $2, 'marketing_email', true, 'signup', $3::inet)",
        )
        .bind(new_id())
        .bind(user_id)
        .bind(client_ip)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

// ===== AddressRepository impl =====

#[async_trait]
impl AddressRepository for PgRepository {
    async fn list(&self, user_id: Uuid) -> Result<Vec<Address>, AppError> {
        let sql = format!(
            "SELECT {ADDRESS_COLUMNS} FROM addresses WHERE user_id = $1 AND deleted_at IS NULL \
             ORDER BY is_default_shipping DESC, created_at DESC"
        );
        let rows = sqlx::query_as::<_, AddressRow>(sqlx::AssertSqlSafe(sql))
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(AddressRow::into_domain).collect())
    }

    async fn get(&self, user_id: Uuid, address_id: Uuid) -> Result<Address, AppError> {
        let sql = format!(
            "SELECT {ADDRESS_COLUMNS} FROM addresses \
             WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL"
        );
        let row = sqlx::query_as::<_, AddressRow>(sqlx::AssertSqlSafe(sql))
            .bind(address_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found("address", address_id))?;
        Ok(row.into_domain())
    }

    async fn clear_defaults(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        shipping: bool,
        billing: bool,
        except: Option<Uuid>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE addresses
               SET is_default_shipping = CASE WHEN $2 THEN false ELSE is_default_shipping END,
                   is_default_billing  = CASE WHEN $3 THEN false ELSE is_default_billing  END
             WHERE user_id = $1 AND deleted_at IS NULL AND ($4::uuid IS NULL OR id <> $4)
            "#,
        )
        .bind(user_id)
        .bind(shipping)
        .bind(billing)
        .bind(except)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    async fn create(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        cmd: UpsertAddressCmd<'_>,
    ) -> Result<Address, AppError> {
        let sql = format!(
            r#"
            INSERT INTO addresses
                (id, user_id, label, kind, first_name, last_name, company, line1, line2,
                 postal_code, city, state, country, phone, is_default_shipping, is_default_billing)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13::country_code, $14, $15, $16)
            RETURNING {ADDRESS_COLUMNS}
            "#
        );
        let row = sqlx::query_as::<_, AddressRow>(sqlx::AssertSqlSafe(sql))
            .bind(new_id())
            .bind(user_id)
            .bind(cmd.label)
            .bind(address_kind_to_str(cmd.kind))
            .bind(cmd.first_name)
            .bind(cmd.last_name)
            .bind(cmd.company)
            .bind(cmd.line1)
            .bind(cmd.line2)
            .bind(cmd.postal_code)
            .bind(cmd.city)
            .bind(cmd.state)
            .bind(cmd.country)
            .bind(cmd.phone)
            .bind(cmd.is_default_shipping)
            .bind(cmd.is_default_billing)
            .fetch_one(&mut **tx)
            .await?;
        Ok(row.into_domain())
    }

    async fn update(
        &self,
        tx: &mut Tx<'_>,
        address_id: Uuid,
        user_id: Uuid,
        cmd: UpsertAddressCmd<'_>,
    ) -> Result<Address, AppError> {
        let sql = format!(
            r#"
            UPDATE addresses SET
                label = $3, kind = $4, first_name = $5, last_name = $6, company = $7,
                line1 = $8, line2 = $9, postal_code = $10, city = $11, state = $12,
                country = $13::country_code, phone = $14,
                is_default_shipping = $15, is_default_billing = $16
             WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL
            RETURNING {ADDRESS_COLUMNS}
            "#
        );
        let row = sqlx::query_as::<_, AddressRow>(sqlx::AssertSqlSafe(sql))
            .bind(address_id)
            .bind(user_id)
            .bind(cmd.label)
            .bind(address_kind_to_str(cmd.kind))
            .bind(cmd.first_name)
            .bind(cmd.last_name)
            .bind(cmd.company)
            .bind(cmd.line1)
            .bind(cmd.line2)
            .bind(cmd.postal_code)
            .bind(cmd.city)
            .bind(cmd.state)
            .bind(cmd.country)
            .bind(cmd.phone)
            .bind(cmd.is_default_shipping)
            .bind(cmd.is_default_billing)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| AppError::not_found("address", address_id))?;
        Ok(row.into_domain())
    }

    async fn set_default(
        &self,
        tx: &mut Tx<'_>,
        address_id: Uuid,
        user_id: Uuid,
        shipping: bool,
        billing: bool,
    ) -> Result<Address, AppError> {
        let sql = format!(
            r#"
            UPDATE addresses
               SET is_default_shipping = CASE WHEN $3 THEN true ELSE is_default_shipping END,
                   is_default_billing  = CASE WHEN $4 THEN true ELSE is_default_billing  END
             WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL
            RETURNING {ADDRESS_COLUMNS}
            "#
        );
        let row = sqlx::query_as::<_, AddressRow>(sqlx::AssertSqlSafe(sql))
            .bind(address_id)
            .bind(user_id)
            .bind(shipping)
            .bind(billing)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| AppError::not_found("address", address_id))?;
        Ok(row.into_domain())
    }

    async fn delete(
        &self,
        tx: &mut Tx<'_>,
        user_id: Uuid,
        address_id: Uuid,
    ) -> Result<u64, AppError> {
        let res = sqlx::query(
            "UPDATE addresses SET deleted_at = now(), is_default_shipping = false, \
             is_default_billing = false WHERE id = $1 AND user_id = $2 AND deleted_at IS NULL",
        )
        .bind(address_id)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;
        Ok(res.rows_affected())
    }
}
