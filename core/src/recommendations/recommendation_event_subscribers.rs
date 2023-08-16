use crate::{
  albums::album_read_model_repository::AlbumReadModelRepository,
  crawler::{
    crawler_interactor::CrawlerInteractor,
    priority_queue::{Priority, QueuePushParameters},
  },
  events::{
    event::{Event, Stream},
    event_subscriber::{EventSubscriber, EventSubscriberBuilder, SubscriberContext},
  },
  files::file_metadata::file_name::ChartParameters,
  settings::Settings,
};
use anyhow::Result;
use chrono::{Datelike, Local};
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;
use tracing::warn;

async fn crawl_similar_albums(
  context: SubscriberContext,
  crawler_interactor: Arc<CrawlerInteractor>,
  album_read_model_repository: AlbumReadModelRepository,
) -> Result<()> {
  if let Event::ProfileAlbumAdded { file_name, .. } = context.payload.event {
    let album = album_read_model_repository.get(&file_name).await?;
    if let Some(release_date) = album.release_date {
      let file_name_string = file_name.to_string();
      let release_type = file_name_string.split('/').collect::<Vec<&str>>()[1];

      // Artists
      for artist in album.artists {
        if let Err(e) = crawler_interactor
          .enqueue_if_stale(QueuePushParameters {
            file_name: artist.file_name,
            correlation_id: Some(format!("crawl_similar_albums:{}", file_name.to_string())),
            priority: Some(Priority::Low),
            deduplication_key: None,
            metadata: None,
          })
          .await
        {
          warn!(
            error = e.to_string(),
            "Failed to enqueue artists for similar albums"
          );
        }
      }

      // Same genres, same year
      let mut primary_genres = album.primary_genres.clone();
      primary_genres.insert(0, "all".to_string());
      if let Err(e) = crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: ChartParameters {
            release_type: release_type.to_string(),
            page_number: 1,
            years_range_start: release_date.year() as u32,
            years_range_end: release_date.year() as u32,
            include_primary_genres: Some(primary_genres),
            ..Default::default()
          }
          .try_into()?,
          correlation_id: Some(format!("crawl_similar_albums:{}", file_name.to_string())),
          priority: Some(Priority::Low),
          deduplication_key: None,
          metadata: None,
        })
        .await
      {
        warn!(
          error = e.to_string(),
          "Failed to enqueue similar albums chart"
        );
      }

      // Same genres, same descriptors
      if let Err(e) = crawler_interactor
        .enqueue_if_stale(QueuePushParameters {
          file_name: ChartParameters {
            release_type: release_type.to_string(),
            page_number: 1,
            years_range_start: 1900,
            years_range_end: Local::now().year() as u32,
            include_primary_genres: Some(album.primary_genres.clone()),
            include_descriptors: Some(album.descriptors.clone()),
            ..Default::default()
          }
          .try_into()?,
          correlation_id: Some(format!("crawl_similar_albums:{}", file_name.to_string())),
          priority: Some(Priority::Low),
          deduplication_key: None,
          metadata: None,
        })
        .await
      {
        warn!(
          error = e.to_string(),
          "Failed to enqueue similar albums chart"
        );
      }
    }
  }
  Ok(())
}

pub fn build_recommendation_event_subscribers(
  redis_connection_pool: Arc<Pool<PooledClientManager>>,
  settings: Arc<Settings>,
  crawler_interactor: Arc<CrawlerInteractor>,
) -> Result<Vec<EventSubscriber>> {
  Ok(vec![EventSubscriberBuilder::default()
    .id("crawl_similar_albums".to_string())
    .stream(Stream::Profile)
    .concurrency(Some(250))
    .redis_connection_pool(Arc::clone(&redis_connection_pool))
    .settings(Arc::clone(&settings))
    .handle(Arc::new(move |context| {
      let crawler_interactor = Arc::clone(&crawler_interactor);
      let album_read_model_repository = AlbumReadModelRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      };
      Box::pin(async move {
        crawl_similar_albums(
          context,
          Arc::clone(&crawler_interactor),
          album_read_model_repository,
        )
        .await
      })
    }))
    .build()?])
}
