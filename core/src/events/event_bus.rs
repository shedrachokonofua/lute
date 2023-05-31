use super::event::{Event, EventTag};
use r2d2::{Pool, PooledConnection};
use redis::Client;
use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

pub trait EventSubscriber<T> {
  fn get_name(&self) -> String;
  fn consume_event(&self, event: &T);
}

type Subscriber = Box<dyn EventSubscriber<Event> + Send + Sync>;

pub struct EventBus {
  redis_connection_pool: Arc<Pool<Client>>,
  subscribers: Arc<Mutex<HashMap<EventTag, Vec<Subscriber>>>>,
}

impl EventBus {
  pub fn new(redis_connection_pool: Arc<Pool<Client>>) -> Self {
    Self {
      redis_connection_pool,
      subscribers: Arc::new(Mutex::new(HashMap::new())),
    }
  }
}
