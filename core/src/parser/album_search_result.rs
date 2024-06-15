use super::{
  dom::HtmlParser,
  parsed_file_data::{ParsedAlbumSearchResult, ParsedArtistReference},
  util::clean_artist_name,
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use tracing::{instrument, warn};

#[instrument(skip(file_content))]
pub fn parse_album_search_result(file_content: &str) -> Result<ParsedAlbumSearchResult> {
  let parser = HtmlParser::try_from(file_content)?;

  let results = parser
    .query_by_selector(&[".infobox"], None)
    .into_iter()
    .filter_map(|tag| {
      let name_tag = parser.find_by_selector(&[".searchpage"], Some(tag))?;
      let name = parser.find_tag_text(name_tag)?;
      let file_name = FileName::try_from(parser.find_tag_href(name_tag)?)
        .map_err(|err| {
          warn!(err = err.to_string(), "Failed to parse album search result");
          err
        })
        .ok()?;
      let artists = parser
        .query_by_selector(&[".artist"], Some(tag))
        .into_iter()
        .filter_map(|tag| {
          let name_text = parser.find_tag_text(tag)?;
          let name = clean_artist_name(&name_text).to_string();
          let file_name = FileName::try_from(parser.find_tag_href(tag)?)
            .map_err(|err| {
              warn!(err = err.to_string(), "Failed to parse artist reference");
              err
            })
            .ok()?;
          Some(ParsedArtistReference { name, file_name })
        })
        .collect::<Vec<_>>();

      Some(ParsedAlbumSearchResult {
        name,
        file_name,
        artists,
      })
    })
    .collect::<Vec<_>>();

  results
    .into_iter()
    .find(|result| result.file_name.page_type().is_album())
    .ok_or(anyhow::anyhow!("No album found in search results"))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::files::file_metadata::file_name::FileName;
  use crate::test_resource;

  #[test]
  fn test_parse_album_search_result() -> Result<(), String> {
    let file_content = include_str!(test_resource!("album_search_result.html"));
    let search_result = parse_album_search_result(&file_content).unwrap();
    assert_eq!(search_result.name, "Unknown Soldier");
    assert_eq!(
      search_result.file_name,
      FileName::try_from("release/album/fela-anikulapo-kuti-and-his-africa-70/unknown-soldier")
        .unwrap()
    );
    assert_eq!(search_result.artists.len(), 2);
    assert_eq!(search_result.artists[0].name, "Fela Kuti");
    assert_eq!(
      search_result.artists[0].file_name,
      FileName::try_from("artist/fela-kuti").unwrap()
    );
    assert_eq!(search_result.artists[1].name, "The Africa '70");
    assert_eq!(
      search_result.artists[1].file_name,
      FileName::try_from("artist/the-africa-70").unwrap()
    );
    Ok(())
  }
}
