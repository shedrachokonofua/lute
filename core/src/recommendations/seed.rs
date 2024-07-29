use crate::{
  albums::album_read_model::AlbumReadModel, files::file_metadata::file_name::FileName,
  profile::profile::ProfileId,
};
use std::collections::HashMap;

pub enum AlbumRecommendationSeed {
  Profile(ProfileId),
  Albums(HashMap<FileName, u32>),
}

pub struct AlbumRecommendationSeedContext {
  pub albums: Vec<AlbumReadModel>,
  pub factor_map: HashMap<FileName, u32>,
}

impl AlbumRecommendationSeedContext {
  pub fn new(albums: Vec<AlbumReadModel>, factor_map: HashMap<FileName, u32>) -> Self {
    Self { albums, factor_map }
  }

  pub fn album_file_names(&self) -> Vec<FileName> {
    self
      .albums
      .iter()
      .map(|album| album.file_name.clone())
      .collect()
  }

  pub fn get_factor(&self, file_name: &FileName) -> Option<u32> {
    self.factor_map.get(file_name).copied()
  }
}
