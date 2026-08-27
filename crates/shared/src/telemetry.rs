use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::{AppConfig, LogFormat};

/// Initializes logs and traces. Call this only once, at the very beginning of `main`.
pub fn init(cfg: &AppConfig) -> TelemetryGuard {
    let filter = EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(filter);

    match cfg.log_format {
        LogFormat::Json => {
            let layer = tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(false);
            init_with(registry.with(layer), cfg)
        }
        LogFormat::Pretty => {
            let layer = tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_line_number(cfg.environment.is_dev());
            init_with(registry.with(layer), cfg)
        }
    }
}

#[cfg(not(feature = "otel"))]
fn init_with<S>(subscriber: S, cfg: &AppConfig) -> TelemetryGuard
where
    S: SubscriberInitExt,
{
    subscriber.init();
    if cfg.otlp_endpoint.is_some() {
        tracing::warn!("OTLP_ENDPOINT is set but the `otel` feature is disabled — ignoring");
    }
    TelemetryGuard::default()
}

#[cfg(feature = "otel")]
fn init_with<S>(subscriber: S, cfg: &AppConfig) -> TelemetryGuard
where
    S: tracing::Subscriber
        + for<'a> tracing_subscriber::layer::SubscriberExt
        + for<'span> tracing_subscriber::registry::LookupSpan<'span>
        + Send
        + Sync
        + 'static,
{
    use opentelemetry::{KeyValue, trace::TracerProvider as _};
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};

    let Some(endpoint) = cfg.otlp_endpoint.clone() else {
        subscriber.init();
        return TelemetryGuard::default();
    };

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("failed to build OTLP exporter");

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", cfg.service_name.clone()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ])
        .build();

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(cfg.service_name.clone());
    opentelemetry::global::set_tracer_provider(provider.clone());

    subscriber
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    TelemetryGuard {
        provider: Some(provider),
    }
}

#[derive(Default)]
pub struct TelemetryGuard {
    #[cfg(feature = "otel")]
    provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        #[cfg(feature = "otel")]
        if let Some(p) = self.provider.take() {
            let _ = p.shutdown();
        }
    }
}
