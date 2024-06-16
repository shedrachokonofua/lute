use super::parsed_file_data::ParsedListSegment;
use crate::{files::file_metadata::file_name::FileName, parser::dom::HtmlParser};
use anyhow::Result;
use tracing::{instrument, warn};

#[instrument(skip_all)]
pub fn parse_list_segment(file_content: &str) -> Result<ParsedListSegment> {
  let parser = HtmlParser::try_from(file_content)?;
  let name = parser.get_text(&["h1"], None)?;

  let top_pagination_bar = parser.get_by_selector(&[".navspan"], None)?;
  let other_segments = parser
    .query_by_selector(&[".navlinknum"], Some(top_pagination_bar))
    .into_iter()
    .filter_map(|tag| {
      let href = parser.find_tag_href(tag)?;
      FileName::try_from(href)
        .inspect_err(|err| warn!("Failed to parse file name for list segment: {}", err))
        .ok()
    })
    .collect::<Vec<_>>();

  let albums = parser
    .query_by_selector(&[".list_album"], None)
    .into_iter()
    .filter_map(|tag| {
      let href = parser.find_tag_href(tag)?;
      FileName::try_from(href)
        .inspect_err(|err| warn!("Failed to parse file name for list segment: {}", err))
        .ok()
    })
    .collect::<Vec<_>>();

  Ok(ParsedListSegment {
    name,
    other_segments,
    albums,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_resource;

  #[test]
  fn test_parse_list_segment() -> Result<(), String> {
    let file_content = include_str!(test_resource!("list_segment.html"));
    let segment = parse_list_segment(file_content).map_err(|err| err.to_string())?;
    assert_eq!(segment.name, "theneedledrop's Top 200 Albums of the 2010's");
    assert_eq!(segment.other_segments.len(), 1);
    assert_eq!(
      segment.other_segments[0],
      FileName::try_from("list/Seab/theneedledrops-top-200-albums-of-the-2010s/1").unwrap()
    );

    assert_eq!(segment.albums.len(), 11);
    assert_eq!(
      segment.albums[0],
      FileName::try_from("release/album/daft-punk/random-access-memories").unwrap()
    );
    assert_eq!(
      segment.albums[1],
      FileName::try_from("release/album/bjork/vulnicura").unwrap()
    );
    assert_eq!(
      segment.albums[2],
      FileName::try_from("release/album/run-the-jewels/run-the-jewels-2").unwrap()
    );
    assert_eq!(
      segment.albums[3],
      FileName::try_from("release/album/regina-spektor/remember-us-to-life").unwrap()
    );
    assert_eq!(
      segment.albums[4],
      FileName::try_from("release/album/fka-twigs/lp1").unwrap()
    );
    assert_eq!(
      segment.albums[5],
      FileName::try_from("release/album/krallice/years-past-matter-3").unwrap()
    );
    assert_eq!(
      segment.albums[6],
      FileName::try_from("release/album/radiohead/a-moon-shaped-pool").unwrap()
    );
    assert_eq!(
      segment.albums[7],
      FileName::try_from("release/album/crying/beyond-the-fleeting-gales").unwrap()
    );
    assert_eq!(
      segment.albums[8],
      FileName::try_from("release/album/yg/still-brazy").unwrap()
    );
    assert_eq!(
      segment.albums[9],
      FileName::try_from("release/album/perfume-genius/put-your-back-n-2-it").unwrap()
    );
    assert_eq!(
      segment.albums[10],
      FileName::try_from("release/album/prurient/frozen-niagara-falls").unwrap()
    );

    Ok(())
  }
}
