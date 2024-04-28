mod helpers;
pub mod onehot;
pub mod openai;
pub mod provider;
pub mod voyageai;

use crate::{helpers::key_value_store::KeyValueStore, settings::Settings};
use onehot::OneHotAlbumEmbeddingProvider;
use openai::OpenAIAlbumEmbeddingProvider;
use provider::AlbumEmbeddingProvider;
use std::sync::Arc;

use self::voyageai::VoyageAIAlbumEmbeddingProvider;

pub struct AlbumEmbeddingProvidersInteractor {
  pub providers: Vec<Arc<dyn AlbumEmbeddingProvider + Send + Sync>>,
}

impl AlbumEmbeddingProvidersInteractor {
  pub fn new(settings: Arc<Settings>, kv: Arc<KeyValueStore>) -> Self {
    let mut providers: Vec<Arc<dyn AlbumEmbeddingProvider + Send + Sync>> = vec![];
    if let Some(openai_settings) = &settings.embedding_provider.openai {
      providers.push(Arc::new(OpenAIAlbumEmbeddingProvider::new(
        openai_settings,
        Arc::clone(&kv),
      )));
    }
    if let Some(_) = &settings.embedding_provider.voyageai {
      providers.push(Arc::new(VoyageAIAlbumEmbeddingProvider::new(kv)));
    }
    providers.push(Arc::new(OneHotAlbumEmbeddingProvider::new()));
    Self { providers }
  }
}
