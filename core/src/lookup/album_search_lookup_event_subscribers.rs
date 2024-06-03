use super::{
  album_search_lookup::{
    get_query_from_album_search_correlation_id, is_album_search_correlation_id, AlbumSearchLookup,
    AlbumSearchLookupStep,
  },
  lookup_interactor::LookupInteractor,
};
use crate::{
  albums::album_interactor::AlbumInteractor,
  context::ApplicationContext,
  crawler::crawler::{Crawler, QueuePushParameters},
  events::{
    event::{Event, EventPayloadBuilder, Topic},
    event_publisher::EventPublisher,
    event_subscriber::{
      EventData, EventHandler, EventSubscriber, EventSubscriberBuilder, GroupingStrategy,
    },
  },
  files::file_metadata::{file_name::FileName, page_type::PageType},
  helpers::priority::Priority,
  parser::parsed_file_data::ParsedFileData,
};
use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tracing::{info, instrument, warn};

impl AlbumSearchLookup {
  #[instrument(skip(self))]
  fn apply_file_processing_event(
    &self,
    event: Event,
    correlation_id: String,
  ) -> Option<AlbumSearchLookup> {
    match event {
      Event::FileSaved { file_name, .. } => match file_name.page_type() {
        PageType::AlbumSearchResult => {
          if self.can_transition(AlbumSearchLookupStep::SearchParsing, &correlation_id) {
            Some(AlbumSearchLookup::SearchParsing {
              album_search_file_name: file_name,
              query: self.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
            })
          } else {
            None
          }
        }
        PageType::Album => {
          if self.can_transition(AlbumSearchLookupStep::AlbumParsing, &correlation_id) {
            Some(AlbumSearchLookup::AlbumParsing {
              album_search_file_name: file_name,
              query: self.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: self.parsed_album_search_result().unwrap(),
            })
          } else {
            None
          }
        }
        _ => None,
      },
      Event::FileParsed {
        file_name, data, ..
      } => match data {
        ParsedFileData::AlbumSearchResult(album_search_result) => {
          if self.can_transition(AlbumSearchLookupStep::SearchParsed, &correlation_id) {
            Some(AlbumSearchLookup::SearchParsed {
              album_search_file_name: file_name,
              query: self.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: album_search_result,
            })
          } else {
            None
          }
        }
        ParsedFileData::Album(album) => {
          if self.can_transition(AlbumSearchLookupStep::AlbumParsed, &correlation_id) {
            Some(AlbumSearchLookup::AlbumParsed {
              album_search_file_name: file_name,
              query: self.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: self.parsed_album_search_result().unwrap(),
              parsed_album: album,
            })
          } else {
            None
          }
        }
        _ => None,
      },
      Event::FileParseFailed {
        file_name, error, ..
      } => match file_name.page_type() {
        PageType::AlbumSearchResult => {
          if self.can_transition(AlbumSearchLookupStep::SearchParseFailed, &correlation_id) {
            Some(AlbumSearchLookup::SearchParseFailed {
              album_search_file_name: file_name,
              query: self.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              album_search_file_parse_error: error,
            })
          } else {
            None
          }
        }
        PageType::Album => {
          if self.can_transition(AlbumSearchLookupStep::AlbumParseFailed, &correlation_id) {
            Some(AlbumSearchLookup::AlbumParseFailed {
              album_search_file_name: file_name,
              query: self.query().clone(),
              last_updated_at: chrono::Utc::now().naive_utc(),
              file_processing_correlation_id: correlation_id.clone(),
              parsed_album_search_result: self.parsed_album_search_result().unwrap(),
              album_file_parse_error: error,
            })
          } else {
            None
          }
        }
        _ => None,
      },
      _ => None,
    }
  }
}

struct AlbumSearchLookupOrchestrator {
  crawler: Arc<Crawler>,
  lookup_interactor: LookupInteractor,
  event_publisher: Arc<EventPublisher>,
  album_interactor: Arc<AlbumInteractor>,
}

impl AlbumSearchLookupOrchestrator {
  fn new(app_context: Arc<ApplicationContext>) -> Self {
    Self {
      crawler: Arc::clone(&app_context.crawler),
      lookup_interactor: LookupInteractor::new(
        Arc::clone(&app_context.settings),
        Arc::clone(&app_context.redis_connection_pool),
        Arc::clone(&app_context.sqlite_connection),
      ),
      event_publisher: Arc::clone(&app_context.event_publisher),
      album_interactor: Arc::clone(&app_context.album_interactor),
    }
  }

  #[instrument(skip(self))]
  async fn save_lookup(&self, lookup: &AlbumSearchLookup) -> Result<()> {
    info!("Saving album search lookup");
    self
      .lookup_interactor
      .put_album_search_lookup(lookup)
      .await?;
    Ok(())
  }

  #[instrument(skip(self))]
  async fn enqueue_to_crawler(&self, file_name: &FileName, correlation_id: String) -> Result<()> {
    self
      .crawler
      .enqueue(QueuePushParameters {
        file_name: file_name.clone(),
        priority: Some(Priority::High),
        deduplication_key: Some(format!("{}:{}", file_name.to_string(), correlation_id)),
        correlation_id: Some(correlation_id),
      })
      .await?;
    Ok(())
  }

