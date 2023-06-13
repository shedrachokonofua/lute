use super::{
  dom::{self, get_link_tag_href, get_meta_value},
  parsed_file_data::{ParsedArtist, ParsedArtistAlbum},
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use tl::VDom;

fn parse_artist_album(dom: &VDom, selector: &str) -> Result<Vec<ParsedArtistAlbum>> {
  dom
    .query_selector(selector)
    .and_then(|mut iter| iter.next())
    .and_then(|node| node.get(dom.parser()))
    .and_then(|node| node.as_tag())
    .and_then(|tag| tag.query_selector(dom.parser(), ".album"))
    .ok_or(anyhow::anyhow!("No artist albums found"))
    .map(|iter| {
      iter
        .map(|node| {
          let tag = node
            .get(dom.parser())
            .and_then(|node| node.as_tag())
            .unwrap();

          ParsedArtistAlbum {
            name: dom::get_node_inner_text(dom.parser(), &node)
              .unwrap()
              ,
            file_name: FileName::try_from(get_link_tag_href(tag).unwrap()).unwrap(),
          }
        })
        .collect()
    })
}

pub fn parse_artist(file_content: &str) -> Result<ParsedArtist> {
  let dom = tl::parse(file_content, tl::ParserOptions::default())?;
  let name = get_meta_value(&dom, "name")?;
  let albums = parse_artist_album(&dom, "#disco_type_s").unwrap_or(vec![]);
  let mixtapes = parse_artist_album(&dom, "#disco_type_m").unwrap_or(vec![]);
  let eps = parse_artist_album(&dom, "#disco_type_e").unwrap_or(vec![]);
  let albums = albums
    .into_iter()
    .chain(mixtapes.into_iter())
    .chain(eps.into_iter())
    .collect();

  Ok(ParsedArtist { name, albums })
}
