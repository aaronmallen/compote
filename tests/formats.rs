//! One chain, eight formats.
//!
//! Every source under `tests/fixtures/layered` speaks a different format and owns a different slice
//! of the configuration. Nothing about merging is per format: each file parses into the same shape
//! first, so a YAML table lays over a TOML one exactly as it would over another YAML one, an XML
//! and an INI file whose every value is text sit between typed ones without any of them noticing,
//! and an environment variable, which is only ever text as well, lands on a numeric field at the
//! bottom of a three-level path. The `.env` file sits directly beneath the environment it is
//! written in the shape of, which is the pattern that format exists for.

#![cfg(all(
  feature = "dotenv",
  feature = "env",
  feature = "ini",
  feature = "json",
  feature = "msgpack",
  feature = "toml",
  feature = "xml",
  feature = "yaml"
))]

mod common;

use ::compote::{Compote, Dotenv, Env, Ini, Json, MsgPack, Serialized, Toml, Xml, Yaml};

use crate::common::{
  Database, Feature, Level, Logging, Owner, Pool, Replica, Server, Settings, Target, Tls, fixture, map, strings,
};

const PREFIX: &str = "COMPOTE_FORMATS_";

/// The environment sitting on top of the files. Every value is text, as it always is.
fn environment() -> Vec<(String, Option<String>)> {
  [
    ("CONFIG", "/etc/compote/config.toml"),
    ("DATABASE__POOL__IDLE_TIMEOUT", "45.5"),
    ("MILESTONES", "2026-01-01, 2026-06-15"),
    ("SERVER__BACKLOG", "-1"),
    ("SERVER__MAX_BODY_BYTES", "4294967296"),
    ("SERVER__TLS__CIPHERS", "TLS_AES_128_GCM_SHA256, TLS_AES_256_GCM_SHA384"),
  ]
  .into_iter()
  .map(|(key, value)| (format!("{PREFIX}{key}"), Some(value.to_owned())))
  .collect()
}

/// Defaults, then the shipped file, then policy, then this host's, then the environment's, then the
/// developer's, then the generated secrets, then that developer's exports, then the process
/// environment. The generated layer is the binary one, which is the shape MessagePack is actually
/// for, and the two beneath the top speak the same shell names in a file and out of one.
fn stack() -> Settings {
  Compote::from(Serialized::defaults(Settings::default()))
    .merge(Toml::path(fixture("layered/base.toml")))
    .merge(Xml::path(fixture("layered/policy.xml")))
    .merge(Ini::path(fixture("layered/host.ini")).split("."))
    .merge(Yaml::path(fixture("layered/environment.yaml")))
    .merge(Json::path(fixture("layered/local.jsonc")))
    .merge(MsgPack::path(fixture("layered/secrets.msgpack")))
    .merge(
      Dotenv::path(fixture("layered/developer.env"))
        .prefixed("APP_")
        .ignore(&["CONFIG"])
        .split("__"),
    )
    .merge(Env::prefixed(PREFIX).ignore(&["CONFIG"]).split("__"))
    .extract()
    .unwrap()
}

fn merged() -> Settings {
  temp_env::with_vars(environment(), stack)
}

mod compote {
  use super::*;

  mod merge {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_merges_eight_formats_into_one_value() {
      assert_eq!(
        merged(),
        Settings {
          database: Database {
            pool: Pool {
              // Defaulted, raised by YAML, raised again by the environment.
              idle_timeout: 45.5,
              // Set by TOML, lowered by XML's text, raised by INI's, raised again by YAML.
              max: 32,
              // Set by TOML and never touched again.
              min: 2,
            },
            // Only YAML says anything about replicas.
            replicas: vec![Replica {
              host: "replica-1.internal".to_owned(),
              port: 5432,
              weight: 0.75,
            }],
            // TOML names a database, MessagePack replaces it with one that has credentials.
            url: "postgres://app:s3cret@db.internal/compote".to_owned(),
          },
          // Only XML mentions it, as two child elements of one.
          extra: strings(vec![("region", "us-east-1"), ("tier", "gold")]),
          features: map(vec![
            // YAML introduces the key, JSON changes both of its fields.
            (
              "beta-ui",
              Feature {
                enabled: true,
                rollout: 0.05,
              }
            ),
            // TOML's entry is untouched by the layers that add a sibling.
            (
              "metrics",
              Feature {
                enabled: true,
                rollout: 1.0,
              }
            ),
          ]),
          logging: Logging {
            // warn, then info, then debug.
            level: Level::Debug,
            // A list is replaced whole, never appended to.
            targets: vec![
              Target::Stdout,
              Target::File {
                path: "/tmp/compote.log".to_owned(),
                rotate: false,
              }
            ],
          },
          // Text from the environment, split on commas into a list.
          milestones: vec!["2026-01-01".to_owned(), "2026-06-15".to_owned()],
          name: "compote".to_owned(),
          // Only INI mentions them, as one comma-separated value.
          notes: vec!["handwritten".to_owned(), "uncommitted".to_owned()],
          // Only the generated file knows these, and one of them is an explicit null.
          owners: vec![
            Owner {
              email: Some("hello@aaronmallen.me".to_owned()),
              name: "Aaron".to_owned(),
            },
            Owner {
              email: None,
              name: "Ops".to_owned(),
            },
          ],
          // A bare TOML date, over a default string.
          released: "2026-08-28".to_owned(),
          server: Server {
            // INI's "512", then "-1" from the environment, both text onto an i32.
            backlog: -1,
            // JSON names one, the .env file adds another beside it.
            headers: strings(vec![("x-powered-by", "compote"), ("x_request_id", "from-the-dotenv")]),
            host: "0.0.0.0".to_owned(),
            // The .env file says 1, and the real environment above it says this instead.
            max_body_bytes: 4_294_967_296,
            // 80, then 8080, then 8443.
            port: 8443,
            tls: Tls {
              // The environment replaces the list TOML set.
              ciphers: vec!["TLS_AES_128_GCM_SHA256".to_owned(), "TLS_AES_256_GCM_SHA384".to_owned(),],
              // false in TOML, true in YAML.
              enabled: true,
              // XML sets the floor policy allows, YAML raises it.
              min_version: Some("1.3".to_owned()),
            },
          },
          tags: vec!["api".to_owned(), "web".to_owned()],
        }
      );
    }

