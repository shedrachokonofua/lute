use super::parsed_file_data::ParsedListSegment;
use anyhow::{anyhow, Result};
use tracing::{instrument, warn};

#[instrument(skip_all)]
pub fn parse_list_segment(file_content: &str) -> Result<ParsedListSegment> {
  let dom = tl::parse(file_content, tl::ParserOptions::default())?;
  Err(anyhow!("Not implemented"))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_resource;

  #[test]
  fn test_parse_list_segment() -> Result<(), String> {
    let file_content = test_resource!("list_segment.html");
    let result = parse_list_segment(file_content).map_err(|err| err.to_string());
    assert_eq!(result.unwrap_err(), "Not implemented".to_string());
    Ok(())
  }
}
