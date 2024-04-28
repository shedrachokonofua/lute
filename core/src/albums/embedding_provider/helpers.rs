use crate::albums::album_read_model::AlbumReadModel;
use sha2::{Digest, Sha256};

pub fn get_document_id(content: String) -> String {
  let mut hasher = Sha256::new();
  hasher.update(content);
  let result = hasher.finalize();
  format!("{:x}", result)
}

pub fn get_embedding_api_input(album: &AlbumReadModel) -> (String, String) {
  let mut body = vec![];
  body.push(album.rating.to_string());
  body.push(album.rating_count.to_string());
  if let Some(release_date) = album.release_date {
    body.push(release_date.to_string());
  }
  body.extend(album.artists.clone().into_iter().map(|artist| artist.name));
  body.extend(album.primary_genres.clone());
  body.extend(album.secondary_genres.clone());
  body.extend(album.descriptors.clone());
  body.extend(album.languages.clone());
  body.extend(album.credits.clone().into_iter().map(|c| c.artist.name));
  let body = body.join(", ");
  let id = get_document_id(body.clone());
  (id, body)
}
