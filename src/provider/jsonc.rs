use std::path::PathBuf;

use serde::Deserialize;

use crate::{Provider, Result, Value};

/// Configuration read from a JSON file that allows comments.
///
/// Line and block comments are both allowed, as are trailing commas. Apart from that it reads the
/// same as JSON.
///
/// An empty file, or one holding only a null document, reads as an empty table rather than an
/// error, so an optional file costs nothing.
///
/// ```no_run
/// use compote::{Compote, Jsonc};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Jsonc::path("config.jsonc")).extract().unwrap();
/// ```
pub struct Jsonc {
  path: PathBuf,
}

impl Jsonc {
  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
    }
  }
}

impl Provider for Jsonc {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, parse)
  }
}

#[derive(Debug, thiserror::Error)]
enum ParseError {
  #[error(transparent)]
  Jsonc(#[from] jsonc_parser::errors::ParseError),
  #[error(transparent)]
  Value(#[from] serde_json::Error),
}

fn parse(source: &str) -> std::result::Result<Value, ParseError> {
  let parsed: Option<serde_json::Value> =
    jsonc_parser::parse_to_serde_value(source, &jsonc_parser::ParseOptions::default())?;

  match parsed {
    Some(value) => Ok(Value::deserialize(value)?),
    None => Ok(Value::table()),
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

  mod jsonc {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file("\n");

        assert_eq!(Jsonc::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file("{ nope");
        let error = Jsonc::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }
    }
  }

  mod parse {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_allows_comments() {
      let source = r#"{
        // the host to bind
        "host": "localhost",
        /* and the port */
        "port": 8080
      }"#;

      assert_eq!(
        parse(source).unwrap(),
        table(vec![("host", string("localhost")), ("port", Value::Integer(8080))])
      );
    }

    #[test]
    fn it_allows_trailing_commas() {
      assert_eq!(
        parse(r#"{"host": "localhost",}"#).unwrap(),
        table(vec![("host", string("localhost"))])
      );
    }

    #[test]
    fn it_reads_a_comment_only_document_as_an_empty_table() {
      assert_eq!(parse("// nothing here").unwrap(), Value::table());
    }
  }
}
