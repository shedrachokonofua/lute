use super::{
  provider::EmbeddingProvider,
  providers::{
    ollama::OllamaEmbeddingProvider, openai::OpenAIEmbeddingProvider,
    voyageai::VoyageAIEmbeddingProvider,
  },
};
use crate::{
  files::file_metadata::file_name::FileName, helpers::key_value_store::KeyValueStore,
  settings::Settings,
};
use anyhow::{anyhow, Result};
use chrono::Duration;
use ollama_rs::Ollama;
use reqwest::Url;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};

struct EmbeddingProviderCache {
  kv: Arc<KeyValueStore>,
}

impl EmbeddingProviderCache {
  pub fn new(kv: Arc<KeyValueStore>) -> Self {
    Self { kv }
  }

  pub fn build_key(&self, provider_name: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    let hash = format!("{:x}", result);
    format!("embedding_cache:{}:{}", provider_name, hash)
  }

  pub async fn set_many(&self, provider_name: &str, items: Vec<(String, Vec<f32>)>) -> Result<()> {
    self
      .kv
      .set_many(
        items
          .iter()
          .map(|(key, value)| {
            Ok((
              self.build_key(provider_name, key.as_str()),
              value.clone(),
              Duration::try_weeks(2).map(|d| d.to_std()).transpose()?,
            ))
          })
          .collect::<Result<Vec<_>>>()?,
      )
      .await?;
    Ok(())
  }

  pub async fn get_many(
    &self,
    provider_name: &str,
    keys: HashMap<FileName, String>,
  ) -> Result<HashMap<FileName, Vec<f32>>> {
    let mut key_file_names = keys
      .iter()
      .map(|(file_name, content)| {
        (
          self.build_key(provider_name, content.as_str()),
          file_name.clone(),
        )
      })
      .collect::<HashMap<_, _>>();
    let cached_embeddings = self
      .kv
      .get_many::<Vec<f32>>(key_file_names.keys().cloned().collect())
      .await?;
    let mut embeddings = HashMap::new();
    for (key, value) in cached_embeddings {
      if let Some(file_name) = key_file_names.remove(&key) {
        embeddings.insert(file_name, value);
      }
    }

    Ok(embeddings)
  }
}

pub struct EmbeddingProviderInteractor {
  pub providers: HashMap<String, Arc<dyn EmbeddingProvider + Send + Sync>>,
  cache: EmbeddingProviderCache,
}

impl EmbeddingProviderInteractor {
  pub fn new(settings: Arc<Settings>, kv: Arc<KeyValueStore>) -> Self {
    let mut providers: HashMap<String, Arc<dyn EmbeddingProvider + Send + Sync>> = HashMap::new();

    if let Some(openai_settings) = &settings.embedding_provider.openai {
      let provider = Arc::new(OpenAIEmbeddingProvider::new(openai_settings));
      providers.insert(provider.name().to_string(), provider);
    }

    if let Some(voyageai_settings) = &settings.embedding_provider.voyageai {
      let provider = Arc::new(VoyageAIEmbeddingProvider::new(voyageai_settings.clone()));
      providers.insert(provider.name().to_string(), provider);
    }

    if let Some(ollama_settings) = &settings.embedding_provider.ollama {
      if let Ok(url) = Url::parse(
        ollama_settings
          .url
          .clone()
          .unwrap_or("http://localhost:11434".to_string())
          .as_str(),
      )
      .inspect_err(|e| {
        tracing::error!(
          "Failed to parse Ollama URL, cannot register Ollama providers: {}",
          e
        )
      }) {
        for model in ollama_settings.models.iter() {
          let provider = Arc::new(OllamaEmbeddingProvider::new(
            model.clone(),
            Ollama::from_url(url.clone()),
          ));
          providers.insert(provider.name().to_string(), provider);
        }
      }
    }

    Self {
      providers,
      cache: EmbeddingProviderCache::new(kv),
    }
  }

  pub fn get_provider_by_name(
    &self,
    name: &str,
  ) -> Result<Arc<dyn EmbeddingProvider + Send + Sync>> {
    let provider = self
      .providers
      .get(name)
      .ok_or_else(|| anyhow!("Provider not found: {}", name))?;
    Ok(Arc::clone(provider))
  }

  #[instrument(skip(self, input), fields(count = input.len()))]
  pub async fn generate(
    &self,
    provider_name: &str,
    input: HashMap<FileName, String>,
  ) -> Result<HashMap<FileName, Vec<f32>>> {
    let provider = self.get_provider_by_name(provider_name)?;
    let mut input = input;
    let mut embeddings = self.cache.get_many(provider_name, input.clone()).await?;
    let uncached_keys = input
      .keys()
      .filter(|&key| !embeddings.contains_key(key))
      .cloned()
      .collect::<Vec<_>>();

    if uncached_keys.is_empty() {
      info!(count = embeddings.len(), "All embeddings are cached");
      return Ok(embeddings);
    }

    let new_embeddings = provider
      .generate(
        uncached_keys
          .iter()
          .filter_map(|key| input.get(key))
          .cloned()
          .collect(),
      )
      .await?;
    let mut cache_input = HashMap::new();
    for (key, value) in uncached_keys.into_iter().zip(new_embeddings.into_iter()) {
      if let Some(content) = input.remove(&key) {
        cache_input.insert(content, value.clone());
      }
      embeddings.insert(key, value);
    }
    self
      .cache
      .set_many(provider_name, cache_input.into_iter().collect::<Vec<_>>())
      .await?;

    Ok(embeddings)
  }
}
