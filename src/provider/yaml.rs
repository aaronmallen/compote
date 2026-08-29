use std::path::PathBuf;

use crate::{Provider, Result, Value};

/// Configuration read from a YAML file.
///
/// Parsed by `yaml_serde`, which is maintained, rather than the deprecated `serde_yaml`.
///
/// An empty file, or one holding only a null document, reads as an empty table rather than an
/// error, so an optional file costs nothing.
///
/// ```no_run
/// use compote::{Compote, Yaml};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Yaml::path("config.yaml")).extract().unwrap();
/// ```
pub struct Yaml {
  path: PathBuf,
}

impl Yaml {
  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
    }
  }
}

impl Provider for Yaml {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, |source| yaml_serde::from_str(source))
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

  mod yaml {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_the_scalar_types_the_file_declares() {
        let handle = file("host: localhost\nport: 8080\ntls: true\n");

        assert_eq!(
          Yaml::path(handle.path()).data().unwrap(),
          table(vec![
            ("host", string("localhost")),
            ("port", Value::Integer(8080)),
            ("tls", Value::Bool(true)),
          ])
        );
      }

      #[test]
      fn it_reads_a_list() {
        let handle = file("tags:\n  - web\n  - api\n");

        assert_eq!(
          Yaml::path(handle.path()).data().unwrap(),
          table(vec![("tags", Value::List(vec![string("web"), string("api")]))])
        );
      }

      #[test]
      fn it_reads_a_nested_mapping() {
        let handle = file("server:\n  host: localhost\n");

        assert_eq!(
          Yaml::path(handle.path()).data().unwrap(),
          table(vec![("server", table(vec![("host", string("localhost"))]))])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file("\n");

        assert_eq!(Yaml::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file("host: [unterminated\n");
        let error = Yaml::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }

      #[test]
      fn it_turns_a_null_document_into_an_empty_table() {
        let handle = file("~\n");

        assert_eq!(Yaml::path(handle.path()).data().unwrap(), Value::table());
      }
    }
  }
}
