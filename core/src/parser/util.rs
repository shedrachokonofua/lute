use anyhow::Result;
use chrono::{Month, NaiveDate};

pub fn parse_release_date(date_string: String) -> Result<NaiveDate> {
  let date_string = date_string.trim();
  if date_string.is_empty() {
    return Err(anyhow::anyhow!("Empty date"));
  }

  // Possible formats:  "2020", "January 2020", 1 January 2020"
  let parts = date_string.split(" ").collect::<Vec<&str>>();
  match parts.len() {
    1 => {
      let year = parts[0].parse::<i32>()?;
      NaiveDate::from_yo_opt(year, 1).ok_or(anyhow::anyhow!("Invalid year: {}", year))
    }
    2 => {
      let month = parts[0]
        .parse::<Month>()
        .map_err(|_| anyhow::anyhow!("Invalid month: {}", parts[0]))?;
      let year = parts[1].parse::<i32>()?;
      NaiveDate::from_ymd_opt(year, month.number_from_month(), 1).ok_or(anyhow::anyhow!(
        "Invalid year: {} month: {}",
        year,
        month.number_from_month()
      ))
    }
    3 => NaiveDate::parse_from_str(date_string, "%d %B %Y")
      .map_err(|e| anyhow::anyhow!("Failed to parse date: {}", date_string)),
    _ => Err(anyhow::anyhow!("Invalid date: {}", date_string)),
  }
}

pub fn clean_artist_name(artist_name: &str) -> &str {
  artist_name.trim_end_matches(" &amp;")
}
