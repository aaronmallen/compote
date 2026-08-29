#![cfg(feature = "jsonc")]

mod common;

use compote::{Compote, Jsonc};

use crate::common::{Settings, complete, fixture};

fn settings() -> Settings {
  Compote::from(Jsonc::path(fixture("complete/config.jsonc")))
    .extract()
    .unwrap()
}

mod jsonc {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_commented_document_the_same_as_plain_json() {
      assert_eq!(settings(), complete());
    }

    #[test]
    fn it_reads_past_a_comment_before_the_opening_brace() {
      assert_eq!(settings().name, "Compote — ünïcode ✓");
    }

    #[test]
    fn it_allows_a_trailing_comma_in_an_object() {
      let tls = settings().server.tls;

      assert!(tls.enabled);
      assert_eq!(tls.min_version.as_deref(), Some("1.3"));
    }

    #[test]
    fn it_allows_a_trailing_comma_in_an_array() {
      assert_eq!(
        settings().server.tls.ciphers,
        vec!["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
      );
    }

    #[test]
    fn it_allows_a_block_comment_between_nested_values() {
      let replicas = settings().database.replicas;

      assert_eq!(replicas.len(), 2);
      assert_eq!(replicas[1].host, "replica-2.internal");
    }

    #[test]
    fn it_reads_a_null_as_none() {
      assert_eq!(settings().owners[1].email, None);
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Jsonc::path(fixture("complete/missing.jsonc")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.jsonc"), "{error}");
    }
  }
}
