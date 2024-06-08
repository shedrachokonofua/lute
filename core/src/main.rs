use anyhow::Result;
use lute::{
  albums::album_event_subscribers::build_album_event_subscribers,
  artists::artist_event_subscribers::build_artist_event_subscribers,
  context::ApplicationContext,
  crawler::crawler_jobs::setup_crawler_jobs,
  events::{event_subscriber::EventSubscriber, event_subscriber_jobs::setup_event_subscriber_jobs},
  helpers::key_value_store::setup_kv_jobs,
  lookup::lookup_event_subscribers::build_lookup_event_subscribers,
  parser::{
    parser_event_subscribers::build_parser_event_subscribers, parser_jobs::setup_parser_jobs,
  },
  profile::profile_event_subscribers::build_profile_event_subscribers,
  recommendations::{
    recommendation_event_subscribers::build_recommendation_event_subscribers,
    recommendation_jobs::setup_recommendation_jobs,
  },
  redis::setup_redis_indexes,
  rpc::RpcServer,
};
use mimalloc::MiMalloc;
use std::{collections::HashMap, sync::Arc};
use tokio::spawn;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn start_event_subscribers(app_context: Arc<ApplicationContext>) -> Result<()> {
  let mut event_subscribers: Vec<EventSubscriber> = Vec::new();
  event_subscribers.extend(build_album_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_artist_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_lookup_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_parser_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_profile_event_subscribers(Arc::clone(&app_context))?);
  event_subscribers.extend(build_recommendation_event_subscribers(Arc::clone(
    &app_context,
  ))?);
  event_subscribers.into_iter().for_each(|subscriber| {
    spawn(async move { subscriber.run().await });
  });
  Ok(())
}

async fn setup_jobs(context: Arc<ApplicationContext>) -> Result<()> {
  setup_crawler_jobs(Arc::clone(&context)).await?;
  setup_event_subscriber_jobs(Arc::clone(&context)).await?;
  setup_kv_jobs(Arc::clone(&context)).await?;
  setup_parser_jobs(Arc::clone(&context)).await?;
  setup_recommendation_jobs(context).await?;
  Ok(())
}

async fn setup_elasticsearch_indexes(context: Arc<ApplicationContext>) -> Result<()> {
  context.artist_interactor.setup_search_index().await?;
  Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
  let context = ApplicationContext::init().await?;
  context
    .doc_store
    .setup_indexes(HashMap::from([
      (
        "profile_spotify_import",
        vec!["album_search_lookup_encoded_query", "profile_id"],
      ),
      (
        "album_search_lookup",
        vec!["status", "parsed_album_search_result.file_name"],
      ),
    ]))
    .await?;
  setup_elasticsearch_indexes(Arc::clone(&context)).await?;
  setup_redis_indexes(Arc::clone(&context)).await?;
  start_event_subscribers(Arc::clone(&context))?;
  setup_jobs(Arc::clone(&context)).await?;
  context.scheduler.run().await?;
  RpcServer::new(context).run().await?;
  Ok(())
}
