use crate::settings::TracingSettings;
use anyhow::Result;
use opentelemetry::sdk::{trace, Resource};
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

pub fn setup_tracing(tracing_settings: &TracingSettings) -> Result<()> {
  let otlp_exporter = opentelemetry_otlp::new_exporter()
    .tonic()
    .with_timeout(Duration::from_secs(3))
    .with_endpoint(&tracing_settings.otel_collector_endpoint);

  let mut resource_labels = vec![
    opentelemetry::KeyValue::new(
      "service.namespace",
      tracing_settings.service_namespace.clone(),
    ),
    opentelemetry::KeyValue::new("service.name", tracing_settings.service_name.clone()),
    opentelemetry::KeyValue::new("host.name", tracing_settings.host_name.clone()),
  ];

  if let Some(labels) = &tracing_settings.resource_labels {
    resource_labels.extend(
      labels
        .iter()
        .map(|(key, value)| opentelemetry::KeyValue::new(key.clone(), value.clone())),
    )
  }

  let trace_config = trace::Config::default().with_resource(Resource::new(resource_labels));

  let tracer = opentelemetry_otlp::new_pipeline()
    .tracing()
    .with_exporter(otlp_exporter)
    .with_trace_config(trace_config)
    .install_simple()?;

  let registry = Registry::default()
    .with(tracing_opentelemetry::layer().with_tracer(tracer))
    .with(tracing_subscriber::fmt::layer().json())
    .with(EnvFilter::from_default_env());

  tracing::subscriber::set_global_default(registry).expect("setting default subscriber failed");

  info!("Tracing initialized");

  Ok(())
}
