use std::path::PathBuf;

use crate::{Provider, Result, Value};

/// Configuration read from a MessagePack file.
///
/// The one binary format here. Nothing about it is meant to be edited by hand, so it earns its place
/// when something else writes the file: a build step, a cache, a sidecar another process hands over.
/// Numbers, booleans, and strings keep the type the file gives them, and every integer a `u64` can
/// hold survives whole.
///
/// The file has to encode its maps with string keys, which is what `rmp_serde::to_vec_named` and
/// `Serializer::with_struct_map` produce. The compact encoding `rmp_serde::to_vec` produces instead
/// writes a struct as an array, dropping the field names, and an array of values in declaration
/// order cannot be merged with anything or read into a named field.
///
/// MessagePack's `bin` and `ext` families, timestamps among them, have no place in configuration and
/// are refused rather than guessed at.
///
/// An empty file, or one holding only a nil document, reads as an empty table rather than an error,
/// so an optional file costs nothing.
///
/// ```no_run
/// use compote::{Compote, MsgPack};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(MsgPack::path("config.msgpack")).extract().unwrap();
/// ```
pub struct MsgPack {
  path: PathBuf,
}

impl MsgPack {
  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
    }
  }
}

impl Provider for MsgPack {
  fn data(&self) -> Result<Value> {
    super::load_bytes(&self.path, |source| rmp_serde::from_slice(source))
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

  fn file(bytes: &[u8]) -> NamedTempFile {
    let mut handle = NamedTempFile::new().unwrap();
    handle.write_all(bytes).unwrap();

    handle
  }

  fn named(value: &impl Serialize) -> Vec<u8> {
    rmp_serde::to_vec_named(value).unwrap()
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

  mod msg_pack {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_the_scalar_types_the_file_declares() {
        let handle = file(&named(&settings()));

        assert_eq!(
          MsgPack::path(handle.path()).data().unwrap(),
          table(vec![("host", string("localhost")), ("port", Value::Integer(8080))])
        );
      }

      #[test]
      fn it_keeps_an_integer_too_wide_for_an_i64_whole() {
        let handle = file(&named(&std::collections::BTreeMap::from([("max", u64::MAX)])));

        assert_eq!(
          MsgPack::path(handle.path()).data().unwrap(),
          table(vec![("max", Value::Integer(i128::from(u64::MAX)))])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file(&[]);

        assert_eq!(MsgPack::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_a_file_it_cannot_read() {
        let error = MsgPack::path("/does/not/exist.msgpack").data().unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file(&[0xc1, 0xc1, 0xc1]);
        let error = MsgPack::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }

      /// `{"blob": <3 bytes>}`, written by hand because nothing in the crate encodes a `bin`.
      #[test]
      fn it_refuses_a_binary_payload() {
        let handle = file(&[
          0x81, // a map of one
          0xa4, b'b', b'l', b'o', b'b', // the key, as a string of four
          0xc4, 0x03, 0x01, 0x02, 0x03, // the value, as three raw bytes
        ]);

        let error = MsgPack::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains("byte array"), "{error}");
      }

      /// `{1: 2}`, which MessagePack allows and a table of named values does not.
      #[test]
      fn it_refuses_a_map_key_that_is_not_a_string() {
        let handle = file(&[0x81, 0x01, 0x02]);
        let error = MsgPack::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains("expected a string"), "{error}");
      }

      #[test]
      fn it_reads_a_struct_written_positionally_as_a_list() {
        let handle = file(&rmp_serde::to_vec(&settings()).unwrap());

        assert_eq!(
          MsgPack::path(handle.path()).data().unwrap(),
          Value::List(vec![string("localhost"), Value::Integer(8080)]),
          "the compact encoding drops the field names, so there is no table left to merge"
        );
      }

      #[test]
      fn it_turns_a_nil_document_into_an_empty_table() {
        let handle = file(&named(&()));

        assert_eq!(MsgPack::path(handle.path()).data().unwrap(), Value::table());
      }
    }
  }
}
