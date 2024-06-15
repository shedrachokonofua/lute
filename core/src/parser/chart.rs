use super::{
  dom::HtmlParser,
  parsed_file_data::{ParsedArtistReference, ParsedChartAlbum},
  util::{clean_artist_name, parse_release_date},
};
use crate::files::file_metadata::file_name::FileName;
use anyhow::Result;
use tracing::{instrument, warn};

#[instrument(skip(file_content))]
pub fn parse_chart(file_content: &str) -> Result<Vec<ParsedChartAlbum>> {
  let parser = HtmlParser::try_from(file_content)?;
  let albums = parser
    .query_by_selector(&[".page_charts_section_charts_item"], None)
    .into_iter()
    .filter_map(|tag| {
      let name = parser.find_text(&[".page_charts_section_charts_item_title"], Some(tag))?;
      let rating = parser
        .find_text(
          &[".page_charts_section_charts_item_details_average_num"],
          Some(tag),
        )?
        .parse::<f32>()
        .inspect_err(|err| {
          warn!(err = err.to_string(), "Failed to parse rating");
        })
        .ok()?;
      let rating_count = parser
        .find_text(
          &[".page_charts_section_charts_item_details_ratings", ".full"],
          Some(tag),
        )?
        .replace(',', "")
        .parse::<u32>()
        .inspect_err(|err| {
          warn!(err = err.to_string(), "Failed to parse rating count");
        })
        .ok()?;
      let artists = parser
        .query_by_selector(
          &[
            ".page_charts_section_charts_item_credited_links_primary",
            "a",
          ],
          Some(tag),
        )
        .into_iter()
        .filter_map(|tag| {
          let name = clean_artist_name(&parser.find_tag_text(tag)?).to_string();
          let file_name = FileName::try_from(parser.find_tag_href(tag)?)
            .map_err(|err| {
              warn!(err = err.to_string(), "Failed to parse artist reference");
              err
            })
            .ok()?;
          Some(ParsedArtistReference { name, file_name })
        })
        .collect::<Vec<_>>();
      let primary_genres = parser
        .query_by_selector(
          &[".page_charts_section_charts_item_genres_primary", "a"],
          Some(tag),
        )
        .into_iter()
        .map(|tag| parser.find_tag_text(tag))
        .collect::<Option<Vec<_>>>()?;
      let secondary_genres = parser
        .query_by_selector(
          &[".page_charts_section_charts_item_genres_secondary", "a"],
          Some(tag),
        )
        .into_iter()
        .filter_map(|tag| parser.find_tag_text(tag))
        .collect::<Vec<_>>();
      let descriptors = parser
        .query_by_selector(
          &[".page_charts_section_charts_item_genre_descriptors", "span"],
          Some(tag),
        )
        .into_iter()
        .filter_map(|tag| parser.find_tag_text(tag))
        .collect::<Vec<_>>();
      let file_name = FileName::try_from(
        parser
          .find_by_selector(&[".page_charts_section_charts_item_link"], Some(tag))
          .and_then(|tag| parser.find_tag_href(tag))?,
      )
      .map_err(|err| {
        warn!(err = err.to_string(), "Failed to parse album reference");
        err
      })
      .ok()?;
      let release_date = parser
        .find_text(
          &[".page_charts_section_charts_item_date", "span"],
          Some(tag),
        )
        .and_then(|date_string| {
          parse_release_date(date_string)
            .inspect_err(|err| {
              warn!(err = err.to_string(), "Failed to parse release date");
            })
            .ok()
        });

      Some(ParsedChartAlbum {
        file_name,
        name,
        rating,
        rating_count,
        artists,
        primary_genres,
        secondary_genres,
        descriptors,
        release_date,
      })
    })
    .collect::<Vec<_>>();

  Ok(albums)
}

#[cfg(test)]
mod tests {
  use chrono::NaiveDate;

  use super::*;
  use crate::test_resource;

  #[test]
  fn test_chart_parser() -> Result<(), String> {
    let file_content = include_str!(test_resource!("chart.html"));
    let chart = parse_chart(&file_content).map_err(|err| err.to_string())?;
    assert_eq!(chart.len(), 3);

    assert_eq!(
      chart[0].file_name,
      FileName::try_from("release/album/kendrick-lamar/to-pimp-a-butterfly").unwrap()
    );
    assert_eq!(chart[0].name, "To Pimp a Butterfly");
    assert_eq!(chart[0].rating, 4.38);
    assert_eq!(chart[0].rating_count, 79984);
    assert_eq!(chart[0].artists[0].name, "Kendrick Lamar");
    assert_eq!(
      chart[0].artists[0].file_name,
      FileName::try_from("artist/kendrick-lamar").unwrap()
    );
    assert_eq!(
      chart[0].primary_genres,
      vec!["Conscious Hip Hop", "West Coast Hip Hop", "Jazz Rap"]
    );
    assert_eq!(
      chart[0].secondary_genres,
      vec![
        "Political Hip Hop",
        "Neo-Soul",
        "Funk",
        "Poetry",
        "Experimental Hip Hop"
      ]
    );
    assert_eq!(
      chart[0].descriptors,
      vec![
        "political",
        "conscious",
        "concept album",
        "poetic",
        "introspective",
        "protest",
        "urban",
        "eclectic"
      ]
    );
    assert_eq!(chart[0].release_date, NaiveDate::from_ymd_opt(2015, 3, 15));

    assert_eq!(
      chart[1].file_name,
      FileName::try_from("release/album/radiohead/ok-computer").unwrap()
    );
    assert_eq!(chart[1].name, "OK Computer");
    assert_eq!(chart[1].rating, 4.29);
    assert_eq!(chart[1].rating_count, 104764);
    assert_eq!(chart[1].artists[0].name, "Radiohead");
    assert_eq!(
      chart[1].artists[0].file_name,
      FileName::try_from("artist/radiohead").unwrap()
    );
    assert_eq!(
      chart[1].primary_genres,
      vec!["Alternative Rock", "Art Rock"]
    );
    assert_eq!(
      chart[1].secondary_genres,
      vec!["Post-Britpop", "Space Rock Revival"]
    );
    assert_eq!(
      chart[1].descriptors,
      vec![
        "melancholic",
        "anxious",
        "alienation",
        "futuristic",
        "existential",
        "lonely",
        "atmospheric",
        "cold"
      ]
    );
    assert_eq!(chart[1].release_date, NaiveDate::from_ymd_opt(1997, 6, 16));

    assert_eq!(
      chart[2].file_name,
      FileName::try_from("release/album/radiohead/in-rainbows").unwrap()
    );
    assert_eq!(chart[2].name, "In Rainbows");
    assert_eq!(chart[2].rating, 4.31);
    assert_eq!(chart[2].rating_count, 77859);
    assert_eq!(chart[2].artists[0].name, "Radiohead");
    assert_eq!(
      chart[2].artists[0].file_name,
      FileName::try_from("artist/radiohead").unwrap()
    );
    assert_eq!(
      chart[2].primary_genres,
      vec!["Art Rock", "Alternative Rock"]
    );
    assert_eq!(
      chart[2].secondary_genres,
      vec!["Electronic", "Dream Pop", "Art Pop"]
    );
    assert_eq!(
      chart[2].descriptors,
      vec![
        "lush",
        "melancholic",
        "introspective",
        "mellow",
        "bittersweet",
        "atmospheric",
        "warm",
        "ethereal"
      ]
    );
    assert_eq!(chart[2].release_date, NaiveDate::from_ymd_opt(2007, 10, 10));

    Ok(())
  }
}
