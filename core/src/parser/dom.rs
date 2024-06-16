use anyhow::Result;
use htmlescape::decode_html;
use tl::{HTMLTag, VDom};
use tracing::warn;

pub struct HtmlParser<'a> {
  pub dom: VDom<'a>,
}

impl<'a> HtmlParser<'a> {
  pub fn try_from(html: &'a str) -> Result<Self> {
    let dom = tl::parse(html, tl::ParserOptions::default())?;
    Ok(Self { dom })
  }

  pub fn query_by_selector(
    &'a self,
    path: &[&str],
    root: Option<&'a HTMLTag<'a>>,
  ) -> Vec<&'a HTMLTag<'a>> {
    let mut current_row = Vec::<&'a HTMLTag<'a>>::new();
    if let Some(root) = root {
      current_row.push(root);
    }
    for selector in path {
      let mut next_row = vec![];
      if current_row.is_empty() {
        match self.dom.query_selector(selector) {
          Some(iter) => {
            for node in iter {
              if let Some(tag) = node.get(self.dom.parser()).and_then(|node| node.as_tag()) {
                next_row.push(tag);
              }
            }
          }
          None => return vec![],
        }
      } else {
        for tag in current_row {
          if let Some(iter) = tag.query_selector(self.dom.parser(), selector) {
            for node in iter {
              if let Some(tag) = node.get(self.dom.parser()).and_then(|node| node.as_tag()) {
                next_row.push(tag);
              }
            }
          }
        }
      }

      if next_row.is_empty() {
        return vec![];
      }
      current_row = next_row;
    }

    current_row
  }

  pub fn find_by_selector(
    &'a self,
    path: &[&str],
    root: Option<&'a HTMLTag<'a>>,
  ) -> Option<&'a HTMLTag<'a>> {
    self.query_by_selector(path, root).first().copied()
  }

  pub fn get_by_selector(
    &'a self,
    path: &[&str],
    root: Option<&'a HTMLTag<'a>>,
  ) -> Result<&'a HTMLTag<'a>> {
    self
      .query_by_selector(path, root)
      .first()
      .ok_or(anyhow::anyhow!("No element found for path: {:?}", path))
      .map(|tag| *tag)
  }

  pub fn find_by_id(&'a self, id: &str) -> Option<&'a HTMLTag<'a>> {
    self
      .dom
      .get_element_by_id(id)
      .and_then(|node| node.get(self.dom.parser()))
      .and_then(|node| node.as_tag())
  }

  pub fn get_by_id(&'a self, id: &str) -> Result<&'a HTMLTag<'a>> {
    self
      .find_by_id(id)
      .ok_or(anyhow::anyhow!("No element found for id: {}", id))
  }

  pub fn find_tag_attribute_value(&self, tag: &HTMLTag, attribute: &str) -> Option<String> {
    tag
      .attributes()
      .get(attribute)
      .flatten()
      .map(|value| value.as_utf8_str().to_string())
      .and_then(|value| {
        decode_html(&value)
          .map(|text| text.trim().to_string())
          .map_err(|err| anyhow::anyhow!("Failed to decode html: {:?}", err))
          .ok()
      })
  }

  pub fn get_tag_attribute_value(&self, tag: &HTMLTag, attribute: &str) -> Result<String> {
    self
      .find_tag_attribute_value(tag, attribute)
      .ok_or(anyhow::anyhow!("No attribute found for tag: {:?}", tag))
  }

  pub fn find_attribute_value(
    &self,
    path: &[&str],
    attribute: &str,
    root: Option<&'a HTMLTag<'a>>,
  ) -> Option<String> {
    let tag = self.find_by_selector(path, root)?;
    self.find_tag_attribute_value(tag, attribute)
  }

  pub fn get_attribute_value(
    &self,
    path: &[&str],
    attribute: &str,
    root: Option<&'a HTMLTag<'a>>,
  ) -> Result<String> {
    let tag = self.get_by_selector(path, root)?;
    self
      .find_tag_attribute_value(tag, attribute)
      .ok_or(anyhow::anyhow!("No attribute found for path: {:?}", path))
  }

  pub fn get_meta_item_prop(&self, name: &str) -> Result<String> {
    let selector = format!("meta[itemprop=\"{}\"]", name);
    let value = self.get_attribute_value(&[&selector], "content", None)?;
    let decoded =
      decode_html(&value).map_err(|err| anyhow::anyhow!("Failed to decode html: {:?}", err))?;
    Ok(decoded.trim().to_string())
  }

  pub fn get_tag_text(&self, tag: &HTMLTag) -> Result<String> {
    let text = tag.inner_text(self.dom.parser());
    let decoded = decode_html(text.as_ref())
      .map_err(|err| anyhow::anyhow!("Failed to decode html: {:?}", err))?;
    Ok(decoded.trim().to_string())
  }

  pub fn find_tag_text(&self, tag: &HTMLTag) -> Option<String> {
    let text = tag.inner_text(self.dom.parser());
    let decoded = decode_html(text.as_ref())
      .inspect_err(|err| {
        warn!("Failed to decode html: {:?}", err);
      })
      .ok()?;
    Some(decoded.trim().to_string())
  }

  pub fn get_text(&self, path: &[&str], root: Option<&'a HTMLTag<'a>>) -> Result<String> {
    let tag = self.get_by_selector(path, root)?;
    self.get_tag_text(tag)
  }

  pub fn find_text(&self, path: &[&str], root: Option<&'a HTMLTag<'a>>) -> Option<String> {
    let tag = self.find_by_selector(path, root)?;
    self.find_tag_text(tag)
  }

  pub fn get_tag_href(&self, tag: &HTMLTag) -> Result<String> {
    self.get_tag_attribute_value(tag, "href")
  }

  pub fn find_tag_href(&self, tag: &HTMLTag) -> Option<String> {
    self.find_tag_attribute_value(tag, "href")
  }
}
