pub mod onehot;
pub mod openai;
pub mod provider;

use crate::{helpers::key_value_store::KeyValueStore, settings::Settings};
use onehot::OneHotAlbumEmbeddingProvider;
use openai::OpenAIAlbumEmbeddingProvider;
use provider::AlbumEmbeddingProvider;
use std::sync::Arc;

pub struct AlbumEmbeddingProvidersInteractor {
  pub providers: Vec<Arc<dyn AlbumEmbeddingProvider + Send + Sync>>,
}

impl AlbumEmbeddingProvidersInteractor {
  pub fn new(settings: Arc<Settings>, kv: Arc<KeyValueStore>) -> Self {
    let mut providers: Vec<Arc<dyn AlbumEmbeddingProvider + Send + Sync>> =
      vec![Arc::new(OneHotAlbumEmbeddingProvider::new())];
    if let Some(openai_settings) = &settings.embedding_provider.openai {
      providers.push(Arc::new(OpenAIAlbumEmbeddingProvider::new(
        openai_settings,
        kv,
      )));
    }
    Self { providers }
  }
}
