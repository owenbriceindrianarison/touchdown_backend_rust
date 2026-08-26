use utoipa::openapi::{
    ContactBuilder, InfoBuilder, OpenApi, OpenApiBuilder, ServerBuilder,
    security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

/// Common OpenAPI skeleton. Each service generates its own fragment;
/// the Gateway merges them into a single specification
///  that will be used to generate TypeScript types for Next.js and Angular.
pub fn base(title: &str, description: &str) -> OpenApi {
    OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title(title)
                .version(env!("CARGO_PKG_VERSION"))
                .description(Some(description))
                .contact(Some(ContactBuilder::new().name(Some("Touchdown")).build()))
                .build(),
        )
        .servers(Some(vec![
            ServerBuilder::new()
                .url("http://localhost:8080")
                .description(Some("Local Development"))
                .build(),
        ]))
        .build()
}

/// PASETO security scheme (`Authorization: Bearer v4.public....`).
pub fn paseto_security_scheme() -> SecurityScheme {
    SecurityScheme::Http(
        HttpBuilder::new()
            .scheme(HttpAuthScheme::Bearer)
            .bearer_format("PASETO")
            .description(Some("PASETO v4.public access token"))
            .build(),
    )
}
