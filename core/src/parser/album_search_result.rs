use crate::files::file_metadata::file_name::FileName;

use super::{
  dom::{get_link_tag_href, query_select_first},
  parsed_file_data::{ParsedAlbumSearchResult, ParsedArtistReference},
  util::clean_artist_name,
};
use anyhow::Result;
use tracing::{instrument, warn};

#[instrument(skip(file_content))]
pub fn parse_album_search_result(file_content: &str) -> Result<ParsedAlbumSearchResult> {
  let dom = tl::parse(file_content, tl::ParserOptions::default())?;

  let results = dom
    .query_selector(".infobox")
    .ok_or(anyhow::anyhow!("No search results found"))?
    .map(|node| -> Result<ParsedAlbumSearchResult> {
      let tag = node
        .get(dom.parser())
        .and_then(|node| node.as_tag())
        .ok_or(anyhow::anyhow!("Failed to get tag for search result"))?;

      let item = query_select_first(dom.parser(), tag, ".searchpage")?;
      let name = item.inner_text(dom.parser()).trim().to_string();
      let file_name = FileName::try_from(get_link_tag_href(item)?)?;
      let artists = tag
        .query_selector(dom.parser(), ".artist")
        .unwrap()
        .map(|node| {
          let tag = node
            .get(dom.parser())
            .and_then(|node| node.as_tag())
            .unwrap();
          let name =
            clean_artist_name(tag.inner_text(dom.parser()).to_string().as_str()).to_string();
          let file_name = FileName::try_from(get_link_tag_href(tag).unwrap()).unwrap();

          ParsedArtistReference { name, file_name }
        })
        .collect::<Vec<ParsedArtistReference>>();

      Ok(ParsedAlbumSearchResult {
        name,
        file_name,
        artists,
      })
    })
    .filter_map(|result| match result {
      Ok(result) => Some(result),
      Err(err) => {
        warn!(err = err.to_string(), "Failed to parse search result");
        None
      }
    })
    .collect::<Vec<ParsedAlbumSearchResult>>();

  results
    .iter()
    .find(|result| result.file_name.page_type().is_album())
    .ok_or(anyhow::anyhow!("No album found in search results"))
    .cloned()
}
