use sqlx::{
    Pool, Postgres,
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use crate::{AppError, config::AppConfig};

pub type PgPool = Pool<Postgres>;

pub async fn connect(cfg: &AppConfig) -> Result<PgPool, AppError> {
    let url = cfg.database_url()?;
    let options: PgConnectOptions = url
        .parse()
        .map_err(|e| AppError::Config(format!("invalid DATABASE_URL: {e}")))?;

    let pool = PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .acquire_timeout(cfg.db_acquire_timeout)
        // Recycle connections: avoids keeping sessions open
        // whose server state has changed (search_path, prepared statements).
        .max_lifetime(Some(std::time::Duration::from_secs(30 * 60)))
        .idle_timeout(Some(std::time::Duration::from_secs(10 * 60)))
        .test_before_acquire(true)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Config(format!("cannot connect to Postgres: {e}")))?;

    tracing::info!(service = %&cfg.service_name, "postgres poll ready");
    Ok(pool)
}

/// Applies the service's migrations. Each service runs its own
/// `sqlx::migrate!(“./migrations”)` — `shared` does not have a schema.
pub async fn run_migrations(pool: &PgPool, migrator: &Migrator) -> Result<(), AppError> {
    migrator
        .run(pool)
        .await
        .map_err(|e| AppError::Config(format!("migration failed: {e}")))?;
    tracing::info!("migrations applied");
    Ok(())
}
