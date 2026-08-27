//! Shared test container fixtures.
//!
//! Each fixture keeps its `ContainerAsync` alive within the struct:
//! the container is destroyed when the fixture goes out of scope.
//! Never use`let _ = fixture.node`, or the container will be destroyed in the middle of the test.

use sqlx::migrate::Migrator;
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

use crate::{
    db::PgPool,
    nats::{JetStreamPublisher, NatsRpc, ensure_streams},
};

pub struct PgFixture {
    _node: ContainerAsync<testcontainers_modules::postgres::Postgres>,
    pub pool: PgPool,
    pub url: String,
}

/// Disposable Postgres. If `migrator` is provided, the migrations are applied.
pub async fn postgres(migrator: Option<&Migrator>) -> PgFixture {
    let node = testcontainers_modules::postgres::Postgres::default()
        .start()
        .await
        .expect("cannot start postgres container");
    let host = node.get_host().await.expect("host");
    let port = node.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("cannot to connect to test postgres");

    if let Some(m) = migrator {
        m.run(&pool).await.expect("migrations test failed");
    }

    PgFixture {
        _node: node,
        pool,
        url,
    }
}

pub struct NatsFixture {
    _node: ContainerAsync<GenericImage>,
    pub url: String,
    pub rpc: NatsRpc,
    pub publisher: JetStreamPublisher,
}

/// NATS with JetStream enabled; system streams have already been created.
pub async fn nats() -> NatsFixture {
    let node = GenericImage::new("nats", "2.11-alpine")
        .with_exposed_port(4222.tcp())
        .with_wait_for(WaitFor::message_on_stderr("Server is ready"))
        .with_cmd(vec!["--jetstream", "--store_dir=/data"])
        .start()
        .await
        .expect("cannot start nats container");

    let host = node.get_host().await.expect("host");
    let port = node.get_host_port_ipv4(4222).await.expect("port");
    let url = format!("nats://{host}:{port}");

    let mut cfg = crate::config::AppConfig::from_env("test").expect("test config");
    cfg.nats_url = url.clone();
    let rpc = NatsRpc::connect(&cfg)
        .await
        .expect("cannot connect to test nats");
    let publisher = JetStreamPublisher::new(rpc.client());
    ensure_streams(publisher.context())
        .await
        .expect("cannot create streams");

    NatsFixture {
        _node: node,
        url,
        rpc,
        publisher,
    }
}

pub struct RedisFixture {
    _node: ContainerAsync<testcontainers_modules::redis::Redis>,
    pub url: String,
}

pub async fn redis() -> RedisFixture {
    let node = testcontainers_modules::redis::Redis::default()
        .start()
        .await
        .expect("cannot start redis test container");
    let host = node.get_host().await.expect("host");
    let port = node.get_host_port_ipv4(6379).await.expect("port");

    RedisFixture {
        _node: node,
        url: format!("redis://{host}:{port}"),
    }
}

pub struct MinioFixture {
    _node: ContainerAsync<GenericImage>,
    pub endpoint: String,
    pub access_key: &'static str,
    pub secret_key: &'static str,
}

pub async fn minio() -> MinioFixture {
    let node = GenericImage::new("minio/minio", "latest")
        .with_exposed_port(9000.tcp())
        .with_wait_for(WaitFor::message_on_stdout("API:"))
        .with_cmd(vec!["server", "/data"])
        .with_env_var("MINIO_ROOT_USER", "touchdown")
        .with_env_var("MINIO_ROOT_PASSWORD", "touchdown123")
        .start()
        .await
        .expect("cannot start minio test container");
    let host = node.get_host().await.expect("host");
    let port = node.get_host_port_ipv4(9000).await.expect("port");

    MinioFixture {
        _node: node,
        endpoint: format!("http://{host}:{port}"),
        access_key: "touchdown",
        secret_key: "touchdown123",
    }
}

pub struct MeiliFixture {
    _node: ContainerAsync<GenericImage>,
    pub url: String,
    pub master_key: &'static str,
}

pub async fn meilisearch() -> MeiliFixture {
    let node = GenericImage::new("getmeili/meilisearch", "v1.13")
        .with_exposed_port(7700.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Server listening"))
        .with_env_var("MEILI_MASTER_KEY", "test_master_key")
        .with_env_var("MEILI_ENV", "development")
        .start()
        .await
        .expect("cannot start meilisearch test container");
    let host = node.get_host().await.expect("host");
    let port = node.get_host_port_ipv4(7700).await.expect("port");

    MeiliFixture {
        _node: node,
        url: format!("http://{host}:{port}"),
        master_key: "test_master_key",
    }
}
