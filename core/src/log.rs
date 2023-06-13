use tracing::info;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

pub fn setup_logging() {
  let formatting_layer = BunyanFormattingLayer::new("lute".into(), std::io::stdout);
  let subscriber = Registry::default()
    .with(JsonStorageLayer)
    .with(formatting_layer);
  tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
  info!("Logging initialized");
}
