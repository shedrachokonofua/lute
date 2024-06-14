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

impl From<FileMetadata> for proto::FileMetadata {
  fn from(val: FileMetadata) -> Self {
    proto::FileMetadata {
      id: val.id.to_string(),
      name: val.name.to_string(),
      first_saved_at: val.first_saved_at().to_string(),
      last_saved_at: val.last_saved_at.to_string(),
    }
  }
}
