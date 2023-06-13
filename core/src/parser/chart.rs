use super::{
  dom::{get_link_tag_href, get_node_inner_text, get_tag_inner_text, query_select_first},
  parsed_file_data::{ParsedArtistReference, ParsedChartAlbum},
  util::{clean_artist_name, parse_release_date},
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::{Ok, Result};

pub fn parse_chart(file_content: &str) -> Result<Vec<ParsedChartAlbum>> {
  let dom = tl::parse(file_content, tl::ParserOptions::default())?;
  let handle = dom
    .query_selector(".page_charts_section_charts_item")
    .ok_or(anyhow::anyhow!("No charts found"))?;

  let albums: Vec<ParsedChartAlbum> = handle
    .map(|item| match item.get(dom.parser()) {
      Some(node) => {
        let tag = node.as_tag().unwrap();

        let name = get_tag_inner_text(dom.parser(), tag, ".page_charts_section_charts_item_title")?;

        let rating = get_tag_inner_text(
          dom.parser(),
          tag,
          ".page_charts_section_charts_item_details_average_num",
        )?
        .parse::<f32>()?;

        let rating_count = get_tag_inner_text(
          dom.parser(),
          query_select_first(
            dom.parser(),
            tag,
            ".page_charts_section_charts_item_details_ratings",
          )?,
          ".full",
        )?
        .replace(',', "")
        .parse::<u32>()?;

        let artists = query_select_first(
          dom.parser(),
          tag,
          ".page_charts_section_charts_item_credited_links_primary",
        )?
        .query_selector(dom.parser(), "a")
        .unwrap()
        .map(|node| ParsedArtistReference {
          name: clean_artist_name(get_node_inner_text(dom.parser(), &node).unwrap().as_str())
            .to_string(),
          file_name: FileName::try_from(
            get_link_tag_href(node.get(dom.parser()).unwrap().as_tag().unwrap()).unwrap(),
          )
          .unwrap(),
        })
        .collect::<Vec<ParsedArtistReference>>();

        let primary_genres = query_select_first(
          dom.parser(),
          tag,
          ".page_charts_section_charts_item_genres_primary",
        )?
        .query_selector(dom.parser(), "a")
        .ok_or(anyhow::anyhow!("No primary genres found"))?
        .map(|genre| get_node_inner_text(dom.parser(), &genre).unwrap())
        .collect::<Vec<String>>();

        let secondary_genres = query_select_first(
          dom.parser(),
          tag,
          ".page_charts_section_charts_item_genres_secondary",
        )
        .map(|tag| {
          tag.query_selector(dom.parser(), "a").map(|genres| {
            genres
              .map(|genre| get_node_inner_text(dom.parser(), &genre).unwrap())
              .collect::<Vec<String>>()
          })
        })
        .unwrap_or(Some(Vec::new()))
        .unwrap_or(Vec::new());

        let descriptors = query_select_first(
          dom.parser(),
          tag,
          ".page_charts_section_charts_item_genre_descriptors",
        )
        .map(|tag| {
          tag.query_selector(dom.parser(), "span").map(|descriptors| {
            descriptors
              .map(|descriptor| get_node_inner_text(dom.parser(), &descriptor).unwrap())
              .collect::<Vec<String>>()
          })
        })
        .unwrap_or(Some(Vec::new()))
        .unwrap_or(Vec::new());

        let file_name = FileName::try_from(
          get_link_tag_href(query_select_first(
            dom.parser(),
            tag,
            ".page_charts_section_charts_item_link",
          )?)
          .unwrap(),
        )?;

        let release_date_string = get_tag_inner_text(
          dom.parser(),
          query_select_first(dom.parser(), tag, ".page_charts_section_charts_item_date")?,
          "span",
        )
        .ok();

        let release_date =
          release_date_string.and_then(|date_string| parse_release_date(date_string).ok());

        let data = ParsedChartAlbum {
          file_name,
          name,
          rating,
          rating_count,
          artists,
          primary_genres,
          secondary_genres,
          descriptors,
          release_date,
        };
        Ok(data)
      }
      None => {
        println!("No node found");
        Err(anyhow::anyhow!("No node found"))
      }
    })
    .map(|res| res.unwrap())
    .collect();

  Ok(albums)
}
