//! A `.env` file is the environment written down.
//!
//! Every value reaches its field as a string and is coerced there, and the names are lowercased and
//! nested exactly as [`Env`] does it, so the same file lands the same whether it is read here or
//! sourced into the shell first. What it costs is the shape a section gives INI: a `.env` file is a
//! flat list of names, and depth is [`split`] or nothing.
//!
//! [`Env`]: compote::Env
//! [`split`]: compote::Dotenv::split

#![cfg(feature = "dotenv")]

mod common;

use ::compote::{Compote, Dotenv, Provider, Value};

use crate::common::{Database, Logging, Server, Settings, Target, complete, fixture, map};

fn settings() -> Settings {
  Compote::from(Dotenv::path(fixture("complete/config.env")).split("__"))
    .extract()
    .unwrap()
}

/// The shared fixture, less the five fields a flat list of shell names has no way to say.
///
/// Each of `database.replicas`, `owners`, and the file entry under `logging.targets` wants a table
/// inside a list. A name holds one value, and saying it twice replaces that value rather than
/// adding to a list, so there is no spelling for one. This is INI's gap for the same reason.
///
/// `server.headers` and the `beta-ui` feature want a hyphen in a key, and a `.env` name is a shell
/// variable name. Its siblings `metrics` and `tracing` need no hyphen and are here, which is where
/// the line falls.
fn expected() -> Settings {
  Settings {
    database: Database {
      replicas: Vec::new(),
      ..complete().database
    },
    features: complete()
      .features
      .into_iter()
      .filter(|(name, _)| !name.contains('-'))
      .collect(),
    logging: Logging {
      targets: vec![Target::Stdout],
      ..complete().logging
    },
    owners: Vec::new(),
    server: Server {
      headers: map(Vec::new()),
      ..complete().server
    },
    ..complete()
  }
}

mod dotenv {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_file_that_uses_every_shape_dotenv_offers() {
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
    fn it_nests_a_name_on_the_separator() {
      let pool = settings().database.pool;

      assert_eq!(pool.max, 32);
      assert_eq!(pool.min, 4);
      assert_eq!(settings().server.tls.min_version.as_deref(), Some("1.3"));
    }

    #[test]
    fn it_reads_a_comma_separated_value_as_a_list() {
      assert_eq!(settings().tags, vec!["api", "web"]);
      assert_eq!(settings().milestones, vec!["2026-01-01", "2026-06-15"]);
      assert_eq!(
        settings().server.tls.ciphers,
        vec!["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
      );
    }

    #[test]
    fn it_reads_a_name_with_no_value_as_an_empty_list_and_an_empty_map() {
      assert!(settings().notes.is_empty());
      assert!(settings().owners.is_empty());
      assert!(settings().extra.is_empty());
    }

    #[test]
    fn it_reads_a_set_of_nested_names_as_a_map_of_structs() {
      let features = settings().features;

      assert_eq!(features.len(), 2, "beta-ui wants a hyphen, which no shell name has");
      assert!(features["metrics"].enabled);
      assert_eq!(features["metrics"].rollout, 1.0);
      assert!(!features["tracing"].enabled);
    }

    #[test]
    fn it_fills_a_name_in_from_an_earlier_line() {
      assert_eq!(
        settings().database.url,
        "postgres://localhost/compote",
        "the host is a name of its own, and the url is built from it"
      );
    }

    #[test]
    fn it_cannot_spell_a_key_holding_a_hyphen() {
      assert!(
        settings().server.headers.is_empty(),
        "both header names hold a hyphen, so the map is empty rather than wrong"
      );
    }

    #[test]
    fn it_reads_a_value_that_shares_its_line_with_a_comment() {
      assert_eq!(settings().logging.level, crate::common::Level::Info);
    }

    #[test]
    fn it_reads_an_export_prefix_as_if_it_were_not_there() {
      let Value::Table(root) = Dotenv::path(fixture("complete/config.env")).split("__").data().unwrap() else {
        panic!("a .env file is always a table");
      };
      let Some(Value::Table(database)) = root.get("database") else {
        panic!("the database table is nested on the separator");
      };

      assert_eq!(database.get("host"), Some(&Value::String("localhost".to_owned())));
    }

    #[test]
    fn it_keeps_a_name_whole_without_a_separator() {
      let Value::Table(root) = Dotenv::path(fixture("complete/config.env")).data().unwrap() else {
        panic!("a .env file is always a table");
      };

      assert!(root.contains_key("server__port"), "the name is one key until split");
      assert!(!root.contains_key("server"), "nothing nests on its own");
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Dotenv::path(fixture("complete/missing.env")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.env"), "{error}");
    }
  }
}
