use super::{
  dom::{get_node_inner_text, get_tag_inner_text, query_select_first},
  parsed_file_data::ParsedChartAlbum,
  util::parse_release_date,
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
        .replace(",", "")
        .parse::<u32>()?;

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
        .and_then(|tag| {
          Ok(tag.query_selector(dom.parser(), "a").map(|genres| {
            genres
              .map(|genre| get_node_inner_text(dom.parser(), &genre).unwrap())
              .collect::<Vec<String>>()
          }))
        })
        .unwrap_or_else(|e| Some(Vec::new()))
        .unwrap_or_else(|| Vec::new());

        let descriptors = query_select_first(
          dom.parser(),
          tag,
          ".page_charts_section_charts_item_genre_descriptors",
        )?
        .query_selector(dom.parser(), "span")
        .ok_or(anyhow::anyhow!("No descriptor found"))?
        .map(|descriptor| get_node_inner_text(dom.parser(), &descriptor).unwrap())
        .collect::<Vec<String>>();

        let file_name = FileName::try_from(
          query_select_first(dom.parser(), tag, ".page_charts_section_charts_item_link")?
            .attributes()
            .get("href")
            .unwrap()
            .ok_or(anyhow::anyhow!("No file name found"))?
            .as_utf8_str()
            .to_string(),
        )?;

        let release_date_string = get_tag_inner_text(
          dom.parser(),
          query_select_first(dom.parser(), tag, ".page_charts_section_charts_item_date")?,
          "span",
        )
        .ok();

        let release_date = release_date_string
          .map(|date_string| parse_release_date(date_string))
          .unwrap_or(Ok(None))?;

        let data = ParsedChartAlbum {
          file_name,
          name,
          rating,
          rating_count,
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
