use super::{
  album_search::{
    album_search_lookup::{
      get_album_search_correlation_id, AlbumSearchLookup, AlbumSearchLookupQuery,
    },
    album_search_lookup_repository::{AggregatedStatus, AlbumSearchLookupRepository},
  },
  file_processing_status::{FileProcessingStatus, FileProcessingStatusRepository},
  list::{
    list_lookup_interactor::ListLookupInteractor, list_lookup_repository::ListSegmentReadModel,
  },
  ListLookup,
};
use crate::{
  crawler::crawler::Crawler,
  events::{
    event::{Event, EventPayloadBuilder, Topic},
    event_publisher::EventPublisher,
  },
  files::file_metadata::file_name::{FileName, ListRootFileName},
  helpers::{document_store::DocumentStore, key_value_store::KeyValueStore},
  sqlite::SqliteConnection,
};
use anyhow::Result;
use std::{collections::HashMap, sync::Arc};

pub struct LookupInteractor {
  file_processing_status_repository: Arc<FileProcessingStatusRepository>,
  album_search_lookup_repository: AlbumSearchLookupRepository,
  event_publisher: Arc<EventPublisher>,
  list_lookup_interactor: ListLookupInteractor,
}

impl LookupInteractor {
  pub fn new(
    sqlite_connection: Arc<SqliteConnection>,
    doc_store: Arc<DocumentStore>,
    event_publisher: Arc<EventPublisher>,
    kv: Arc<KeyValueStore>,
    crawler: Arc<Crawler>,
  ) -> Self {
    let file_processing_status_repository = Arc::new(FileProcessingStatusRepository::new(kv));
    Self {
      album_search_lookup_repository: AlbumSearchLookupRepository::new(doc_store),
      file_processing_status_repository: Arc::clone(&file_processing_status_repository),
      event_publisher: Arc::clone(&event_publisher),
      list_lookup_interactor: ListLookupInteractor::new(
        file_processing_status_repository,
        sqlite_connection,
        crawler,
        event_publisher,
      ),
    }
  }

  pub async fn put_album_search_lookup(&self, lookup: &AlbumSearchLookup) -> Result<()> {
    self.album_search_lookup_repository.put(lookup).await
  }

  pub async fn find_album_search_lookup(
    &self,
    query: &AlbumSearchLookupQuery,
  ) -> Result<Option<AlbumSearchLookup>> {
    self.album_search_lookup_repository.find(query).await
  }

  pub async fn get_album_search_lookup(
    &self,
    query: &AlbumSearchLookupQuery,
  ) -> Result<AlbumSearchLookup> {
    self.album_search_lookup_repository.get(query).await
  }

  pub async fn search_album(
    &self,
    artist_name: String,
    album_name: String,
  ) -> Result<AlbumSearchLookup> {
    let query = AlbumSearchLookupQuery::new(album_name, artist_name);
    let lookup = self.album_search_lookup_repository.find(&query).await?;
    match lookup {
      Some(AlbumSearchLookup::Started { .. }) | None => {
        let lookup = AlbumSearchLookup::new(query);
        self.put_album_search_lookup(&lookup).await?;
        self
          .event_publisher
          .publish(
            Topic::Lookup,
            EventPayloadBuilder::default()
              .key(get_album_search_correlation_id(lookup.query()))
              .event(Event::LookupAlbumSearchUpdated {
                lookup: lookup.clone(),
              })
              .correlation_id(get_album_search_correlation_id(lookup.query()))
              .build()?,
          )
          .await?;
        Ok(lookup)
      }
      Some(lookup) => Ok(lookup),
    }
  }

  pub async fn aggregate_statuses(&self) -> Result<Vec<AggregatedStatus>> {
    self
      .album_search_lookup_repository
      .aggregate_statuses()
      .await
  }

  pub async fn find_many_album_search_lookups(
    &self,
    queries: Vec<&AlbumSearchLookupQuery>,
  ) -> Result<HashMap<String, AlbumSearchLookup>> {
    self.album_search_lookup_repository.find_many(queries).await
  }

  pub async fn find_many_album_search_lookups_by_album_file_name(
    &self,
    album_file_name: &FileName,
  ) -> Result<Vec<AlbumSearchLookup>> {
    self
      .album_search_lookup_repository
      .find_many_by_album_file_name(album_file_name)
      .await
  }

  pub async fn delete_album_search_lookup(&self, query: &AlbumSearchLookupQuery) -> Result<()> {
    self.album_search_lookup_repository.delete(query).await
  }

  pub async fn put_many_file_processing_status(
    &self,
    input: HashMap<FileName, FileProcessingStatus>,
  ) -> Result<()> {
    self
      .file_processing_status_repository
      .put_many(input)
      .await?;
    Ok(())
  }

  pub async fn delete_many_file_processing_status(&self, input: Vec<FileName>) -> Result<()> {
    self
      .file_processing_status_repository
      .delete_many(input)
      .await
  }

  pub async fn put_many_list_segments(&self, docs: Vec<ListSegmentReadModel>) -> Result<()> {
    self
      .list_lookup_interactor
      .put_many_list_segments(docs)
      .await?;
    Ok(())
  }

  pub async fn put_list_lookup(&self, root_file_name: ListRootFileName) -> Result<ListLookup> {
    self.list_lookup_interactor.put_lookup(root_file_name).await
  }
}
