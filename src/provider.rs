#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "dotenv")]
mod dotenv;
#[cfg(feature = "env")]
mod env;
#[cfg(feature = "ini")]
mod ini;
#[cfg(feature = "json")]
mod json;
#[cfg(feature = "msgpack")]
mod msgpack;
mod serialized;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "xml")]
mod xml;
#[cfg(feature = "yaml")]
mod yaml;

#[cfg(any(feature = "dotenv", feature = "env"))]
use std::collections::BTreeMap;

#[cfg(feature = "cbor")]
pub use cbor::Cbor;
#[cfg(feature = "dotenv")]
pub use dotenv::Dotenv;
#[cfg(feature = "env")]
pub use env::Env;
#[cfg(feature = "ini")]
pub use ini::Ini;
#[cfg(feature = "json")]
pub use json::Json;
#[cfg(feature = "msgpack")]
pub use msgpack::MsgPack;
pub use serialized::Serialized;
#[cfg(feature = "toml")]
pub use toml::Toml;
#[cfg(feature = "xml")]
pub use xml::Xml;
#[cfg(feature = "yaml")]
pub use yaml::Yaml;

use crate::{Result, Value};

/// A source of configuration.
///
/// Implement this to teach [`Compote`](crate::Compote) to read from somewhere it does not already
/// know about.
///
/// ```
/// use compote::{Provider, Result, Value};
///
/// struct Fixed;
///
/// impl Provider for Fixed {
///   fn data(&self) -> Result<Value> {
///     Ok(Value::String("hello".to_owned()))
///   }
/// }
/// ```
pub trait Provider {
  /// Reads this source and returns what it holds.
  ///
  /// Called once for every [`merge`](crate::Compote::merge) or [`join`](crate::Compote::join), and
  /// may be called more than once for the same source.
  fn data(&self) -> Result<Value>;
}

/// Names the file a parser failed on, and reads a document holding nothing as an empty table.
///
/// A source that says only `null` has nothing to contribute to a merge, which is the same thing an
/// absent key says, so it lays over the layer beneath without taking anything from it.
#[cfg(any(
  feature = "cbor",
  feature = "dotenv",
  feature = "ini",
  feature = "json",
  feature = "msgpack",
  feature = "toml",
  feature = "xml",
  feature = "yaml"
))]
fn finish<E>(path: &std::path::Path, parsed: std::result::Result<Value, E>) -> Result<Value>
where
  E: std::error::Error + Send + Sync + 'static,
{
  let value = parsed.map_err(|source| crate::Error::Parse {
    path: path.to_path_buf(),
    source: Box::new(source),
  })?;

  Ok(match value {
    Value::Null => Value::table(),
    other => other,
  })
}

/// Reads a text file and hands its contents to `parse`.
#[cfg(any(
  feature = "dotenv",
  feature = "ini",
  feature = "json",
  feature = "toml",
  feature = "xml",
  feature = "yaml"
))]
fn load<E>(path: &std::path::Path, parse: impl FnOnce(&str) -> std::result::Result<Value, E>) -> Result<Value>
where
  E: std::error::Error + Send + Sync + 'static,
{
  let source = std::fs::read_to_string(path).map_err(|source| crate::Error::Read {
    path: path.to_path_buf(),
    source,
  })?;

  if source.trim().is_empty() {
    return Ok(Value::table());
  }

  finish(path, parse(&source))
}

/// Reads a binary file and hands its bytes to `parse`.
#[cfg(any(feature = "cbor", feature = "msgpack"))]
fn load_bytes<E>(path: &std::path::Path, parse: impl FnOnce(&[u8]) -> std::result::Result<Value, E>) -> Result<Value>
where
  E: std::error::Error + Send + Sync + 'static,
{
  let source = std::fs::read(path).map_err(|source| crate::Error::Read {
    path: path.to_path_buf(),
    source,
  })?;

  if source.is_empty() {
    return Ok(Value::table());
  }

  finish(path, parse(&source))
}

/// Turns text pairs into a table, which is the shape the environment and a `.env` file share.
///
/// Only names carrying `prefix` are kept, and the prefix comes off what is left. Names are
/// lowercased, since both sources spell theirs in upper case by convention and neither means the
/// case to carry meaning, so one `.env` file lands the same whether it is read here or sourced into
/// the environment first. A name in `ignore` is dropped. Nothing nests unless `separator` says
/// where, and a nested key beats a scalar already sitting at the same path.
#[cfg(any(feature = "dotenv", feature = "env"))]
fn overlay(
  pairs: impl IntoIterator<Item = (String, String)>,
  prefix: &str,
  ignore: &[String],
  separator: &str,
) -> Value {
  // Sorted so a name and the path beneath it arrive in a fixed order, which is what lets the
  // deeper one win. The sort is stable, so a name said twice still ends on its last value.
  let mut sorted: Vec<(String, String)> = pairs.into_iter().collect();
  sorted.sort();

  let mut table = BTreeMap::new();

  for (key, value) in sorted {
    let Some(name) = key.strip_prefix(prefix).map(str::to_ascii_lowercase) else {
      continue;
    };

    if name.is_empty() || ignore.contains(&name) {
      continue;
    }

    insert(&mut table, &name, separator, value);
  }

  Value::Table(table)
}

/// Puts `value` at the path `key` names, making each table along the way that is missing.
#[cfg(any(feature = "dotenv", feature = "env"))]
fn insert(table: &mut BTreeMap<String, Value>, key: &str, separator: &str, value: String) {
  let segments: Vec<&str> = if separator.is_empty() {
    vec![key]
  } else {
    key.split(separator).filter(|segment| !segment.is_empty()).collect()
  };

  let Some((leaf, parents)) = segments.split_last() else {
    return;
  };

  let mut current = table;

  for segment in parents {
    let entry = current.entry((*segment).to_owned()).or_insert_with(Value::table);

    if !matches!(entry, Value::Table(_)) {
      *entry = Value::table();
    }

    let Value::Table(nested) = entry else {
      unreachable!("the entry was just replaced with a table")
    };

    current = nested;
  }

  current.insert((*leaf).to_owned(), Value::String(value));
}

#[cfg(all(test, any(feature = "dotenv", feature = "env")))]
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

  fn pairs(entries: &[(&str, &str)]) -> Vec<(String, String)> {
    entries
      .iter()
      .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
      .collect()
  }

  mod overlay {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_keeps_the_last_value_a_name_is_given() {
      assert_eq!(
        overlay(pairs(&[("HOST", "first"), ("HOST", "last")]), "", &[], ""),
        table(vec![("host", string("last"))]),
        "one name holds one value, which is all an environment variable can be"
      );
    }

    #[test]
    fn it_lets_a_nested_key_win_over_a_scalar_at_the_same_path() {
      assert_eq!(
        overlay(
          pairs(&[("SERVER", "off"), ("SERVER__HOST", "localhost")]),
          "",
          &[],
          "__"
        ),
        table(vec![("server", table(vec![("host", string("localhost"))]))])
      );
    }
  }

  mod insert {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_drops_empty_segments() {
      let mut entries = BTreeMap::new();

      insert(&mut entries, "server____host", "__", "localhost".to_owned());

      assert_eq!(
        Value::Table(entries),
        table(vec![("server", table(vec![("host", string("localhost"))]))])
      );
    }

    #[test]
    fn it_ignores_a_key_that_is_nothing_but_separators() {
      let mut entries = BTreeMap::new();

      insert(&mut entries, "__", "__", "localhost".to_owned());

      assert_eq!(Value::Table(entries), Value::table());
    }
  }
}
