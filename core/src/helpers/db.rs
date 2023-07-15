use rustis::{bb8::PooledConnection, client::PooledClientManager, commands::SearchCommands};

pub async fn does_ft_index_exist<'a>(
  connection: &PooledConnection<'a, PooledClientManager>,
  index_name: &str,
) -> bool {
  match connection.ft_info(index_name).await {
    Ok(_) => true,
    Err(err) => !err.to_string().contains("Unknown Index name"),
  }
}
