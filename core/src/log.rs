use anyhow::Result;
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

pub fn setup_logging() -> Result<()> {
  let otlp_exporter = opentelemetry_otlp::new_exporter()
    .tonic()
    .with_timeout(Duration::from_secs(3))
    .with_endpoint("http://localhost:22003");

  let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(otlp_exporter)
    .install_simple()?;

  let registry = Registry::default()
    .with(tracing_opentelemetry::layer().with_tracer(tracer))
    .with(tracing_subscriber::fmt::layer().json())
    .with(EnvFilter::from_default_env());

  tracing::subscriber::set_global_default(registry).expect("setting default subscriber failed");

  info!("Logging initialized");

  Ok(())
}
