#![cfg(feature = "json")]

mod common;

use compote::{Compote, Json};

use crate::common::{Settings, complete, fixture};

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
}
