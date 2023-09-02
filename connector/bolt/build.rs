fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut config = prost_build::Config::new();
  config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
  config.protoc_arg("--experimental_allow_proto3_optional");

  tonic_build::configure().compile_with_config(config, &["lute.proto"], &["../../proto"])?;
  Ok(())
}