  #[instrument(skip(self))]
  async fn handle_lookup_event(&self, event: Event, correlation_id: String) -> Result<()> {
    if let Event::LookupAlbumSearchUpdated { lookup } = event {
      self.save_lookup(&lookup).await?;

      if let AlbumSearchLookup::Started { query, .. } = lookup {
        self
          .enqueue_to_crawler(&query.file_name(), correlation_id.clone())
          .await?;
        self
          .save_lookup(&AlbumSearchLookup::SearchCrawling {
            query: query.clone(),
            last_updated_at: Utc::now().naive_utc(),
            album_search_file_name: query.file_name(),
            file_processing_correlation_id: correlation_id.clone(),
          })
          .await?;
      } else if let AlbumSearchLookup::SearchParsed {
        parsed_album_search_result,
        query,
        album_search_file_name,
        file_processing_correlation_id,
        ..
      } = lookup
      {
        match self
          .album_interactor
          .find(&parsed_album_search_result.file_name)
          .await?
        {
          Some(album) => {
            self
              .event_publisher
              .publish(
                Topic::Lookup,
                EventPayloadBuilder::default()
                  .key(correlation_id.clone())
                  .event(Event::LookupAlbumSearchUpdated {
                    lookup: AlbumSearchLookup::AlbumParsed {
                      query,
                      last_updated_at: Utc::now().naive_utc(),
                      album_search_file_name,
                      file_processing_correlation_id,
                      parsed_album_search_result,
                      parsed_album: album.into(),
                    },
                  })
                  .correlation_id(correlation_id.clone())
                  .build()?,
              )
              .await?;
          }
          None => {
            self
              .enqueue_to_crawler(
                &parsed_album_search_result.file_name,
                correlation_id.clone(),
              )
              .await?;
            self
              .save_lookup(&AlbumSearchLookup::AlbumCrawling {
                query,
                last_updated_at: Utc::now().naive_utc(),
                album_search_file_name,
                file_processing_correlation_id,
                parsed_album_search_result,
              })
              .await?;
          }
        }
      }
    }
    Ok(())
  }

  async fn handle_non_related_event(&self, event: Event) -> Result<()> {
    if let Event::FileParsed {
      file_name,
      data: ParsedFileData::Album(album),
      ..
    } = event
    {
      let lookups = self
        .lookup_interactor
        .find_many_album_search_lookups_by_album_file_name(&file_name)
        .await?;
      for lookup in lookups {
        info!(
          file_name = file_name.to_string(),
          "Found album search lookup for album file name"
        );
        self
          .event_publisher
          .publish(
            Topic::Lookup,
            EventPayloadBuilder::default()
              .key(lookup.file_processing_correlation_id().clone())
              .event(Event::LookupAlbumSearchUpdated {
                lookup: AlbumSearchLookup::AlbumParsed {
                  album_search_file_name: lookup.album_search_file_name().unwrap(),
                  query: lookup.query().clone(),
                  last_updated_at: chrono::Utc::now().naive_utc(),
                  file_processing_correlation_id: lookup.file_processing_correlation_id().clone(),
                  parsed_album_search_result: lookup.parsed_album_search_result().unwrap(),
                  parsed_album: album.clone(),
                },
              })
              .correlation_id(lookup.file_processing_correlation_id().clone())
              .build()?,
          )
          .await?;
      }
    }
    Ok(())
  }

  #[instrument(skip(self))]
  async fn handle_file_processing_event(&self, event: Event, correlation_id: String) -> Result<()> {
    let query = get_query_from_album_search_correlation_id(&correlation_id)?;
    let lookup = self
      .lookup_interactor
      .find_album_search_lookup(&query)
      .await?;

    if lookup.is_none() {
      warn!("No album search lookup found for correlation id");
      return Ok(());
    }
    let lookup = lookup.unwrap();

    if let Some(next_lookup) = lookup.apply_file_processing_event(event, correlation_id.clone()) {
      info!(
        current_step = lookup.step().to_string(),
        next_step = next_lookup.step().to_string(),
        "Transitioning album search lookup"
      );
      self
        .event_publisher
        .publish(
          Topic::Lookup,
          EventPayloadBuilder::default()
            .key(correlation_id.clone())
            .event(Event::LookupAlbumSearchUpdated {
              lookup: next_lookup.clone(),
            })
            .correlation_id(correlation_id)
            .build()?,
        )
        .await?;
    } else {
      info!(
        current_step = lookup.step().to_string(),
        "Ignoring file processing event"
      );
    }
    Ok(())
  }

  async fn handle_event(&self, event_data: EventData) -> Result<()> {
    if event_data.payload.correlation_id.is_none() {
      return Ok(());
    }
    let correlation_id = event_data.payload.correlation_id.unwrap();

    if is_album_search_correlation_id(&correlation_id) {
      let event = event_data.payload.event;
      match event_data.topic {
        Topic::Lookup => {
          self
            .handle_lookup_event(event, correlation_id.clone())
            .await?
        }
        Topic::File | Topic::Parser => {
          self
            .handle_file_processing_event(event, correlation_id.clone())
            .await?
        }
        _ => (),
      }
    } else {
      self
        .handle_non_related_event(event_data.payload.event)
        .await?;
    }

    Ok(())
  }
}

pub fn build_album_search_lookup_event_subscribers(
  app_context: Arc<ApplicationContext>,
) -> Result<Vec<EventSubscriber>> {
  let orchestrator = Arc::new(AlbumSearchLookupOrchestrator::new(Arc::clone(&app_context)));
  Ok(vec![EventSubscriberBuilder::default()
    .id("album_search_lookup")
    .topics(vec![Topic::File, Topic::Parser, Topic::Lookup])
    .batch_size(250)
    .app_context(Arc::clone(&app_context))
    .grouping_strategy(GroupingStrategy::GroupByCorrelationId)
    .handler(EventHandler::Single(Arc::new(move |(event_data, _, _)| {
      let orchestrator = Arc::clone(&orchestrator);
      Box::pin(async move { orchestrator.handle_event(event_data).await })
    })))
    .build()?])
}
