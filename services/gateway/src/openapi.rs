use contracts::auth::{
    AcceptedResponse, AddressDto, AddressKind, AuthSession, AuthTokens, ChangePasswordRequest,
    CreateAddressRequest, ForgotPasswordRequest, ListUsersRequest, LoginRequest, LogoutRequest,
    OkResponse, RefreshRequest, RegisterRequest, ResetPasswordRequest, SetDefaultAddressRequest,
    UpdateProfileRequest, UserDto, UserStatus, VerifyEmailRequest,
};
use shared::{
    error::ErrorBody,
    health::{HealthCheck, HealthReport, HealthStatus},
    locale::Locale,
    paseto::Role,
};
use utoipa::{Modify, OpenApi};

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme("paseto", shared::openapi::paseto_security_scheme());
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health,
        crate::routes::health::readyz,
        crate::routes::auth::register,
        crate::routes::auth::login,
        crate::routes::auth::refresh,
        crate::routes::auth::logout,
        crate::routes::auth::forgot_password,
        crate::routes::auth::reset_password,
        crate::routes::auth::verify_email,
        crate::routes::me::get_me,
        crate::routes::me::update_me,
        crate::routes::me::change_password,
        crate::routes::me::list_addresses,
        crate::routes::me::create_address,
        crate::routes::me::get_address,
        crate::routes::me::update_address,
        crate::routes::me::set_default_address,
        crate::routes::me::delete_address,
        crate::routes::me::list_users,
    ),
    components(schemas(
        HealthReport, HealthCheck, HealthStatus, ErrorBody, Locale, Role,
        RegisterRequest, LoginRequest, RefreshRequest, LogoutRequest,
        ForgotPasswordRequest, ResetPasswordRequest, VerifyEmailRequest,
        ChangePasswordRequest, UpdateProfileRequest, ListUsersRequest,
        AuthSession, AuthTokens, UserDto, UserStatus,
        AddressDto, AddressKind, CreateAddressRequest, SetDefaultAddressRequest,
        OkResponse, AcceptedResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "Platform health and availability"),
        (name = "auth", description = "Registration, login, tokens"),
        (name = "account", description = "Profile and address book"),
        (name = "admin", description = "Administration (admin role required)"),
    ),
    info(
        title = "Touchdown API",
        description = "Touchdown e-commerce API — American football equipment."
    )
)]
pub struct ApiDoc;
