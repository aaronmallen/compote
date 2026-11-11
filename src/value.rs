mod de;
pub(crate) mod ser;

use std::collections::{BTreeMap, btree_map::Entry};

use crate::{Provider, Result};

/// A configuration value, whatever format it arrived in.
///
/// Every source parses into this one shape, so merging and type coercion happen in a single place
/// rather than once per format. Strings are coerced on the way out, which is what lets an
/// environment variable satisfy a numeric or boolean field.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
  /// A boolean.
  Bool(bool),
  /// A floating point number.
  Float(f64),
  /// A whole number. Wide enough to hold every `i64` and every `u64`.
  Integer(i128),
  /// An ordered list of values.
  List(Vec<Value>),
  /// No value. An absent key, or an explicit null in the source.
  Null,
  /// Text. Also what every environment variable starts out as.
  String(String),
  /// A set of named values, sorted by name.
  Table(BTreeMap<String, Value>),
}

impl Value {
  /// Lays `overlay` over this value.
  ///
  /// Two tables merge key by key and the merge recurses, so keys the overlay leaves alone survive.
  /// Anything else is replaced outright, which means a scalar in the overlay wins over a whole
  /// table beneath it.
  ///
  /// ```
  /// use std::collections::BTreeMap;
  ///
  /// use compote::Value;
  ///
  /// let mut base = Value::Table(BTreeMap::from([
  ///   ("host".to_owned(), Value::String("localhost".to_owned())),
  ///   ("port".to_owned(), Value::Integer(80)),
  /// ]));
  ///
  /// base.merge(Value::Table(BTreeMap::from([("port".to_owned(), Value::Integer(8080))])));
  ///
  /// assert_eq!(
  ///   base,
  ///   Value::Table(BTreeMap::from([
  ///     ("host".to_owned(), Value::String("localhost".to_owned())),
  ///     ("port".to_owned(), Value::Integer(8080)),
  ///   ]))
  /// );
  /// ```
  pub fn merge(&mut self, overlay: Self) {
    match (self, overlay) {
      (Self::Table(base), Self::Table(over)) => {
        for (key, value) in over {
          match base.entry(key) {
            Entry::Occupied(mut occupied) => occupied.get_mut().merge(value),
            Entry::Vacant(vacant) => {
              vacant.insert(value);
            }
          }
        }
      }
      (slot, other) => *slot = other,
    }
  }

  /// Returns an empty table.
  ///
  /// ```
  /// use std::collections::BTreeMap;
  ///
  /// use compote::Value;
  ///
  /// assert_eq!(Value::table(), Value::Table(BTreeMap::new()));
  /// ```
  pub fn table() -> Self {
    Self::Table(BTreeMap::new())
  }
}

impl Provider for Value {
  fn data(&self) -> Result<Value> {
    Ok(self.clone())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn string(value: &str) -> Value {
    Value::String(value.to_owned())
  }

  fn table(entries: Vec<(&str, Value)>) -> Value {
    Value::Table(
      entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect(),
    )
  }

  mod value {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_hands_back_a_copy_of_itself() {
        let value = table(vec![("host", string("localhost"))]);

        assert_eq!(value.data().unwrap(), value);
      }
    }

    mod merge {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_adds_keys_the_base_is_missing() {
        let mut base = table(vec![("host", string("localhost"))]);

        base.merge(table(vec![("port", Value::Integer(8080))]));

        assert_eq!(
          base,
          table(vec![("host", string("localhost")), ("port", Value::Integer(8080))])
        );
      }

      #[test]
      fn it_recurses_into_tables_and_keeps_untouched_keys() {
        let mut base = table(vec![(
          "server",
          table(vec![("host", string("localhost")), ("port", Value::Integer(80))]),
        )]);

        base.merge(table(vec![("server", table(vec![("port", Value::Integer(8080))]))]));

        assert_eq!(
          base,
          table(vec![(
            "server",
            table(vec![("host", string("localhost")), ("port", Value::Integer(8080))]),
          )])
        );
      }

      #[test]
      fn it_replaces_a_scalar_with_the_overlay() {
        let mut base = table(vec![("debug", Value::Bool(false))]);

        base.merge(table(vec![("debug", Value::Bool(true))]));

        assert_eq!(base, table(vec![("debug", Value::Bool(true))]));
      }

      #[test]
      fn it_replaces_a_table_when_the_overlay_is_a_scalar() {
        let mut base = table(vec![("server", table(vec![("host", string("localhost"))]))]);

        base.merge(table(vec![("server", string("off"))]));

        assert_eq!(base, table(vec![("server", string("off"))]));
      }
    }

    mod table {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_returns_an_empty_table() {
        assert_eq!(Value::table(), Value::Table(BTreeMap::new()));
      }
    }
  }
}
