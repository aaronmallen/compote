use std::path::PathBuf;

use crate::{Provider, Result, Value};

/// Configuration read from a JSON file.
///
/// Numbers, booleans, and strings keep the type the file gives them.
///
/// An empty file, or one holding only a null document, reads as an empty table rather than an
/// error, so an optional file costs nothing.
///
/// ```no_run
/// use compote::{Compote, Json};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Json::path("config.json")).extract().unwrap();
/// ```
pub struct Json {
  path: PathBuf,
}

impl Json {
  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
    }
  }
}

impl Provider for Json {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, |source| serde_json::from_str(source))
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

  mod json {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_the_scalar_types_the_file_declares() {
        let handle = file(r#"{"host": "localhost", "port": 8080, "tls": true}"#);

        assert_eq!(
          Json::path(handle.path()).data().unwrap(),
          table(vec![
            ("host", string("localhost")),
            ("port", Value::Integer(8080)),
            ("tls", Value::Bool(true)),
          ])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file("   \n");

        assert_eq!(Json::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_a_file_it_cannot_read() {
        let error = Json::path("/does/not/exist.json").data().unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file("{ nope");
        let error = Json::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }

      #[test]
      fn it_turns_a_null_document_into_an_empty_table() {
        let handle = file("null");

        assert_eq!(Json::path(handle.path()).data().unwrap(), Value::table());
      }
    }
  }
}
