//! XML is the third source that is only ever text.
//!
//! Nothing in the file says what anything is, so every value here reaches its field as a string and
//! is coerced there, exactly as an environment variable is. What XML adds over INI is depth that
//! costs nothing to ask for: an element nests inside an element, and an element said twice is a
//! list whose items carry children of their own, so this is the first text-only format that spells
//! the whole fixture.

#![cfg(feature = "xml")]

mod common;

use ::compote::{Compote, Provider, Value, Xml};

use crate::common::{Level, Settings, Target, complete, fixture};

fn settings() -> Settings {
  Compote::from(Xml::path(fixture("complete/config.xml")))
    .extract()
    .unwrap()
}

mod xml {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_file_that_uses_every_shape_xml_offers() {
      assert_eq!(settings(), complete());
    }

    #[test]
    fn it_coerces_text_onto_every_scalar_the_settings_type_asks_for() {
      let server = settings().server;

      assert_eq!(server.backlog, -1);
      assert_eq!(server.max_body_bytes, 4_294_967_296);
      assert_eq!(server.port, 8443);
      assert!(server.tls.enabled);
      assert_eq!(settings().database.pool.idle_timeout, 30.5);
    }

    #[test]
    fn it_reads_an_attribute_and_a_child_element_as_the_same_kind_of_key() {
      let server = settings().server;

      assert_eq!(server.host, "0.0.0.0", "an attribute");
      assert_eq!(server.headers["x-powered-by"], "compote", "a child element");
    }

    #[test]
    fn it_reads_a_nested_element_as_a_nested_table() {
      let pool = settings().database.pool;

      assert_eq!(pool.max, 32);
      assert_eq!(pool.min, 4);
    }

    #[test]
    fn it_reads_an_element_said_twice_as_a_list() {
      assert_eq!(
        settings().server.tls.ciphers,
        vec!["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
      );
      assert_eq!(settings().tags, vec!["api", "web"]);
      assert_eq!(settings().milestones, vec!["2026-01-01", "2026-06-15"]);
    }

    #[test]
    fn it_reads_a_repeated_element_with_children_as_a_list_of_structs() {
      let replicas = settings().database.replicas;

      assert_eq!(replicas.len(), 2);
      assert_eq!(replicas[0].host, "replica-1.internal");
      assert_eq!(replicas[1].port, 5433);
      assert_eq!(replicas[1].weight, 0.25);
    }

    #[test]
    fn it_reads_an_absent_element_as_none() {
      let owners = settings().owners;

      assert_eq!(owners[0].email.as_deref(), Some("hello@aaronmallen.me"));
      assert_eq!(owners[1].email, None, "the second owner has no email element at all");
    }

    #[test]
    fn it_reads_an_element_holding_nothing_as_an_empty_list_and_an_empty_map() {
      assert!(settings().notes.is_empty());
      assert!(settings().extra.is_empty());
    }

    #[test]
    fn it_reads_a_set_of_named_elements_as_a_map_of_structs() {
      let features = settings().features;

      assert_eq!(features.len(), 3);
      assert!(features["beta-ui"].enabled);
      assert_eq!(features["beta-ui"].rollout, 0.05);
      assert!(!features["tracing"].enabled);
    }

    #[test]
    fn it_joins_the_pieces_a_character_reference_splits_a_value_into() {
      assert_eq!(settings().server.headers["x-greeting"], "hello\tworld");
    }

    #[test]
    fn it_reads_a_cdata_section_as_the_text_it_wraps() {
      assert_eq!(settings().database.url, "postgres://localhost/compote");
    }

    #[test]
    fn it_reads_a_unit_variant_from_a_bare_word_and_a_struct_variant_from_one_key() {
      assert_eq!(settings().logging.level, Level::Info);
      assert_eq!(
        settings().logging.targets,
        vec![
          Target::Stdout,
          Target::File {
            path: "/var/log/compote.log".to_owned(),
            rotate: true,
          }
        ]
      );
    }

    #[test]
    fn it_throws_the_root_element_name_away() {
      let Value::Table(root) = Xml::path(fixture("complete/config.xml")).data().unwrap() else {
        panic!("an XML document with children is always a table");
      };

      assert!(!root.contains_key("config"), "the root names the file, not a key in it");
      assert!(root.contains_key("server"), "its children are the document");
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Xml::path(fixture("complete/missing.xml")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.xml"), "{error}");
    }
  }
}