    #[test]
    fn it_lets_a_later_format_change_one_key_of_a_table_an_earlier_one_set() {
      let pool = merged().database.pool;

      assert_eq!(pool.max, 32, "yaml raises the maximum toml set");
      assert_eq!(pool.min, 2, "and leaves the minimum beside it alone");
    }

    #[test]
    fn it_lets_a_text_only_format_sit_between_two_typed_ones() {
      let settings = merged();

      assert_eq!(
        settings.database.pool.max, 32,
        "toml's 16, then xml's text 12, then ini's text 24, then yaml's 32"
      );
      assert_eq!(
        settings.notes,
        vec!["handwritten", "uncommitted"],
        "a comma-separated value only ini supplies"
      );
      assert_eq!(
        settings.server.tls.min_version.as_deref(),
        Some("1.3"),
        "xml's floor, raised by the typed layer above it"
      );
    }

    #[test]
    fn it_lets_the_environment_beat_the_dotenv_file_written_in_its_shape() {
      let settings = merged();

      assert_eq!(
        settings.server.max_body_bytes, 4_294_967_296,
        "the file says 1, and the export above it wins"
      );
      assert_eq!(
        settings.server.headers["x_request_id"], "from-the-dotenv",
        "and what only the file says survives"
      );
      assert!(
        !settings.server.headers.contains_key("config"),
        "the name pointing at the config file is ignored in both layers"
      );
    }

    #[test]
    fn it_keeps_a_table_no_other_layer_names() {
      assert_eq!(
        merged().extra,
        strings(vec![("region", "us-east-1"), ("tier", "gold")]),
        "two child elements of the one element only xml supplies"
      );
    }

    #[test]
    fn it_replaces_a_list_rather_than_appending_to_it() {
      assert_eq!(
        merged().logging.targets.len(),
        2,
        "json's two targets, not toml's plus them"
      );
    }

    #[test]
    fn it_coerces_environment_text_at_the_bottom_of_a_nested_path() {
      let settings = merged();

      assert_eq!(settings.database.pool.idle_timeout, 45.5);
      assert_eq!(settings.server.tls.ciphers.len(), 2);
    }

    #[test]
    fn it_leaves_a_single_underscore_inside_a_name_alone() {
      assert_eq!(
        merged().server.max_body_bytes,
        4_294_967_296,
        "the separator is two underscores, so max_body_bytes stays one key"
      );
    }

    #[test]
    fn it_carries_a_parse_failure_across_formats_to_the_end_of_the_chain() {
      let error = Compote::from(Serialized::defaults(Settings::default()))
        .merge(Toml::path(fixture("layered/base.toml")))
        .merge(Yaml::path(fixture("layered/broken.yaml")))
        .merge(MsgPack::path(fixture("layered/secrets.msgpack")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to parse"), "{error}");
      assert!(error.to_string().contains("broken.yaml"), "{error}");
    }
  }

  mod join {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_walks_outward_from_the_nearest_file_to_the_furthest() {
      let settings: Settings = Compote::from(Json::path(fixture("layered/local.jsonc")))
        .join(Yaml::path(fixture("layered/environment.yaml")))
        .join(Toml::path(fixture("layered/base.toml")))
        .join(Serialized::defaults(Settings::default()))
        .extract()
        .unwrap();

      assert_eq!(settings.logging.level, Level::Debug, "the nearest file wins");
      assert_eq!(settings.server.port, 8443, "the file behind it fills the gap");
      assert_eq!(
        settings.server.host, "0.0.0.0",
        "and the one behind that fills the rest"
      );
      assert_eq!(settings.database.pool.min, 2);
      assert_eq!(settings.server.backlog, 128, "with the defaults underneath everything");
    }

    #[test]
    fn it_does_not_let_a_furthest_layer_take_a_key_a_nearer_one_set() {
      let settings: Settings = Compote::from(Yaml::path(fixture("layered/environment.yaml")))
        .join(Toml::path(fixture("layered/base.toml")))
        .join(Serialized::defaults(Settings::default()))
        .extract()
        .unwrap();

      assert!(
        settings.server.tls.enabled,
        "yaml said true, toml's false does not come back"
      );
      assert_eq!(settings.database.pool.max, 32);
      assert_eq!(settings.tags, vec!["api", "web"]);
    }
  }
}
