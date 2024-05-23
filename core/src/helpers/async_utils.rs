use futures::{future::BoxFuture, Future};
use std::sync::Arc;

pub type ThreadSafeAsyncFn<A = (), R = (), E = anyhow::Error> =
  Arc<dyn Fn(A) -> BoxFuture<'static, Result<R, E>> + Send + Sync>;

pub fn async_callback<Fut, A, R, E>(f: fn(A) -> Fut) -> ThreadSafeAsyncFn<A, R, E>
where
  Fut: Future<Output = Result<R, E>> + Send + 'static,
  A: Send + 'static,
{
  Arc::new(move |arg| Box::pin(async move { f(arg).await }))
}
