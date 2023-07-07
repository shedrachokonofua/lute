use super::{
  album_search_lookup::{
    get_album_search_correlation_id, AlbumSearchLookup, AlbumSearchLookupQuery,
  },
  album_search_lookup_repository::AlbumSearchLookupRepository,
};
use crate::events::{
  event::{Event, EventPayload, Stream},
  event_publisher::EventPublisher,
};
use anyhow::Result;
use rustis::{bb8::Pool, client::PooledClientManager};
use std::sync::Arc;

pub struct LookupInteractor {
  album_search_lookup_repository: AlbumSearchLookupRepository,
  event_publisher: EventPublisher,
}

impl LookupInteractor {
  pub fn new(redis_connection_pool: Arc<Pool<PooledClientManager>>) -> Self {
    Self {
      album_search_lookup_repository: AlbumSearchLookupRepository {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
      event_publisher: EventPublisher {
        redis_connection_pool: Arc::clone(&redis_connection_pool),
      },
    }
  }

  pub async fn put_lookup(&self, lookup: &AlbumSearchLookup) -> Result<()> {
    self.album_search_lookup_repository.put(lookup).await?;
    self
      .event_publisher
      .publish(
        Stream::Lookup,
        EventPayload {
          event: Event::LookupAlbumSearchStatusChanged {
            lookup: lookup.clone(),
          },
          correlation_id: Some(get_album_search_correlation_id(&lookup.query())),
          metadata: None,
        },
      )
      .await?;
    Ok(())
  }

  pub async fn search_album(
    &self,
    artist_name: String,
    album_name: String,
  ) -> Result<AlbumSearchLookup> {
    let query = AlbumSearchLookupQuery::new(album_name, artist_name);
    let lookup = self.album_search_lookup_repository.find(&query).await?;
    match lookup {
      Some(lookup) => Ok(lookup),
      None => {
        let lookup = AlbumSearchLookup::new(query);
        self.put_lookup(&lookup).await?;
        Ok(lookup)
      }
    }
  }
}
