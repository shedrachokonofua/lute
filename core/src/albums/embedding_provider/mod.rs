pub mod onehot;
pub mod openai;
pub mod provider;

use crate::settings::Settings;
use openai::OpenAIAlbumEmbeddingProvider;
use provider::AlbumEmbeddingProvider;
use std::sync::Arc;

use self::onehot::OneHotAlbumEmbeddingProvider;

pub struct AlbumEmbeddingProvidersInteractor {
  pub providers: Vec<Arc<dyn AlbumEmbeddingProvider + Send + Sync>>,
}

impl AlbumEmbeddingProvidersInteractor {
  pub fn new(settings: Arc<Settings>) -> Self {
    let mut providers: Vec<Arc<dyn AlbumEmbeddingProvider + Send + Sync>> =
      vec![Arc::new(OneHotAlbumEmbeddingProvider::new())];
    if let Some(openai_settings) = &settings.embedding_provider.openai {
      providers.push(Arc::new(OpenAIAlbumEmbeddingProvider::new(openai_settings)));
    }
    Self { providers }
  }
}
