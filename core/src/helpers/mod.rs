use futures::future::BoxFuture;
use std::sync::Arc;

pub mod batch_loader;
pub mod fifo_queue;
pub mod key_value_store;
pub mod math;
pub mod redisearch;

pub type ThreadSafeAsyncFn<A = (), R = (), E = anyhow::Error> =
  Arc<dyn Fn(A) -> BoxFuture<'static, Result<R, E>> + Send + Sync>;
