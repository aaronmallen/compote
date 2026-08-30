//! INI is the second source that is only ever text.
//!
//! Nothing in the file says what anything is, so every value here reaches its field as a string and
//! is coerced there, exactly as an environment variable is. What INI adds over the environment is a
//! shape: a section is a table, and under [`split`] a dot in a section name or a key is another
//! level of one.
//!
//! [`split`]: compote::Ini::split

#![cfg(feature = "ini")]

mod common;

use ::compote::{Compote, Ini, Provider, Value};

use crate::common::{Database, Logging, Settings, Target, complete, fixture};

fn settings() -> Settings {
  Compote::from(Ini::path(fixture("complete/config.ini")).split("."))
    .extract()
    .unwrap()
}

/// The shared fixture, less the three fields INI has no way to say.
///
/// Each of `database.replicas`, `owners`, and the file entry under `logging.targets` wants a table
/// inside a list. A repeated section joins what is already under that name rather than adding to a
/// list, so there is no spelling for one. Everything else matches every other format exactly.
fn expected() -> Settings {
  Settings {
    database: Database {
      replicas: Vec::new(),
      ..complete().database
    },
    logging: Logging {
      targets: vec![Target::Stdout],
      ..complete().logging
    },
    owners: Vec::new(),
    ..complete()
  }
}

mod ini {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_file_that_uses_every_shape_ini_offers() {
      assert_eq!(settings(), expected());
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
    fn it_reads_a_dotted_section_name_as_a_nested_table() {
      let tls = settings().server.tls;

      assert!(tls.enabled);
      assert_eq!(tls.min_version.as_deref(), Some("1.3"));
    }

    #[test]
    fn it_reads_a_dotted_key_as_a_nested_table() {
      let pool = settings().database.pool;

      assert_eq!(pool.max, 32);
      assert_eq!(pool.min, 4);
    }

    #[test]
    fn it_reads_a_key_said_twice_as_a_list() {
      assert_eq!(
        settings().server.tls.ciphers,
        vec!["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
      );
    }

    #[test]
    fn it_reads_a_comma_separated_value_as_a_list() {
      assert_eq!(settings().tags, vec!["api", "web"]);
      assert_eq!(settings().milestones, vec!["2026-01-01", "2026-06-15"]);
    }

    #[test]
    fn it_reads_a_key_with_no_value_as_an_empty_list() {
      assert!(settings().notes.is_empty());
      assert!(settings().owners.is_empty());
    }

    #[test]
    fn it_reads_a_section_holding_nothing_as_an_empty_map() {
      assert!(settings().extra.is_empty());
    }

    #[test]
    fn it_reads_a_section_of_sections_as_a_map_of_structs() {
      let features = settings().features;

      assert_eq!(features.len(), 3);
      assert!(features["beta-ui"].enabled);
      assert_eq!(features["beta-ui"].rollout, 0.05);
      assert!(!features["tracing"].enabled);
    }

    #[test]
    fn it_keeps_an_escape_inside_a_quoted_value() {
      assert_eq!(settings().server.headers["x-greeting"], "hello\tworld");
    }

    #[test]
    fn it_keeps_a_value_that_looks_like_a_url_whole() {
      assert_eq!(settings().database.url, "postgres://localhost/compote");
    }

    #[test]
    fn it_reads_a_unit_variant_from_a_bare_word() {
      assert_eq!(settings().logging.level, crate::common::Level::Info);
      assert_eq!(settings().logging.targets, vec![Target::Stdout]);
    }

    #[test]
    fn it_leaves_a_dotted_section_name_whole_without_a_separator() {
      let Value::Table(root) = Ini::path(fixture("complete/config.ini")).data().unwrap() else {
        panic!("an INI file is always a table");
      };

      assert!(root.contains_key("server.tls"), "the dot is part of the one key");
      assert!(!root.contains_key("features"), "nothing nests on its own");
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Ini::path(fixture("complete/missing.ini")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.ini"), "{error}");
    }
  }
}
