use std::path::PathBuf;

use jsonc_parser::{ParseOptions, errors::ParseError};

use crate::{Provider, Result, Value};

/// What [`Json`] accepts unless told otherwise: JSON, comments, and trailing commas.
///
/// `jsonc-parser` would allow a good deal more by default. Every field is named in these three
/// constants rather than left to `Default`, so a new one upstream has to be ruled on here rather
/// than quietly let in.
const DEFAULT: ParseOptions = ParseOptions {
  allow_comments: true,
  allow_hexadecimal_numbers: false,
  allow_loose_object_property_names: false,
  allow_missing_commas: false,
  allow_single_quoted_strings: false,
  allow_trailing_commas: true,
  allow_unary_plus_numbers: false,
};

/// Everything the parser can take. See [`Json::lenient`].
const LENIENT: ParseOptions = ParseOptions {
  allow_comments: true,
  allow_hexadecimal_numbers: true,
  allow_loose_object_property_names: true,
  allow_missing_commas: true,
  allow_single_quoted_strings: true,
  allow_trailing_commas: true,
  allow_unary_plus_numbers: true,
};

/// JSON and nothing else. See [`Json::strict`].
const STRICT: ParseOptions = ParseOptions {
  allow_comments: false,
  allow_hexadecimal_numbers: false,
  allow_loose_object_property_names: false,
  allow_missing_commas: false,
  allow_single_quoted_strings: false,
  allow_trailing_commas: false,
  allow_unary_plus_numbers: false,
};

/// Configuration read from a JSON file.
///
/// Numbers, booleans, and strings keep the type the file gives them.
///
/// Comments and trailing commas are allowed out of the box, so `.json` and `.jsonc` are one provider
/// and the extension decides nothing. Anything further has to be asked for, and anything unwanted
/// can be refused:
///
/// ```
/// use compote::Json;
///
/// // JSON, comments, and trailing commas.
/// let json = Json::path("config.json");
///
/// // The same, and hexadecimal numbers, and no comments.
/// let json = Json::path("config.json").allow_hexadecimal_numbers().deny_comments();
///
/// // JSON and nothing else.
/// let json = Json::path("config.json").strict();
/// ```
///
/// An empty file, or one holding only comments or a null document, reads as an empty table rather
/// than an error, so an optional file costs nothing.
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
  options: ParseOptions,
  path: PathBuf,
}

impl Json {
  /// Allows line and block comments. On unless refused.
  pub fn allow_comments(mut self) -> Self {
    self.options.allow_comments = true;

    self
  }

  /// Allows hexadecimal numbers, like `0xFF`.
  pub fn allow_hexadecimal_numbers(mut self) -> Self {
    self.options.allow_hexadecimal_numbers = true;

    self
  }

  /// Allows unquoted property names, like `{host: "localhost"}`.
  pub fn allow_loose_object_property_names(mut self) -> Self {
    self.options.allow_loose_object_property_names = true;

    self
  }

  /// Allows no comma at all between one value and the next, like `{"a": 1 "b": 2}`.
  pub fn allow_missing_commas(mut self) -> Self {
    self.options.allow_missing_commas = true;

    self
  }

  /// Allows single-quoted strings, like `'localhost'`.
  pub fn allow_single_quoted_strings(mut self) -> Self {
    self.options.allow_single_quoted_strings = true;

    self
  }

  /// Allows a comma after the last value, like `{"host": "localhost",}`. On unless refused.
  pub fn allow_trailing_commas(mut self) -> Self {
    self.options.allow_trailing_commas = true;

    self
  }

  /// Allows a leading plus on numbers, like `+42`.
  pub fn allow_unary_plus_numbers(mut self) -> Self {
    self.options.allow_unary_plus_numbers = true;

    self
  }

  /// Refuses line and block comments.
  pub fn deny_comments(mut self) -> Self {
    self.options.allow_comments = false;

    self
  }

  /// Refuses hexadecimal numbers, like `0xFF`.
  pub fn deny_hexadecimal_numbers(mut self) -> Self {
    self.options.allow_hexadecimal_numbers = false;

    self
  }

  /// Refuses unquoted property names, like `{host: "localhost"}`.
  pub fn deny_loose_object_property_names(mut self) -> Self {
    self.options.allow_loose_object_property_names = false;

    self
  }

  /// Refuses a value that follows another with no comma between them, like `{"a": 1 "b": 2}`.
  pub fn deny_missing_commas(mut self) -> Self {
    self.options.allow_missing_commas = false;

    self
  }

  /// Refuses single-quoted strings, like `'localhost'`.
  pub fn deny_single_quoted_strings(mut self) -> Self {
    self.options.allow_single_quoted_strings = false;

    self
  }

  /// Refuses a comma after the last value, like `{"host": "localhost",}`.
  pub fn deny_trailing_commas(mut self) -> Self {
    self.options.allow_trailing_commas = false;

    self
  }

  /// Refuses a leading plus on numbers, like `+42`.
  pub fn deny_unary_plus_numbers(mut self) -> Self {
    self.options.allow_unary_plus_numbers = false;

    self
  }

  /// Allows everything the parser can take, all at once.
  ///
  /// The place for a file a person edits by hand and no tool validates. Every allowance can be taken
  /// back one at a time afterwards.
  pub fn lenient(mut self) -> Self {
    self.options = LENIENT;

    self
  }

  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      options: DEFAULT,
      path: path.into(),
    }
  }

  /// Refuses everything JSON itself does not have, comments and trailing commas included.
  ///
  /// The place for a file a tool writes or a schema checks, where anything unusual is likelier to be
  /// a mistake than an intention.
  pub fn strict(mut self) -> Self {
    self.options = STRICT;

    self
  }
}

