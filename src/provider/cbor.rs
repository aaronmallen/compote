use std::path::PathBuf;

use crate::{Provider, Result, Value};

/// Configuration read from a CBOR file.
///
/// The Concise Binary Object Representation, RFC 8949. Like [`MsgPack`](crate::MsgPack) it is for a
/// file something else writes rather than a person, and it reads the same way: numbers, booleans,
/// and strings keep the type the file gives them, and every integer a `u64` can hold survives whole.
///
/// The file has to key its maps with strings. CBOR allows any type as a key, and a table of named
/// values has nowhere to put an integer or a list that arrived where a name belongs.
///
/// Byte strings and tagged values, the standard date and time tags among them, are refused rather
/// than guessed at.
///
/// An empty file, or one holding only a null document, reads as an empty table rather than an error,
/// so an optional file costs nothing.
///
/// ```no_run
/// use compote::{Cbor, Compote};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Cbor::path("config.cbor")).extract().unwrap();
/// ```
pub struct Cbor {
  path: PathBuf,
}

impl Cbor {
  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
    }
  }
}

impl Provider for Cbor {
  fn data(&self) -> Result<Value> {
    super::load_bytes(&self.path, |source| ciborium::from_reader(source))
  }
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use serde::Serialize;
  use tempfile::NamedTempFile;

  use super::*;

  #[derive(Serialize)]
  struct Settings {
    host: String,
    port: u16,
  }

  fn encoded(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes).unwrap();

    bytes
  }

  fn file(bytes: &[u8]) -> NamedTempFile {
    let mut handle = NamedTempFile::new().unwrap();
    handle.write_all(bytes).unwrap();

    handle
  }

  fn settings() -> Settings {
    Settings {
      host: "localhost".to_owned(),
      port: 8080,
    }
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

  mod cbor {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_the_scalar_types_the_file_declares() {
        let handle = file(&encoded(&settings()));

        assert_eq!(
          Cbor::path(handle.path()).data().unwrap(),
          table(vec![("host", string("localhost")), ("port", Value::Integer(8080))])
        );
      }

      #[test]
      fn it_keeps_an_integer_too_wide_for_an_i64_whole() {
        let handle = file(&encoded(&std::collections::BTreeMap::from([("max", u64::MAX)])));

        assert_eq!(
          Cbor::path(handle.path()).data().unwrap(),
          table(vec![("max", Value::Integer(i128::from(u64::MAX)))])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file(&[]);

        assert_eq!(Cbor::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_a_file_it_cannot_read() {
        let error = Cbor::path("/does/not/exist.cbor").data().unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file(&[0xff, 0xff, 0xff]);
        let error = Cbor::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }

      /// `{"blob": h'010203'}`, written by hand because nothing in the crate encodes a byte string.
      #[test]
      fn it_refuses_a_byte_string() {
        let handle = file(&[
          0xa1, // a map of one
          0x64, b'b', b'l', b'o', b'b', // the key, as text of four
          0x43, 0x01, 0x02, 0x03, // the value, as three raw bytes
        ]);

        let error = Cbor::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }

      /// `{"at": 0("2026-08-28")}`, tag 0 being the standard date and time.
      #[test]
      fn it_refuses_a_tagged_value() {
        let handle = file(&[
          0xa1, // a map of one
          0x62, b'a', b't', // the key, as text of two
          0xc0, // tag 0, a date and time
          0x6a, b'2', b'0', b'2', b'6', b'-', b'0', b'8', b'-', b'2', b'8',
        ]);

        let error = Cbor::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }

      /// `{1: 2}`, which CBOR allows and a table of named values does not.
      #[test]
      fn it_refuses_a_map_key_that_is_not_a_string() {
        let handle = file(&[0xa1, 0x01, 0x02]);
        let error = Cbor::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }

      #[test]
      fn it_turns_a_null_document_into_an_empty_table() {
        let handle = file(&encoded(&()));

        assert_eq!(Cbor::path(handle.path()).data().unwrap(), Value::table());
      }
    }
  }
}
