use std::{collections::BTreeMap, path::PathBuf};

use crate::{Provider, Result, Value};

const DATETIME_KEY: &str = "$__toml_private_datetime";

/// Configuration read from a TOML file.
///
/// Dates and times arrive as strings, since configuration rarely wants a date type and every format
/// here has to agree on one shape.
///
/// An empty file, or one holding only a null document, reads as an empty table rather than an
/// error, so an optional file costs nothing.
///
/// ```no_run
/// use compote::{Compote, Toml};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Toml::path("config.toml")).extract().unwrap();
/// ```
pub struct Toml {
  path: PathBuf,
}

impl Toml {
  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
    }
  }
}

impl Provider for Toml {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, |source| toml_edit::de::from_str(source).map(flatten))
  }
}

fn datetime(entries: &BTreeMap<String, Value>) -> Option<String> {
  if entries.len() != 1 {
    return None;
  }

  match entries.get(DATETIME_KEY) {
    Some(Value::String(text)) => Some(text.clone()),
    _ => None,
  }
}

fn flatten(value: Value) -> Value {
  match value {
    Value::List(items) => Value::List(items.into_iter().map(flatten).collect()),
    Value::Table(entries) => match datetime(&entries) {
      Some(text) => Value::String(text),
      None => Value::Table(entries.into_iter().map(|(key, item)| (key, flatten(item))).collect()),
    },
    other => other,
  }
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use tempfile::NamedTempFile;

  use super::*;

  fn file(source: &str) -> NamedTempFile {
    let mut handle = NamedTempFile::new().unwrap();
    handle.write_all(source.as_bytes()).unwrap();

    handle
  }

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

  mod toml {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_the_scalar_types_the_file_declares() {
        let handle = file("host = \"localhost\"\nport = 8080\ntls = true\n");

        assert_eq!(
          Toml::path(handle.path()).data().unwrap(),
          table(vec![
            ("host", string("localhost")),
            ("port", Value::Integer(8080)),
            ("tls", Value::Bool(true)),
          ])
        );
      }

      #[test]
      fn it_reads_a_nested_table() {
        let handle = file("[server]\nhost = \"localhost\"\n");

        assert_eq!(
          Toml::path(handle.path()).data().unwrap(),
          table(vec![("server", table(vec![("host", string("localhost"))]))])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file("\n");

        assert_eq!(Toml::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file("host = ");
        let error = Toml::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }
    }
  }

  mod flatten {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_leaves_an_ordinary_table_alone() {
      let value = table(vec![("host", string("localhost"))]);

      assert_eq!(flatten(value.clone()), value);
    }

    #[test]
    fn it_turns_a_date_inside_a_list_into_a_string() {
      let value = Value::List(vec![table(vec![(DATETIME_KEY, string("2026-08-26"))])]);

      assert_eq!(flatten(value), Value::List(vec![string("2026-08-26")]));
    }

    #[test]
    fn it_turns_a_toml_date_into_a_string() {
      let value = table(vec![("released", table(vec![(DATETIME_KEY, string("2026-08-26"))]))]);

      assert_eq!(flatten(value), table(vec![("released", string("2026-08-26"))]));
    }
  }
}
