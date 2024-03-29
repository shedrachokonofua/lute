use rustis::{bb8::PooledConnection, client::PooledClientManager, commands::SearchCommands};
use tracing::warn;
use unidecode::unidecode;

pub async fn does_ft_index_exist<'a>(
  connection: &PooledConnection<'a, PooledClientManager>,
  index_name: &str,
) -> bool {
  match connection.ft_info(index_name).await {
    Ok(_) => true,
    Err(err) => {
      warn!("Failed to check if index exists: {}", err);
      !err.to_string().contains("Unknown Index name")
    }
  }
}

pub fn escape_search_query_text(input: &str) -> String {
  unidecode(input.trim())
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() {
        c.to_string()
      } else {
        " ".to_string()
      }
    })
    .collect()
}

pub fn escape_tag_value(input: &str) -> String {
  input
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || c == '…' {
        c.to_string()
      } else if c.is_ascii() {
        format!("\\{}", c)
      } else {
        // Convert non-ASCII chars to UTF-8
        c.to_string()
          .as_bytes()
          .iter()
          .map(|b| format!("{:02x}", b))
          .collect::<Vec<String>>()
          .join("")
      }
    })
    .collect()
}
