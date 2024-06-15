use super::{
  parsed_file_data::{ParsedAlbum, ParsedArtistReference, ParsedCredit, ParsedTrack},
  util::{clean_album_name, clean_artist_name, parse_release_date},
};
use crate::{files::file_metadata::file_name::FileName, parser::dom::HtmlParser};
use anyhow::Result;
use serde_json::Value;
use tracing::{instrument, warn};

#[instrument(skip_all)]
pub fn parse_album(file_content: &str) -> Result<ParsedAlbum> {
  let parser = HtmlParser::try_from(file_content)?;

  let name = clean_album_name(parser.get_meta_item_prop("name")?);
  let rating = parser
    .get_meta_item_prop("ratingValue")?
    .parse::<f32>()
    .unwrap_or_default();
  let rating_count = parser
    .get_meta_item_prop("ratingCount")?
    .parse::<u32>()
    .unwrap_or_default();

  let spotify_id = parser
    .find_by_id("media_link_button_container_top")
    .and_then(|tag| parser.find_tag_attribute_value(tag, "data-links"))
    .and_then(|val| {
      serde_json::from_str::<Value>(&val)
        .inspect_err(|err| {
          warn!("Failed to parse data-links: {}", err);
        })
        .ok()
    })
    .and_then(|val| {
      val["spotify"]
        .as_object()?
        .iter()
        .find_map(|(id, attributes)| {
          if attributes.get("default")?.as_bool()? {
            Some(id.clone())
          } else {
            None
          }
        })
    });

  let release_date = parser
    .find_attribute_value(&[".issue_year.ymd"], "title", None)
    .and_then(|release_date_string| {
      parse_release_date(release_date_string)
        .inspect_err(|err| {
          warn!("Failed to parse release date: {}", err);
        })
        .ok()
    });

  let info_container = parser.get_by_selector(&[".release_page"], None)?;

  let cover_image_url = parser
    .find_by_selector(&[".page_release_art_frame", "img"], Some(info_container))
    .and_then(|tag| parser.find_tag_attribute_value(tag, "src"))
    .map(|url| format!("https:{}", url));

  let artists = parser
    .query_by_selector(&["span[itemprop='byArtist']", "a"], Some(info_container))
    .into_iter()
    .map(|tag| {
      let name_text = parser.get_tag_text(tag)?;
      let href = parser.get_tag_href(tag)?;
      let file_name = FileName::try_from(href)?;
      Ok(ParsedArtistReference {
        name: clean_artist_name(&name_text).to_string(),
        file_name,
      })
    })
    .collect::<Result<Vec<_>>>()?;

  let primary_genres = parser
    .query_by_selector(&[".release_pri_genres", ".genre"], Some(info_container))
    .into_iter()
    .map(|tag| parser.get_tag_text(tag))
    .collect::<Result<Vec<_>>>()?;

  let secondary_genres = parser
    .query_by_selector(&[".release_sec_genres", ".genre"], Some(info_container))
    .into_iter()
    .map(|tag| parser.get_tag_text(tag))
    .collect::<Result<Vec<_>>>()
    .unwrap_or_default();

  let descriptors = parser
    .query_by_selector(&[".release_descriptors", "meta"], Some(info_container))
    .into_iter()
    .filter_map(|tag| parser.find_tag_attribute_value(tag, "content"))
    .collect::<Vec<_>>();

  let languages = parser
    .query_by_selector(&[".album_info", "tr"], Some(info_container))
    .into_iter()
    .filter_map(|tag| {
      let title = parser.find_text(&["th"], Some(tag))?;
      let value = parser.find_text(&["td"], Some(tag))?;
      if title == "Language" {
        Some(vec![value])
      } else if title == "Languages" {
        Some(
          value
            .replace(' ', "")
            .split(',')
            .map(|s| s.to_string())
            .collect(),
        )
      } else {
        None
      }
    })
    .flatten()
    .collect::<Vec<_>>();

  let tracks = parser
    .find_by_id("tracks")
    .map(|root| {
      parser
        .query_by_selector(&[".tracklist_line"], Some(root))
        .into_iter()
        .map(|tag| {
          let name = parser.get_text(&[".rendered_text"], Some(tag))?;
          let rating = parser
            .find_text(&[".track_rating_avg"], Some(tag))
            .and_then(|rating| {
              rating
                .parse::<f32>()
                .inspect_err(|err| {
                  warn!("Failed to parse rating: {}", err);
                })
                .ok()
            });
          let position = parser.find_text(&[".tracklist_num"], Some(tag));
          let duration_seconds = parser
            .find_by_selector(&[".tracklist_duration", "meta"], Some(tag))
            .and_then(|tag| parser.find_tag_attribute_value(tag, "data-inseconds"))
            .and_then(|val| {
              val
                .parse::<u32>()
                .inspect_err(|err| {
                  warn!("Failed to parse duration seconds: {}", err);
                })
                .ok()
            });
          Ok(ParsedTrack {
            name,
            rating,
            position,
            duration_seconds,
          })
        })
        .collect::<Result<Vec<_>>>()
        .unwrap_or_default()
    })
    .unwrap_or_default();

  let credits = parser
    .find_by_id("credits_")
    .map(|root| {
      parser
        .query_by_selector(&["li"], Some(root))
        .into_iter()
        .filter_map(|tag| {
          let artist = parser
            .find_by_selector(&["a"], Some(tag))
            .map(|tag| {
              let name = parser.get_tag_text(tag)?;
              let href = parser.get_tag_href(tag)?;
              let file_name = FileName::try_from(href)?;
              Ok::<_, anyhow::Error>(ParsedArtistReference {
                name: clean_artist_name(&name).to_string(),
                file_name,
              })
            })
            .transpose()
            .ok()?;
          let roles = parser
            .query_by_selector(&[".role_name"], Some(tag))
            .into_iter()
            .map(|tag| {
              let mut text = parser.get_tag_text(tag)?;
              if let Some(role_tracks) = parser.find_text(&[".role_tracks"], Some(tag)) {
                text = text.replace(&role_tracks, "");
              }
              Ok(text)
            })
            .collect::<Result<Vec<_>>>()
            .unwrap_or_default();
          artist.map(|artist| ParsedCredit { artist, roles })
        })
        .collect::<Vec<_>>()
    })
    .unwrap_or_default();

  Ok(ParsedAlbum {
    name,
    rating,
    rating_count,
    release_date,
    artists,
    primary_genres,
    secondary_genres,
    descriptors,
    tracks,
    languages,
    credits,
    cover_image_url,
    spotify_id,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_resource;
  use chrono::NaiveDate;

  #[test]
  fn test_album_parser() -> Result<(), String> {
    let file_content = include_str!(test_resource!("album.html"));
    let album = parse_album(file_content).map_err(|err| err.to_string())?;
    assert_eq!(album.name, "Gentleman");
    assert_eq!(album.rating, 3.91);
    assert_eq!(album.rating_count, 3692);
    assert_eq!(album.spotify_id, Some("532y6THMUtDYbRQAvzP1bL".to_string()));
    assert_eq!(album.artists.len(), 2);
    assert_eq!(album.artists[0].name, "Fela Kuti");
    assert_eq!(album.artists[0].file_name.to_string(), "artist/fela-kuti");
    assert_eq!(album.artists[1].name, "The Africa '70");
    assert_eq!(
      album.artists[1].file_name.to_string(),
      "artist/the-africa-70"
    );
    assert_eq!(album.primary_genres, ["Afrobeat"]);
    assert_eq!(album.secondary_genres, ["Jazz-Funk"]);
    assert_eq!(album.descriptors.len(), 12);
    assert_eq!(album.tracks.len(), 3);
    assert_eq!(album.tracks[0].name, "Gentleman");
    assert_eq!(album.tracks[0].duration_seconds, None);
    assert_eq!(album.tracks[0].rating, None);
    assert_eq!(album.tracks[0].position, Some("A".to_string()));
    assert_eq!(album.tracks[1].name, "Igbe (Na Shit)");
    assert_eq!(album.tracks[1].duration_seconds, None);
    assert_eq!(album.tracks[1].rating, None);
    assert_eq!(album.tracks[1].position, Some("B1".to_string()));
    assert_eq!(album.tracks[2].name, "Fefe Naa Efe");
    assert_eq!(album.tracks[2].duration_seconds, None);
    assert_eq!(album.tracks[2].rating, None);
    assert_eq!(album.tracks[2].position, Some("B2".to_string()));
    assert!(album.release_date.is_some());
    assert_eq!(album.release_date, NaiveDate::from_ymd_opt(2020, 6, 26));
    assert_eq!(album.languages, ["English", "Yoruba"]);
    assert_eq!(album.credits.len(), 6);
    assert_eq!(album.credits[0].artist.name, "Fela Ransome Kuti");
    assert_eq!(
      album.credits[0].artist.file_name.to_string(),
      "artist/fela-kuti"
    );
    assert_eq!(
      album.credits[0].roles,
      [
        "tenor saxophone",
        "alto saxophone",
        "electric piano",
        "vocals",
        "writer",
        "arranger",
        "producer"
      ]
    );
    assert_eq!(album.credits[1].artist.name, "Tunde Williams");
    assert_eq!(
      album.credits[1].artist.file_name.to_string(),
      "artist/tunde_williams"
    );
    assert_eq!(album.credits[1].roles, ["trumpet"]);
    assert_eq!(album.credits[2].artist.name, "Emmanuel Odenisi");
    assert_eq!(
      album.credits[2].artist.file_name.to_string(),
      "artist/emmanuel_a__odenusi"
    );
    assert_eq!(
      album.credits[2].roles,
      ["recording engineer", "mixing engineer"]
    );
    assert_eq!(album.credits[3].artist.name, "The Africa '70");
    assert_eq!(
      album.credits[3].artist.file_name.to_string(),
      "artist/the-africa-70"
    );
    assert_eq!(album.credits[3].roles, ["performer"]);
    assert_eq!(album.credits[4].artist.name, "Remi Olowookere");
    assert_eq!(
      album.credits[4].artist.file_name.to_string(),
      "artist/remi-olowookere"
    );
    assert_eq!(album.credits[4].roles, ["graphic design", "art direction"]);
    assert_eq!(album.credits[5].artist.name, "Igo Chico");
    assert_eq!(
      album.credits[5].artist.file_name.to_string(),
      "artist/igo-chico"
    );
    assert_eq!(album.credits[5].roles, ["tenor saxophone"]);
    assert!(album.cover_image_url.is_some());
    assert_eq!(album.cover_image_url.unwrap(), "https://e.snmc.io/i/600/w/5f531a5819eda8ce114ffdb1e2359148/1346423/fela-ransome-kuti-and-the-afrika-70-gentleman-Cover-Art.jpg");
    Ok(())
  }
}
