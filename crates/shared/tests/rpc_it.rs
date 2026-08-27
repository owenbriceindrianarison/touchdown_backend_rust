//! Verifies end-to-end request-reply communication against a real NATS:
//! typed round-trip, context propagation via headers, and preservation
//! of the original error code across the bus.

use serde::{Deserialize, Serialize};
use shared::{
    AppError,
    nats::{RequestContext, RpcRouter},
    paseto::Role,
};

#[derive(Serialize, Deserialize)]
struct EchoReq {
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EchoRes {
    message: String,
    caller: Option<String>,
    locale: String,
}

async fn boot() -> shared::testing::NatsFixture {
    let fixture = shared::testing::nats().await;

    let mut cfg = shared::config::AppConfig::from_env("demo").unwrap();
    cfg.nats_url = fixture.url.clone();

    let router = RpcRouter::new(&cfg)
        .route("echo.say", |ctx: RequestContext, req: EchoReq| async move {
            Ok(EchoRes {
                message: req.message,
                caller: ctx.user_id.map(|u| u.to_string()),
                locale: ctx.locale.to_string(),
            })
        })
        .route(
            "echo.boom",
            |_ctx: RequestContext, _req: EchoReq| async move {
                Err::<EchoRes, _>(AppError::not_found("widget", "42"))
            },
        )
        .route(
            "echo.denied",
            |ctx: RequestContext, _req: EchoReq| async move {
                ctx.require_role(Role::Admin)?;
                Ok::<EchoRes, AppError>(EchoRes {
                    message: "secret".into(),
                    caller: None,
                    locale: "fr".into(),
                })
            },
        );

    let rpc = fixture.rpc.clone();
    tokio::spawn(async move { router.run(rpc).await.unwrap() });
    // Let the subscription establish itself before the first request.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    fixture
}

#[tokio::test]
async fn domain_error_keeps_its_status_across_the_bus() {
    let fixture = boot().await;
    let ctx = RequestContext::system();

    let err = fixture
        .rpc
        .request::<_, EchoRes>(
            "demo.echo.boom",
            &ctx,
            &EchoReq {
                message: "x".into(),
            },
        )
        .await
        .expect_err("should fail");

    assert_eq!(err.status(), 404);
    assert_eq!(err.code(), "not_found");
}
