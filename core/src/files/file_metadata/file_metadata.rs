use crate::proto;

use super::{file_name::FileName, file_timestamp::FileTimestamp, page_type::PageType};
use ulid::Ulid;

#[derive(Default, Clone)]
pub struct FileMetadata {
  pub id: Ulid,
  pub name: FileName,
  pub last_saved_at: FileTimestamp,
}

impl FileMetadata {
  pub fn first_saved_at(&self) -> FileTimestamp {
    self.id.datetime().into()
  }

  pub fn page_type(&self) -> PageType {
    self.name.page_type()
  }
}

impl Into<proto::FileMetadata> for FileMetadata {
  fn into(self) -> proto::FileMetadata {
    proto::FileMetadata {
      id: self.id.to_string(),
      name: self.name.0.clone(),
      first_saved_at: self.first_saved_at().to_string(),
      last_saved_at: self.last_saved_at.to_string(),
    }
  }
}
