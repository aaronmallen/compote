//! One provider, both spellings.
//!
//! `complete/config.json` and `complete/config.jsonc` describe the same configuration, one in strict
//! JSON and one with comments and trailing commas. [`Json`] tells them apart by what they hold, so
//! both land on the same typed value.

#![cfg(feature = "json")]

mod common;

use compote::{Compote, Json};

use crate::common::{Settings, complete, fixture};

fn commented() -> Settings {
  Compote::from(Json::path(fixture("complete/config.jsonc")))
    .extract()
    .unwrap()
}

fn settings() -> Settings {
  Compote::from(Json::path(fixture("complete/config.json")))
    .extract()
    .unwrap()
}

mod json {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_deeply_nested_document() {
      assert_eq!(settings(), complete());
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
    fn it_reads_an_exponent_as_a_float() {
      assert_eq!(settings().features["beta-ui"].rollout, 0.05);
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
    fn it_reads_a_null_as_none() {
      assert_eq!(settings().owners[1].email, None);
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Json::path(fixture("complete/missing.json")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.json"), "{error}");
    }
  }

  /// The same document, spelled with the syntax strict JSON does not have.
  mod commented {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_commented_document_the_same_as_plain_json() {
      assert_eq!(commented(), complete());
    }

    #[test]
    fn it_lands_on_the_same_value_as_the_plain_file_beside_it() {
      assert_eq!(commented(), settings());
    }

    #[test]
    fn it_reads_past_a_comment_before_the_opening_brace() {
      assert_eq!(commented().name, "Compote — ünïcode ✓");
    }

    #[test]
    fn it_allows_a_trailing_comma_in_an_object() {
      let tls = commented().server.tls;

      assert!(tls.enabled);
      assert_eq!(tls.min_version.as_deref(), Some("1.3"));
    }

    #[test]
    fn it_allows_a_trailing_comma_in_an_array() {
      assert_eq!(
        commented().server.tls.ciphers,
        vec!["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
      );
    }

    #[test]
    fn it_allows_a_block_comment_between_nested_values() {
      let replicas = commented().database.replicas;

      assert_eq!(replicas.len(), 2);
      assert_eq!(replicas[1].host, "replica-2.internal");
    }

    #[test]
    fn it_reads_a_null_as_none() {
      assert_eq!(commented().owners[1].email, None);
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Json::path(fixture("complete/missing.jsonc")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.jsonc"), "{error}");
    }
  }
}
