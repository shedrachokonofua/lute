use tracing::info;

pub fn setup_logging() {
  tracing_subscriber::fmt().json().init();
  info!("Logging initialized");
}
