//! Interface contracts between the Gateway and the microservices.
//!
//! Contains ONLY serializable DTOs and NATS topic constants.
//! No logic, no database access, no runtime dependencies.
//! This is what allows the Gateway and the service to be typed against the same contract without sharing any business logic.
pub mod auth;
pub mod validation;
