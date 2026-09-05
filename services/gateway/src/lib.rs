pub mod extract;
pub mod openapi;
pub mod routes {
    pub mod auth;
    pub mod health;
    pub mod me;
}
pub mod state;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub use state::AppState;

use crate::openapi::ApiDoc;

pub fn app(state: AppState) -> Router {
    let cors = build_cors(&state);

    // Everything served here is user-specific or mutating:
    // nothing should land in a shared cache. Cacheable catalogue routes will have their own policy.
    let no_store = SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );

    let public = Router::new()
        .route("/auth/register", post(routes::auth::register))
        .route("/auth/login", post(routes::auth::login))
        .route("/auth/refresh", post(routes::auth::refresh))
        .route("/auth/logout", post(routes::auth::logout))
        .route("/auth/password/forgot", post(routes::auth::forgot_password))
        .route("/auth/password/reset", post(routes::auth::reset_password))
        .route("/auth/email/verify", post(routes::auth::verify_email));

    let account = Router::new()
        .route("/me", get(routes::me::get_me).patch(routes::me::update_me))
        .route("/me/password", post(routes::me::change_password))
        .route(
            "/me/addresses",
            get(routes::me::list_addresses).post(routes::me::create_address),
        )
        .route(
            "/me/addresses/{id}",
            get(routes::me::get_address)
                .put(routes::me::update_address)
                .delete(routes::me::delete_address),
        )
        .route(
            "/me/addresses/{id}/default",
            post(routes::me::set_default_address),
        );

    let admin = Router::new().route("/admin/users", get(routes::me::list_users));

    Router::new()
        .route("/health", get(routes::health::health))
        .route("/readyz", get(routes::health::readyz))
        .merge(public)
        .merge(account)
        .merge(admin)
        .layer(no_store)
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

fn build_cors(state: &AppState) -> CorsLayer {
    let origins: Vec<HeaderValue> = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let allow_headers = [
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT_LANGUAGE,
    ];

    if origins.is_empty() && state.cfg.environment.is_dev() {
        CorsLayer::new()
            .allow_methods(Any)
            .allow_headers(allow_headers)
            .allow_origin(Any)
    } else {
        // `allow_credentials` forbids wildcards: methods and origins must be explicit.
        CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers(allow_headers)
            .allow_origin(AllowOrigin::list(origins))
            .allow_credentials(true)
    }
}
