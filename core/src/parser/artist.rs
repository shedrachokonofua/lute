use super::{
  dom::HtmlParser,
  parsed_file_data::{ParsedArtist, ParsedArtistAlbum},
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use tracing::instrument;

fn parse_artist_albums(parser: &HtmlParser, id: &str) -> Result<Vec<ParsedArtistAlbum>> {
  let root = parser.get_by_id(id)?;
  let album_tags = parser.query_by_selector(&[".album"], Some(root));

  album_tags
    .into_iter()
    .map(|tag| {
      Ok(ParsedArtistAlbum {
        name: parser.get_tag_text(tag)?,
        file_name: FileName::try_from(parser.get_tag_href(tag)?)?,
      })
    })
    .collect::<Result<Vec<_>>>()
}

#[instrument(skip_all)]
pub fn parse_artist(file_content: &str) -> Result<ParsedArtist> {
  let parser = HtmlParser::try_from(file_content)?;

  let name = parser.get_meta_item_prop("name")?;
  let albums = parse_artist_albums(&parser, "disco_type_s").unwrap_or_default();
  let mixtapes = parse_artist_albums(&parser, "disco_type_m").unwrap_or_default();
  let eps = parse_artist_albums(&parser, "disco_type_e").unwrap_or_default();
  let compilations = parse_artist_albums(&parser, "disco_type_c").unwrap_or_default();
  let albums = albums
    .into_iter()
    .chain(mixtapes.into_iter())
    .chain(eps.into_iter())
    .chain(compilations.into_iter())
    .collect();

  Ok(ParsedArtist { name, albums })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_resource;

  #[test]
  fn test_artist_parser() -> Result<(), String> {
    let file_content = include_str!(test_resource!("artist.html"));
    let artist = parse_artist(file_content).map_err(|err| err.to_string())?;
    assert_eq!(artist.name, "billy woods");
    assert_eq!(artist.albums.len(), 12);
    assert_eq!(artist.albums[0].name, "Camouflage");
    assert_eq!(
      artist.albums[0].file_name,
      FileName::try_from("release/album/billy-woods-and-vordul/camouflage").unwrap()
    );
    assert_eq!(artist.albums[1].name, "The Chalice");
    assert_eq!(
      artist.albums[1].file_name,
      FileName::try_from("release/album/billy-woods/the-chalice").unwrap()
    );
    assert_eq!(artist.albums[2].name, "History Will Absolve Me");
    assert_eq!(
      artist.albums[2].file_name,
      FileName::try_from("release/album/billy-woods/history-will-absolve-me").unwrap()
    );
    assert_eq!(artist.albums[3].name, "Dour Candy");
    assert_eq!(
      artist.albums[3].file_name,
      FileName::try_from("release/album/billy-woods/dour-candy").unwrap()
    );
    assert_eq!(artist.albums[4].name, "Today, I Wrote Nothing");
    assert_eq!(
      artist.albums[4].file_name,
      FileName::try_from("release/album/billy-woods/today-i-wrote-nothing").unwrap()
    );
    assert_eq!(artist.albums[5].name, "Known Unknowns");
    assert_eq!(
      artist.albums[5].file_name,
      FileName::try_from("release/album/billy-woods/known-unknowns").unwrap()
    );
    assert_eq!(artist.albums[6].name, "Hiding Places");
    assert_eq!(
      artist.albums[6].file_name,
      FileName::try_from("release/album/billy-woods-kenny-segal/hiding-places").unwrap()
    );
    assert_eq!(artist.albums[7].name, "Terror Management");
    assert_eq!(
      artist.albums[7].file_name,
      FileName::try_from("release/album/billy-woods/terror-management").unwrap()
    );
    assert_eq!(artist.albums[8].name, "Brass");
    assert_eq!(
      artist.albums[8].file_name,
      FileName::try_from("release/album/moor-mother-billy-woods/brass").unwrap()
    );
    assert_eq!(artist.albums[9].name, "Aethiopes");
    assert_eq!(
      artist.albums[9].file_name,
      FileName::try_from("release/album/billy-woods/aethiopes").unwrap()
    );
    assert_eq!(artist.albums[10].name, "Church");
    assert_eq!(
      artist.albums[10].file_name,
      FileName::try_from("release/album/billy-woods-x-messiah-musik/church").unwrap()
    );
    assert_eq!(artist.albums[11].name, "Maps");
    assert_eq!(
      artist.albums[11].file_name,
      FileName::try_from("release/album/billy-woods-kenny-segal/maps").unwrap()
    );
    Ok(())
  }
}
