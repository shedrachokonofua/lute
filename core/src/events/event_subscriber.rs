use super::event::{EventPayload, Stream};
use anyhow::Result;
use r2d2::Pool;
use rayon::prelude::*;
use redis::{
  streams::{StreamReadOptions, StreamReadReply},
  Client, Commands,
};
use std::thread;
use std::{sync::Arc, time::Duration};

pub struct EventSubscriber {
  pub redis_pool: Arc<Pool<Client>>,
  pub id: String,
  pub stream: Stream,
  pub handle: Box<dyn Fn(EventPayload) -> Result<()> + Send + Sync>,
}

impl EventSubscriber {
  pub fn get_cursor_key(&self) -> String {
    self.stream.redis_cursor_key(&self.id)
  }

  pub fn get_cursor(&self) -> Result<String> {
    let cursor: Option<String> = self.redis_pool.get()?.get(self.get_cursor_key())?;
    Ok(cursor.unwrap_or("0".to_string()))
  }

  pub fn set_cursor(&self, cursor: &str) -> Result<()> {
    self.redis_pool.get()?.set(self.get_cursor_key(), cursor)?;
    Ok(())
  }

  pub fn delete_cursor(&self) -> Result<()> {
    self.redis_pool.get()?.del(self.get_cursor_key())?;
    Ok(())
  }

  pub fn poll_stream(&self) -> Result<()> {
    let cursor = self.get_cursor()?;
    let reply: StreamReadReply = self.redis_pool.get()?.xread_options(
      &[&self.stream.redis_key()],
      &[&cursor],
      &StreamReadOptions::default().count(10).block(1000),
    )?;
    match reply.keys.get(0) {
      Some(stream) => {
        stream
          .ids
          .par_iter()
          .map(|id| {
            let payload = EventPayload::try_from(id.map.clone()).unwrap();
            (self.handle)(payload)
          })
          .collect::<Result<Vec<()>>>()?;

        let tail = stream.ids.last().unwrap().id.clone();
        self.set_cursor(&tail)?;
      }
      None => {}
    }
    Ok(())
  }

  pub fn sleep(&self) {
    thread::sleep(Duration::from_secs(5));
  }

  pub fn run(&self) {
    loop {
      match self.poll_stream() {
        Ok(_) => {}
        Err(error) => {
          println!("Error polling stream: {}", error);
          thread::sleep(std::time::Duration::from_secs(1));
        }
      }
    }
  }
}
