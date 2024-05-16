use anyhow::Result;
use futures::future::BoxFuture;
use std::sync::Arc;

pub mod batch_loader;
pub mod fifo_queue;
pub mod key_value_store;
pub mod math;
pub mod redisearch;

pub type ThreadSafeAsyncFn<A = (), R = ()> =
  Arc<dyn Fn(A) -> BoxFuture<'static, Result<R>> + Send + Sync>;
