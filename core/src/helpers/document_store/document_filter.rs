use anyhow::Result;
use rusqlite::ToSql;
use std::fmt;
use strum::EnumString;

fn condense_whitespace(sql: &str) -> String {
  sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, PartialEq, EnumString, strum_macros::Display, Clone)]
#[strum(ascii_case_insensitive)]
pub enum LogicalOperator {
  And,
  Or,
}

const VALID_OPERATORS: [&str; 10] = [
  "=", "!=", ">", "<", ">=", "<=", "LIKE", "NOT LIKE", "IN", "NOT IN",
];

fn is_valid_operator(op: &str) -> bool {
  VALID_OPERATORS.contains(&op.to_uppercase().as_str())
}

pub struct Condition<T: ToSql + Send + Sync>(pub String, pub String, T);

pub struct ConditionGroup(
  Vec<Condition<Box<dyn ToSql + Send + Sync>>>,
  LogicalOperator,
);

pub struct DocumentFilter {
  condition_groups: Vec<(ConditionGroup, Option<LogicalOperator>)>,
}

impl Default for DocumentFilter {
  fn default() -> Self {
    Self::new()
  }
}

impl DocumentFilter {
  pub fn new() -> Self {
    Self {
      condition_groups: Vec::new(),
    }
  }

  pub fn from(condition_groups: Vec<(ConditionGroup, Option<LogicalOperator>)>) -> Self {
    Self { condition_groups }
  }

  pub fn condition<T: ToSql + Send + Sync + Clone + 'static>(
    &mut self,
    key: &str,
    op: &str,
    val: T,
  ) -> &mut Self {
    let key = key.to_string();
    if !is_valid_operator(op) {
      panic!("Invalid operator: {}", op);
    }
    let op = op.to_uppercase();

    if self.condition_groups.last().is_some_and(|c| c.1.is_none()) {
      self.and();
    }
    let val = Box::new(val.clone());
    self.condition_groups.push((
      ConditionGroup(vec![Condition(key, op, val)], LogicalOperator::And),
      None,
    ));
    self
  }

  pub fn group(&mut self, condition_group: ConditionGroup) -> &mut Self {
    for condition in &condition_group.0 {
      if !is_valid_operator(&condition.1) {
        panic!("Invalid operator: {}", condition.1);
      }
    }

    self.condition_groups.push((condition_group, None));
    self
  }

  pub fn and(&mut self) -> &mut Self {
    if let Some(c) = self.condition_groups.last_mut() {
      c.1 = Some(LogicalOperator::And);
    }
    self
  }

  pub fn or(&mut self) -> &mut Self {
    if let Some(c) = self.condition_groups.last_mut() {
      c.1 = Some(LogicalOperator::Or);
    }
    self
  }

  pub fn columns_select_list() -> String {
    "id, collection, key, json(json), created_at, updated_at, expires_at".to_string()
  }

  pub fn build(&mut self) -> Self {
    Self::from(self.condition_groups.drain(..).collect())
  }

  pub fn to_sql(
    &mut self,
    collection: String,
  ) -> Result<(String, Vec<(String, Box<dyn ToSql + Send + Sync>)>)> {
    let mut sql = format!(
      "
      SELECT {}
      FROM document_store
      WHERE collection = :collection
      AND (expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP)
      ",
      DocumentFilter::columns_select_list()
    );
    let mut added_sql = String::new();
    let mut params: Vec<(String, Box<dyn ToSql + Send + Sync>)> =
      vec![(":collection".to_string(), Box::new(collection))];

    for (group_idx, (ConditionGroup(condition_group, condition_chain), group_chain)) in
      self.condition_groups.drain(..).enumerate()
    {
      let mut group_sql = String::new();
      let mut group_params = Vec::new();
      let condition_group_size = condition_group.len();
      for (condition_idx, Condition(key, op, val)) in condition_group.into_iter().enumerate() {
        let param_key = format!(
          ":g{}_c{}_{}",
          group_idx,
          condition_idx,
          key.replace('.', "_")
        );
        let clause = format!("jsonb_extract(json, '$.{}') {} {} ", key, op, param_key);
        if condition_idx == condition_group_size - 1 {
          group_sql.push_str(&clause);
        } else {
          group_sql.push_str(&format!(
            "{} {} ",
            clause,
            condition_chain.to_string().to_uppercase()
          ));
        }
        group_params.push((param_key, val));
      }

      added_sql.push_str(&format!(
        "({}) {} ",
        group_sql.trim_end(),
        group_chain
          .as_ref()
          .map(|c| c.to_string().to_uppercase())
          .unwrap_or_default(),
      ));
      params.extend(group_params);
    }
    added_sql = added_sql.trim().to_string();
    if !added_sql.is_empty() {
      sql.push_str(format!("AND ({})", added_sql).as_str());
    }
    sql = condense_whitespace(&sql);

    Ok((sql, params))
  }
}

impl fmt::Debug for DocumentFilter {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let printable = self
      .condition_groups
      .iter()
      .map(|(group, op)| {
        let conditions = group
          .0
          .iter()
          .map(|Condition(key, op, val)| (key.to_string(), op.to_string(), val.to_sql().unwrap()))
          .collect::<Vec<_>>();
        ((conditions, group.1.clone()), op)
      })
      .collect::<Vec<_>>();
    write!(f, "{:?}", printable)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_document_filter_to_sql() {
    let mut filter = DocumentFilter::new();
    filter
      .condition("name", "=", "John")
      .and()
      .condition("age", ">", 18)
      .or()
      .condition("city", "=", "New York");

    let (sql, params) = filter.to_sql("users".to_string()).unwrap();

    let expected_sql = condense_whitespace(
      r#"
        SELECT id, collection, key, json(json), created_at, updated_at, expires_at
        FROM document_store
        WHERE collection = :collection
        AND (expires_at IS NULL OR expires_at > CURRENT_TIMESTAMP)
        AND ((jsonb_extract(json, '$.name') = :g0_c0_name) AND (jsonb_extract(json, '$.age') > :g1_c0_age) OR (jsonb_extract(json, '$.city') = :g2_c0_city))"#,
    );

    assert_eq!(sql, expected_sql);

    let expected_params: Vec<(&str, &dyn ToSql)> = vec![
      (":collection", &"users"),
      (":g0_c0_name", &"John"),
      (":g1_c0_age", &18 as &dyn ToSql),
      (":g2_c0_city", &"New York"),
    ];
    let expected = expected_params
      .into_iter()
      .map(|(k, v)| (k.to_string(), v.to_sql().unwrap()))
      .collect::<Vec<_>>();

    let params = params
      .iter()
      .map(|(k, v)| (k.clone(), v.to_sql().unwrap().clone()))
      .collect::<Vec<_>>();

    for (expected, actual) in expected.iter().zip(params.iter()) {
      assert_eq!(expected, actual);
    }
  }

  #[test]
  fn test_optional_and() {
    let mut left_filter = DocumentFilter::new();
    left_filter
      .condition("name", "=", "Jane")
      .and()
      .condition("age", ">", 18)
      .and()
      .condition("city", "=", "New York")
      .or()
      .condition("city", "=", "Barcelona");
    let left_output = left_filter.to_sql("users".to_string()).unwrap();

    let mut right_filter = DocumentFilter::new();
    right_filter
      .condition("name", "=", "Jane")
      .condition("age", ">", 18)
      .condition("city", "=", "New York")
      .or()
      .condition("city", "=", "Barcelona");
    let right_output = right_filter.to_sql("users".to_string()).unwrap();

    assert_eq!(left_output.0, right_output.0);
  }
}
