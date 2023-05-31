use super::event::{Event, EventTag};

pub struct EventStream {}

pub fn get_stream_key(event: &Event) -> String {
  let event_tag = EventTag::from(event);
  format!("stream:{}", event_tag.to_string())
}
