//! The other binary format.
//!
//! `complete/config.cbor` holds what every other fixture under `complete/` holds. It is not written
//! by hand, so [`regenerate`] builds it from the JSON fixture beside it rather than leaving a blob
//! nobody can read to drift on its own:
//!
//! ```sh
//! cargo test --all-features --test cbor -- --ignored regenerate
//! ```

#![cfg(feature = "cbor")]

mod common;

use compote::{Cbor, Compote};

use crate::common::{Settings, complete, fixture};

fn settings() -> Settings {
  Compote::from(Cbor::path(fixture("complete/config.cbor")))
    .extract()
    .unwrap()
}

mod cbor {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_deeply_nested_document() {
      assert_eq!(
        settings(),
        complete(),
        "a binary document says the same thing or it is no use"
      );
    }

    #[test]
    fn it_reads_a_list_of_objects_as_a_list_of_structs() {
      let replicas = settings().database.replicas;

      assert_eq!(replicas.len(), 2);
      assert_eq!(replicas[0].weight, 0.75);
      assert_eq!(replicas[1].weight, 0.25);
    }

    #[test]
    fn it_reads_an_object_with_arbitrary_keys_as_a_map() {
      let features = settings().features;

      assert_eq!(features.len(), 3);
      assert!(features["beta-ui"].enabled);
      assert!(!features["tracing"].enabled);
    }

    #[test]
    fn it_reads_an_empty_object_and_an_empty_array() {
      let settings = settings();

      assert!(settings.extra.is_empty());
      assert!(settings.notes.is_empty());
    }

    #[test]
    fn it_reads_a_number_wider_than_u32() {
      assert_eq!(settings().server.max_body_bytes, 4_294_967_296);
    }

    #[test]
    fn it_reads_a_negative_number() {
      assert_eq!(settings().server.backlog, -1);
    }

    #[test]
    fn it_keeps_an_escape_sequence_intact() {
      assert_eq!(settings().server.headers["x-greeting"], "hello\tworld");
    }

    #[test]
    fn it_keeps_unicode_intact() {
      assert_eq!(settings().name, "Compote — ünïcode ✓");
    }

    #[test]
    fn it_reads_a_null_as_none() {
      assert_eq!(settings().owners[1].email, None);
    }

    #[test]
    fn it_reads_a_heterogeneous_list_of_enum_variants() {
      let targets = settings().logging.targets;

      assert_eq!(targets.len(), 2, "a unit variant and a struct variant in one list");
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Cbor::path(fixture("complete/missing.cbor")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.cbor"), "{error}");
    }
  }
}

/// Writes the binary fixture from the hand-written JSON one beside it.
///
/// Ignored, so it runs only when asked. See the note at the top of this file.
#[cfg(feature = "json")]
#[ignore = "rewrites a checked-in fixture; run it deliberately"]
#[test]
fn regenerate() {
  let settings: Settings = Compote::from(compote::Json::path(fixture("complete/config.json")))
    .extract()
    .unwrap();

  let mut bytes = Vec::new();
  ciborium::into_writer(&settings, &mut bytes).unwrap();

  std::fs::write(fixture("complete/config.cbor"), bytes).unwrap();
}
