use std::{env, net::SocketAddr, str::FromStr, time::Duration};

use crate::{AppError, locale::Locale};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Staging,
    Production,
}

impl Environment {
    pub fn is_dev(&self) -> bool {
        matches!(self, Environment::Development | Environment::Test)
    }
}

impl FromStr for Environment {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "development" | "dev" | "local" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            other => Err(AppError::Config(format!("unknown ENVIRONMENT `{other}`"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

#[derive(Debug, Clone)]
pub struct PasetoConfig {
    /// ed25519 secret key (64 hex bytes). Not present on services that
    /// only perform verification: only Auth issues tokens.
    pub secret_key: Option<Vec<u8>>,
    pub public_key: Vec<u8>,
    pub issuer: String,
    pub audience: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub service_name: String,
    pub environment: Environment,
    pub http_addr: SocketAddr,
    pub database_url: Option<String>,
    pub db_max_connections: u32,
    pub db_acquire_timeout: Duration,
    pub nats_url: String,
    pub nats_request_timeout: Duration,
    pub rpc_concurrency: usize,
    pub redis_url: Option<String>,
    pub log_level: String,
    pub log_format: LogFormat,
    pub otlp_endpoint: Option<String>,
    pub default_locale: Locale,
    pub paseto: PasetoConfig,
}

impl AppConfig {
    /// Loads the configuration from the environment.
    ///
    /// `DATABASE_URL` is resolved in two steps: `<SERVICE>_DATABASE_URL` first, then `DATABASE_URL`.
    ///  This allows all services to run from a single `.env` file locally without conflicts.
    pub fn from_env(service_name: &str) -> Result<Self, AppError> {
        let _ = dotenvy_load();
        let upper = service_name.to_ascii_uppercase().replace('-', "_");

        let paseto = PasetoConfig {
            secret_key: opt(&format!("{upper}_PASETO_SECRET_KEY"))
                .or_else(|| opt("PASETO_SECRET_KEY"))
                .filter(|s| !s.is_empty())
                .map(|s| decode_hex("PASETO_SECRET_KEY", &s))
                .transpose()?,

            public_key: decode_hex(
                "PASETO_PUBLIC_KEY",
                &opt("PASETO_PUBLIC_KEY").unwrap_or_default(),
            )
            .unwrap_or_default(),

            issuer: opt("PASETO_ISSUER").unwrap_or_else(|| "touchdown".into()),

            audience: opt("PASETO_AUDIENCE").unwrap_or_else(|| "touchdown-api".into()),

            access_ttl: Duration::from_secs(num("PASETO_ACCESS_TTL_SECS", 900)?),

            refresh_ttl: Duration::from_secs(num("PASETO_REFRESH_TTL_SECS", 2_592_000)?),
        };

        Ok(Self {
            service_name: service_name.to_string(),

            environment: opt("ENVIRONMENT")
                .unwrap_or_else(|| "development".into())
                .parse()?,

            http_addr: opt("HTTP_ADDR")
                .unwrap_or_else(|| "0.0.0.0:8080".into())
                .parse()
                .map_err(|e| AppError::Config(format!("invalid HTTP_ADDR: {e}")))?,

            database_url: opt(&format!("{upper}_DATABASE_URL")).or_else(|| opt("DATABASE_URL")),

            db_max_connections: num("DB_MAX_CONNECTIONS", 10)? as u32,

            db_acquire_timeout: Duration::from_millis(num("DB_ACQUIRE_TIMEOUT_MS", 5_000)?),

            nats_url: opt("NATS_URL").unwrap_or_else(|| "nats://localhost:4222".into()),

            nats_request_timeout: Duration::from_millis(num("NATS_REQUEST_TIMEOUT_MS", 5_000)?),

            rpc_concurrency: num("RPC_CONCURRENCY", 256)? as usize,

            redis_url: opt("REDIS_URL"),

            log_level: opt("LOG_LEVEL").unwrap_or_else(|| "info,sqlx=warn".into()),

            log_format: match opt("LOG_FORMAT").as_deref() {
                Some("json") => LogFormat::Json,
                _ => LogFormat::Pretty,
            },

            otlp_endpoint: opt("OTLP_ENDPOINT").filter(|s| !s.is_empty()),

            default_locale: opt("DEFAULT_LOCALE")
                .and_then(|s| s.parse().ok())
                .unwrap_or(Locale::En),

            paseto,
        })
    }

    pub fn database_url(&self) -> Result<&str, AppError> {
        self.database_url
            .as_deref()
            .ok_or_else(|| AppError::Config("DATABASE_URL is required".into()))
    }
}

fn opt(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn num(key: &str, default: u64) -> Result<u64, AppError> {
    match opt(key) {
        None => Ok(default),
        Some(v) => v
            .parse()
            .map_err(|_| AppError::Config(format!("`{key}` must be a number, got {v}"))),
    }
}

fn decode_hex(key: &str, value: &str) -> Result<Vec<u8>, AppError> {
    hex::decode(value).map_err(|e| AppError::Config(format!("`{key}` is not valid hex: {e}")))
}

fn dotenvy_load() -> bool {
    std::path::Path::new(".env").exists()
}
