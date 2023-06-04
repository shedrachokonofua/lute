use anyhow::Result;

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