impl Provider for Json {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, |source| parse(source, &self.options))
  }
}

/// Reads `source` straight into a [`Value`], without a parser-shaped tree in between.
fn parse(source: &str, options: &ParseOptions) -> std::result::Result<Value, ParseError> {
  jsonc_parser::parse_to_serde_value(source, options)
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use tempfile::NamedTempFile;

  use super::*;

  /// One sample of each syntax that is not JSON, paired with the toggle that admits it.
  const BEYOND_JSON: [&str; 7] = [
    "{\"host\": \"localhost\"} // a comment",
    r#"{"mask": 0xFF}"#,
    r#"{host: "localhost"}"#,
    r#"{"a": 1 "b": 2}"#,
    r#"{'host': 'localhost'}"#,
    r#"{"host": "localhost",}"#,
    r#"{"port": +8080}"#,
  ];

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
      fn it_reads_a_commented_file_the_same_as_a_plain_one() {
        let plain = file(r#"{"host": "localhost", "port": 8080}"#);
        let commented = file(
          r#"{
            // the host to bind
            "host": "localhost",
            /* and the port */
            "port": 8080,
          }"#,
        );

        assert_eq!(
          Json::path(commented.path()).data().unwrap(),
          Json::path(plain.path()).data().unwrap()
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
      fn it_reports_the_path_when_the_syntax_is_one_the_provider_refuses() {
        let handle = file(r#"{"mask": 0xFF}"#);
        let error = Json::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }

      #[test]
      fn it_carries_a_toggle_through_to_the_parse() {
        let handle = file(r#"{"mask": 0xFF}"#);

        assert!(Json::path(handle.path()).data().is_err());
        assert_eq!(
          Json::path(handle.path()).allow_hexadecimal_numbers().data().unwrap(),
          table(vec![("mask", Value::Integer(255))])
        );
      }

      #[test]
      fn it_turns_a_comment_only_document_into_an_empty_table() {
        let handle = file("// nothing here\n");

        assert_eq!(Json::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_turns_a_null_document_into_an_empty_table() {
        let handle = file("null");

        assert_eq!(Json::path(handle.path()).data().unwrap(), Value::table());
      }
    }

    mod lenient {
      use super::*;

      #[test]
      fn it_takes_every_syntax_the_parser_knows() {
        for source in BEYOND_JSON {
          assert!(parse(source, &LENIENT).is_ok(), "{source} should parse");
        }
      }

      #[test]
      fn it_can_have_one_allowance_taken_back() {
        let handle = file(r#"{"mask": 0xFF}"#);

        assert!(
          Json::path(handle.path())
            .lenient()
            .deny_hexadecimal_numbers()
            .data()
            .is_err()
        );
      }
    }

    mod strict {
      use super::*;

      #[test]
      fn it_refuses_every_syntax_that_is_not_json() {
        for source in BEYOND_JSON {
          assert!(parse(source, &STRICT).is_err(), "{source} should not parse");
        }
      }

      #[test]
      fn it_refuses_the_comments_the_default_allows() {
        let handle = file("// a note\n{\"host\": \"localhost\"}");

        assert!(Json::path(handle.path()).data().is_ok());
        assert!(Json::path(handle.path()).strict().data().is_err());
      }

      #[test]
      fn it_can_have_one_allowance_handed_back() {
        let handle = file(r#"{"host": "localhost",}"#);

        assert!(Json::path(handle.path()).strict().data().is_err());
        assert!(
          Json::path(handle.path())
            .strict()
            .allow_trailing_commas()
            .data()
            .is_ok()
        );
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
        parse(source, &DEFAULT).unwrap(),
        table(vec![("host", string("localhost")), ("port", Value::Integer(8080))])
      );
    }

    #[test]
    fn it_allows_trailing_commas() {
      assert_eq!(
        parse(r#"{"host": "localhost",}"#, &DEFAULT).unwrap(),
        table(vec![("host", string("localhost"))])
      );
    }

    #[test]
    fn it_reads_a_comment_only_document_as_null() {
      assert_eq!(parse("// nothing here", &DEFAULT).unwrap(), Value::Null);
    }

    #[test]
    fn it_keeps_a_number_wider_than_i64_whole() {
      assert_eq!(
        parse(r#"{"max": 9223372036854775808}"#, &DEFAULT).unwrap(),
        table(vec![("max", Value::Integer(9_223_372_036_854_775_808))])
      );
    }

    #[test]
    fn it_reads_an_exponent_as_a_float() {
      assert_eq!(
        parse(r#"{"rollout": 5e-2}"#, &DEFAULT).unwrap(),
        table(vec![("rollout", Value::Float(0.05))])
      );
    }

    /// The five the parser would take by default and this provider will not.
    #[test]
    fn it_refuses_what_is_neither_json_nor_a_comment_nor_a_trailing_comma() {
      for source in [
        r#"{"mask": 0xFF}"#,
        r#"{host: "localhost"}"#,
        r#"{"a": 1 "b": 2}"#,
        r#"{'host': 'localhost'}"#,
        r#"{"port": +8080}"#,
      ] {
        assert!(parse(source, &DEFAULT).is_err(), "{source} should not parse");
      }
    }
  }
}
