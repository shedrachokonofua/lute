use super::{
  dom::{
    get_link_tag_href, get_meta_value, get_node_inner_text, get_tag_inner_text, query_select_first,
  },
  parsed_file_data::{ParsedAlbum, ParsedArtistReference, ParsedCredit, ParsedTrack},
  util::{clean_artist_name, parse_release_date},
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::{Error, Result};
use tracing::{instrument, warn};

#[instrument(skip(file_content))]
pub fn parse_album(file_content: &str) -> Result<ParsedAlbum> {
  let dom = tl::parse(file_content, tl::ParserOptions::default())?;

  let name = get_meta_value(&dom, "name")?;

  let rating = get_meta_value(&dom, "ratingValue").and_then(|rating| {
    rating
      .parse::<f32>()
      .map_err(|err| anyhow::anyhow!("Failed to parse rating: {}", err))
  })?;

  let rating_count = get_meta_value(&dom, "ratingCount").and_then(|rating_count| {
    rating_count
      .parse::<u32>()
      .map_err(|err| anyhow::anyhow!("Failed to parse rating count: {}", err))
  })?;

  let release_date = dom
    .query_selector(".issue_year.ymd")
    .and_then(|mut iter| iter.next())
    .and_then(|node| node.get(dom.parser()))
    .and_then(|node| node.as_tag())
    .and_then(|tag| tag.attributes().get("title"))
    .flatten()
    .map(|content| content.as_utf8_str())
    .map(|name| name.to_string())
    .and_then(|release_date_string| parse_release_date(release_date_string).ok());

  let container = dom
    .query_selector(".release_page")
    .and_then(|mut iter| iter.next())
    .and_then(|node| node.get(dom.parser()))
    .and_then(|node| node.as_tag())
    .ok_or(anyhow::anyhow!("No release page container found"))?;

  let artists = query_select_first(dom.parser(), container, "span[itemprop='byArtist']")?
    .query_selector(dom.parser(), "a")
    .map(|iter| {
      iter
        .map(|node| {
          let tag = node
            .get(dom.parser())
            .and_then(|node| node.as_tag())
            .unwrap();

          ParsedArtistReference {
            name: clean_artist_name(get_node_inner_text(dom.parser(), &node).unwrap().as_str())
              .to_string(),
            file_name: FileName::try_from(get_link_tag_href(tag).unwrap()).unwrap(),
          }
        })
        .collect::<Vec<ParsedArtistReference>>()
    })
    .ok_or(anyhow::anyhow!("Failed to parse artists"))?;

  let primary_genres = query_select_first(dom.parser(), container, ".release_pri_genres")?
    .query_selector(dom.parser(), ".genre")
    .map(|iter| {
      iter
        .map(|node| get_node_inner_text(dom.parser(), &node).unwrap())
        .collect::<Vec<String>>()
    })
    .ok_or(anyhow::anyhow!("Failed to parse primary genres"))?;

  let secondary_genres = match query_select_first(dom.parser(), container, ".release_sec_genres") {
    Ok(node) => node
      .query_selector(dom.parser(), ".genre")
      .map(|iter| {
        iter
          .map(|node| get_node_inner_text(dom.parser(), &node).unwrap())
          .collect::<Vec<String>>()
      })
      .ok_or(anyhow::anyhow!("Failed to parse secondary genres"))?,
    Err(_) => vec![],
  };

  let descriptors = query_select_first(dom.parser(), container, ".release_descriptors")?
    .query_selector(dom.parser(), "meta")
    .map(|iter| {
      iter
        .map(|node| {
          node
            .get(dom.parser())
            .and_then(|node| node.as_tag())
            .and_then(|tag| tag.attributes().get("content"))
            .flatten()
            .map(|content| content.as_utf8_str())
            .map(|name| name.to_string().trim().to_string())
            .unwrap()
        })
        .collect::<Vec<String>>()
    })
    .ok_or(anyhow::anyhow!("Failed to parse descriptors"))?;

  let languages = query_select_first(dom.parser(), container, ".album_info")?
    .query_selector(dom.parser(), "tr")
    .map(|iter| {
      iter
        .filter_map(|node| {
          let tag = node
            .get(dom.parser())
            .and_then(|node| node.as_tag())
            .unwrap();

          let row_key = get_tag_inner_text(dom.parser(), tag, "th").unwrap_or_else(|err| {
            warn!("Failed to parse row key: {}", err);
            "".to_string()
          });
          if row_key == "Language" {
            let row_value = get_tag_inner_text(dom.parser(), tag, "td").unwrap();
            Some(vec![row_value])
          } else if row_key == "Languages" {
            let row_value = get_tag_inner_text(dom.parser(), tag, "td").unwrap();
            Some(
              row_value
                .replace(" ", "")
                .split(",")
                .map(|s| s.to_string())
                .collect(),
            )
          } else {
            None
          }
        })
        .flatten()
        .collect::<Vec<String>>()
    })
    .unwrap_or(vec![]);

  let tracks = query_select_first(dom.parser(), container, "#tracks")?
    .query_selector(dom.parser(), ".tracklist_line")
    .map(|iter| {
      iter
        .map(|node| {
          let tag = node
            .get(dom.parser())
            .and_then(|node| node.as_tag())
            .unwrap();

          let name = get_tag_inner_text(dom.parser(), tag, ".rendered_text").unwrap();

          let rating = get_tag_inner_text(dom.parser(), tag, ".track_rating_avg")
            .ok()
            .and_then(|rating| rating.parse::<f32>().ok());

          let position = get_tag_inner_text(dom.parser(), tag, ".tracklist_num").ok();

          let duration_seconds = tag
            .query_selector(dom.parser(), ".tracklist_duration")
            .and_then(|mut iter| iter.next())
            .and_then(|node| node.get(dom.parser()))
            .and_then(|node| node.as_tag())
            .and_then(|tag| tag.attributes().get("data-inseconds"))
            .flatten()
            .map(|content| content.as_utf8_str())
            .map(|name| name.to_string().parse::<u32>().unwrap());

          ParsedTrack {
            name,
            rating,
            duration_seconds,
            position,
          }
        })
        .collect::<Vec<ParsedTrack>>()
    })
    .ok_or(anyhow::anyhow!("Failed to parse tracks"))?;

  let credits = match query_select_first(dom.parser(), container, "#credits_") {
    Ok(tag) => tag
      .query_selector(dom.parser(), "li")
      .map(|iter| {
        iter
          .map(|node| {
            let tag = node.get(dom.parser()).and_then(|node| node.as_tag());
            if tag.is_none() {
              return None;
            }
            let tag = tag.unwrap();

            let artist = tag
              .query_selector(dom.parser(), "a")
              .and_then(|mut iter| iter.next())
              .and_then(|node| node.get(dom.parser()))
              .and_then(|node| node.as_tag())
              .map(|tag| ParsedArtistReference {
                name: clean_artist_name(tag.inner_text(dom.parser()).as_ref()).to_string(),
                file_name: FileName::try_from(get_link_tag_href(tag).unwrap()).unwrap(),
              });
            if artist.is_none() {
              return None;
            }
            let artist = artist.unwrap();

            let roles = tag
              .query_selector(dom.parser(), ".role_name")
              .map(|iter| {
                iter
                  .map(|node| {
                    let text = get_node_inner_text(dom.parser(), &node)?;
                    let tag = node.get(dom.parser()).and_then(|node| node.as_tag());
                    if tag.is_none() {
                      return Ok::<String, Error>(text);
                    }
                    let tag = tag.unwrap();
                    let role_tracks = get_tag_inner_text(dom.parser(), tag, ".role_tracks");
                    if role_tracks.is_err() {
                      return Ok(text);
                    }
                    let role_tracks = role_tracks.unwrap();
                    Ok(text.replace(&role_tracks, ""))
                  })
                  .filter_map(|role: Result<String, _>| role.ok())
                  .collect::<Vec<String>>()
              })
              .unwrap_or(vec![]);

            Some(ParsedCredit { artist, roles })
          })
          .filter_map(|credit| credit)
          .collect::<Vec<ParsedCredit>>()
      })
      .unwrap_or(vec![]),
    Err(_) => vec![],
  };

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
  })
}
