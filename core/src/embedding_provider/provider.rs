use crate::scheduler::job_name::JobName;
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait EmbeddingProvider {
  fn name(&self) -> &str;
  fn dimensions(&self) -> usize;
  fn interval(&self) -> Duration;
  fn concurrency(&self) -> usize;
  fn batch_size(&self) -> usize;
  fn job_name(&self) -> JobName;
  async fn generate(&self, inputs: Vec<String>) -> Result<Vec<Vec<f32>>>;
}
