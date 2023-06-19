use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut config = prost_build::Config::new();
  config.protoc_arg("--experimental_allow_proto3_optional");

  tonic_build::configure()
    .file_descriptor_set_path(
      PathBuf::from(env::var("OUT_DIR").unwrap()).join("lute_descriptor.bin"),
    )
    .compile_with_config(config, &["lute.proto"], &["../proto"])?;
  Ok(())
}
