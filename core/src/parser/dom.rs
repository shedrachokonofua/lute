use anyhow::Result;
use tl::VDom;

pub fn query_select_first<'a>(
  parser: &'a tl::Parser<'a>,
  tag: &'a tl::HTMLTag<'a>,
  selector: &'a str,
) -> Result<&'a tl::HTMLTag<'a>> {
  tag
    .query_selector(parser, &selector)
    .and_then(|mut iter| iter.next())
    .ok_or(anyhow::anyhow!(
      "No element found for selector: {}",
      selector
    ))?
    .get(parser)
    .ok_or(anyhow::anyhow!("Failed to get parser node"))?
    .as_tag()
    .ok_or(anyhow::anyhow!("Failed to convert node to tag"))
}

pub fn get_tag_inner_text<'a>(
  parser: &'a tl::Parser<'a>,
  tag: &'a tl::HTMLTag<'a>,
  selector: &'a str,
) -> Result<String> {
  query_select_first(parser, tag, selector).map(|tag| tag.inner_text(parser).trim().to_string())
}

pub fn get_node_inner_text<'a>(
  parser: &'a tl::Parser<'a>,
  node: &'a tl::NodeHandle,
) -> Result<String> {
  node
    .get(parser)
    .ok_or(anyhow::anyhow!("Failed to get parser node"))?
    .as_tag()
    .ok_or(anyhow::anyhow!("Failed to convert node to tag"))
    .map(|tag| tag.inner_text(parser).trim().to_string())
}

pub fn get_meta_value<'a>(dom: &'a VDom, name: &'a str) -> Result<String> {
  dom
    .query_selector(&format!("meta[itemprop=\"{}\"]", name))
    .and_then(|mut iter| iter.next())
    .and_then(|node| node.get(dom.parser()))
    .and_then(|node| node.as_tag())
    .and_then(|tag| tag.attributes().get("content"))
    .flatten()
    .map(|content| content.as_utf8_str())
    .map(|name| name.to_string())
    .ok_or(anyhow::anyhow!("No meta value found for name: {}", name))
}

pub fn get_link_tag_href(tag: &tl::HTMLTag) -> Result<String> {
  tag
    .attributes()
    .get("href")
    .flatten()
    .map(|content| content.as_utf8_str())
    .map(|name| {
      name
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
    })
    .ok_or(anyhow::anyhow!("No href found"))
}
