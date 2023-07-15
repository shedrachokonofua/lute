use super::profile::ProfileId;
use crate::{
  lookup::album_search_lookup::AlbumSearchLookupQuery,
  spotify::spotify_client::{SpotifyAlbumType, SpotifyTrack},
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PartialEq, Serialize, Deserialize, Clone)]
pub struct SpotifyImportLookupSubscription {
  pub album_search_lookup_encoded_query: String,
  pub album_search_lookup_query: AlbumSearchLookupQuery,
  pub profile_id: ProfileId,
  pub factor: u32,
}

pub fn build_spotify_import_lookup_subscriptions(
  profile_id: &ProfileId,
  spotify_tracks: Vec<SpotifyTrack>,
) -> Vec<SpotifyImportLookupSubscription> {
  let mut subscriptions: HashMap<AlbumSearchLookupQuery, SpotifyImportLookupSubscription> =
    HashMap::new();
  for track in spotify_tracks {
    if track.album.album_type == SpotifyAlbumType::Album {
      let query = AlbumSearchLookupQuery::new(
        track.album.name,
        track.artists.get(0).expect("artist not found").name.clone(),
      );
      let subscription = subscriptions.get(&query);
      if subscription.is_none() {
        subscriptions.insert(
          query.clone(),
          SpotifyImportLookupSubscription {
            album_search_lookup_encoded_query: query.to_encoded_string(),
            album_search_lookup_query: query.clone(),
            profile_id: profile_id.clone(),
            factor: 1,
          },
        );
      } else {
        let mut subscription = subscription.unwrap().clone();
        subscription.factor += 1;
        subscriptions.insert(query, subscription);
      }
    }
  }
  subscriptions.into_iter().map(|(_, v)| v).collect()
}
