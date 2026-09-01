use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::{locale::Locale, pagination::Page, paseto::Role};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// NATS request-reply subjects exposed by the Auth service.
pub mod subjects {
    pub const REGISTER: &str = "auth.user.register";
    pub const LOGIN: &str = "auth.user.login";
    pub const LOGOUT: &str = "auth.user.logout";
    pub const GET_USER: &str = "auth.user.get";
    pub const UPDATE_USER: &str = "auth.user.update";
    pub const LIST_USERS: &str = "auth.user.list";
    pub const CHANGE_PASSWORD: &str = "auth.user.change_password";

    pub const REFRESH: &str = "auth.token.refresh";

    pub const REQUEST_RESET: &str = "auth.password.request_reset";
    pub const CONFIRM_RESET: &str = "auth.password.confirm_reset";
    pub const CONFIRM_EMAIL: &str = "auth.email.confirm";

    pub const ADDRESS_CREATE: &str = "auth.address.create";
    pub const ADDRESS_LIST: &str = "auth.address.list";
    pub const ADDRESS_GET: &str = "auth.address.get";
    pub const ADDRESS_UPDATE: &str = "auth.address.update";
    pub const ADDRESS_DELETE: &str = "auth.address.delete";
    pub const ADDRESS_SET_DEFAULT: &str = "auth.address.set_default";
}

// ============== user ======

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Locked,
    Disabled,
    Anonymized,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub email_verified: bool,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub locale: Locale,
    pub role: Role,
    pub status: UserStatus,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthTokens {
    /// PASETO v4.public, short-lived.
    pub access_token: String,
    /// Opaque, single-use: rotated on every refresh.
    pub refresh_token: String,
    /// Access token validity duration, in seconds.
    pub expires_in: u64,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub user: UserDto,
    pub tokens: AuthTokens,
}

// ======= requests ====

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(email(message = "must be a valid email address"))]
    #[schema(example = "coach@touchdown.fr")]
    pub email: String,
    /// Minimum 8 characters. Length takes priority over imposed complexity.
    #[validate(length(min = 8, max = 128, message = "must be 8 to 128 characters"))]
    #[schema(example = "correct-horse-battery")]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub first_name: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub last_name: Option<String>,
    #[serde(default)]
    pub locale: Locale,
    #[serde(default)]
    pub marketing_consent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    #[validate(length(min = 20))]
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    #[validate(length(min = 20))]
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    #[validate(length(min = 20))]
    pub token: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1))]
    pub current_password: String,
    #[validate(length(min = 12, max = 128))]
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailRequest {
    #[validate(length(min = 20))]
    pub token: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    #[validate(length(min = 1, max = 100))]
    pub first_name: Option<String>,
    #[validate(length(min = 1, max = 100))]
    pub last_name: Option<String>,
    #[validate(length(min = 5, max = 30))]
    pub phone: Option<String>,
    pub locale: Option<Locale>,
}

/// `user_id` absent = "me". Provided = reading another account,
/// restricted to staff/admin roles; enforcement is done in the handler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetUserRequest {
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersRequest {
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub per_page: Option<u32>,
    pub role: Option<Role>,
    /// Partial search on email.
    pub search: Option<String>,
}

pub type ListUsersResponse = Page<UserDto>;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedResponse {
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OkResponse {
    pub ok: bool,
}

// ======= adresses ======

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AddressKind {
    Shipping,
    Billing,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddressDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: Option<String>,
    pub kind: AddressKind,
    pub first_name: String,
    pub last_name: String,
    pub company: Option<String>,
    pub line1: String,
    pub line2: Option<String>,
    pub postal_code: String,
    pub city: String,
    pub state: Option<String>,
    /// ISO 3166-1 alpha-2, uppercase.
    pub country: String,
    pub phone: Option<String>,
    pub is_default_shipping: bool,
    pub is_default_billing: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateAddressRequest {
    #[validate(length(max = 50))]
    pub label: Option<String>,
    pub kind: AddressKind,
    #[validate(length(min = 1, max = 100))]
    pub first_name: String,
    #[validate(length(min = 1, max = 100))]
    pub last_name: String,
    #[validate(length(max = 150))]
    pub company: Option<String>,
    #[validate(length(min = 1, max = 200))]
    pub line1: String,
    #[validate(length(max = 200))]
    pub line2: Option<String>,
    #[validate(length(min = 1, max = 20))]
    pub postal_code: String,
    #[validate(length(min = 1, max = 100))]
    pub city: String,
    #[validate(length(max = 100))]
    pub state: Option<String>,
    #[validate(length(equal = 2, message = "must be an ISO 3166-1 alpha-2 code"))]
    #[schema(example = "FR")]
    pub country: String,
    #[validate(length(max = 30))]
    pub phone: Option<String>,
    #[serde(default)]
    pub is_default_shipping: bool,
    #[serde(default)]
    pub is_default_billing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressIdRequest {
    pub address_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAddressRequest {
    pub address_id: Uuid,
    #[serde(flatten)]
    pub data: CreateAddressRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetDefaultAddressRequest {
    #[serde(skip_deserializing)]
    pub address_id: Uuid,
    #[serde(default)]
    pub shipping: bool,
    #[serde(default)]
    pub billing: bool,
}

// ================ events ============

/// Payloads for JetStream events emitted by Auth. Consumed by
/// Notification (emails), Analytics, and Cart (merge on login).
pub mod events {
    use super::*;

    pub const USER_REGISTERED: &str = "events.user.registered";
    pub const USER_LOGGED_IN: &str = "events.user.logged_in";
    pub const USER_UPDATED: &str = "events.user.updated";
    pub const USER_EMAIL_VERIFIED: &str = "events.user.email_verified";
    pub const USER_PASSWORD_CHANGED: &str = "events.user.password_changed";
    pub const USER_PASSWORD_RESET_REQUESTED: &str = "events.user.password_reset_requested";
    pub const ADDRESS_CREATED: &str = "events.address.created";
    pub const ADDRESS_UPDATED: &str = "events.address.updated";
    pub const ADDRESS_DELETED: &str = "events.address.deleted";
    pub const CONSENT_GRANTED: &str = "events.consent.granted";

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserRegistered {
        pub user_id: Uuid,
        pub email: String,
        pub first_name: Option<String>,
        pub locale: Locale,
        /// Email verification token, consumed by Notification to build the link. TTL 24 h.
        pub verification_token: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserLoggedIn {
        pub user_id: Uuid,
        pub session_id: Uuid,
        pub client_ip: Option<String>,
        pub user_agent: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PasswordResetRequested {
        pub user_id: Uuid,
        pub email: String,
        pub locale: Locale,
        /// TTL 1 h.
        pub reset_token: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct UserSimpleEvent {
        pub user_id: Uuid,
        pub email: String,
        pub locale: Locale,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AddressChanged {
        pub user_id: Uuid,
        pub address_id: Uuid,
        pub country: String,
    }
}
