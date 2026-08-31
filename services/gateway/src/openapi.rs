use shared::{
    error::ErrorBody,
    health::{HealthCheck, HealthReport, HealthStatus},
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(crate::routes::health::health, crate::routes::health::readyz),
    components(schemas(HealthReport, HealthCheck, HealthStatus, ErrorBody)),
    tags((name = "system", description = "Platform health and availability")),
    info(
        title = "Touchdown API",
        description = "Touchdown E-commerce API — American football equipment."
    )
)]
pub struct ApiDoc;
