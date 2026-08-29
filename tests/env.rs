//! Environment variables are the one source that is always text.
//!
//! Everything here leans on that: the names have to be reshaped into a tree, and the values have to
//! land on typed fields without the file formats' help. `temp-env` puts the variables in place for
//! the length of a closure and takes them back out afterwards.

#![cfg(feature = "env")]

mod common;

use ::compote::{Compote, Env, Serialized};
use serde::Deserialize;

use crate::common::{Level, Settings, Target};

const PREFIX: &str = "COMPOTE_ENV_TEST_";

#[derive(Debug, Deserialize, PartialEq)]
struct Coerced {
  backlog: i32,
  enabled: bool,
  max_body_bytes: u64,
  ratio: f64,
  tags: Vec<String>,
  version: Option<String>,
  workers: u16,
}

fn vars(pairs: &[(&str, &str)]) -> Vec<(String, Option<String>)> {
  pairs
    .iter()
    .map(|(key, value)| (format!("{PREFIX}{key}"), Some((*value).to_owned())))
    .collect()
}

/// Reads the environment through `Env` and deserializes it into `T`.
fn read<T>(pairs: &[(&str, &str)], provider: impl Fn() -> Env) -> T
where
  T: serde::de::DeserializeOwned,
{
  temp_env::with_vars(vars(pairs), || Compote::from(provider()).extract().unwrap())
}

mod env {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_coerces_text_onto_every_scalar_the_settings_type_asks_for() {
      let coerced: Coerced = read(
        &[
          ("BACKLOG", "-1"),
          ("ENABLED", "yes"),
          ("MAX_BODY_BYTES", "4294967296"),
          ("RATIO", "0.75"),
          ("TAGS", "api, web,  cli "),
          ("VERSION", "1.3"),
          ("WORKERS", "8443"),
        ],
        || Env::prefixed(PREFIX),
      );

      assert_eq!(
        coerced,
        Coerced {
          backlog: -1,
          enabled: true,
          max_body_bytes: 4_294_967_296,
          ratio: 0.75,
          tags: vec!["api".to_owned(), "web".to_owned(), "cli".to_owned()],
          version: Some("1.3".to_owned()),
          workers: 8443,
        }
      );
    }

    #[test]
    fn it_accepts_every_spelling_of_a_boolean() {
      #[derive(Debug, Deserialize, PartialEq)]
      struct Flags {
        a: bool,
        b: bool,
        c: bool,
        d: bool,
        e: bool,
        f: bool,
        g: bool,
        h: bool,
      }

      let flags: Flags = read(
        &[
          ("A", "true"),
          ("B", "yes"),
          ("C", "on"),
          ("D", "1"),
          ("E", "false"),
          ("F", "no"),
          ("G", "off"),
          ("H", "0"),
        ],
        || Env::prefixed(PREFIX),
      );

      assert_eq!(
        flags,
        Flags {
          a: true,
          b: true,
          c: true,
          d: true,
          e: false,
          f: false,
          g: false,
          h: false,
        }
      );
    }

    #[test]
    fn it_reports_a_value_that_does_not_fit_the_field() {
      let error = temp_env::with_vars(vars(&[("WORKERS", "not a number")]), || {
        Compote::from(Env::prefixed(PREFIX)).extract::<Coerced>().unwrap_err()
      });

      assert!(error.to_string().contains("not a number"), "{error}");
    }

    #[test]
    fn it_reports_a_number_too_wide_for_the_field() {
      let error = temp_env::with_vars(vars(&[("WORKERS", "70000")]), || {
        Compote::from(Env::prefixed(PREFIX)).extract::<Coerced>().unwrap_err()
      });

      assert_eq!(error.to_string(), "70000 is out of range");
    }
  }

  mod prefixed {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_only_the_variables_carrying_the_prefix() {
      #[derive(Debug, Deserialize, PartialEq)]
      struct Named {
        host: String,
      }

      let named: Named = temp_env::with_vars(
        [
          (format!("{PREFIX}HOST"), Some("localhost".to_owned())),
          ("PATH_TO_NOWHERE".to_owned(), Some("/usr/bin".to_owned())),
          ("COMPOTE_OTHER_HOST".to_owned(), Some("elsewhere".to_owned())),
        ],
        || Compote::from(Env::prefixed(PREFIX)).extract().unwrap(),
      );

      assert_eq!(
        named,
        Named {
          host: "localhost".to_owned(),
        }
      );
    }

    #[test]
    fn it_keeps_names_flat_until_a_separator_is_set() {
      #[derive(Debug, Deserialize, PartialEq)]
      struct Flat {
        #[serde(rename = "server__host")]
        server_host: String,
      }

      let flat: Flat = read(&[("SERVER__HOST", "localhost")], || Env::prefixed(PREFIX));

      assert_eq!(
        flat,
        Flat {
          server_host: "localhost".to_owned(),
        }
      );
    }
  }

  mod ignore {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_drops_the_variable_that_points_at_the_configuration() {
      #[derive(Debug, Deserialize, PartialEq)]
      #[serde(deny_unknown_fields)]
      struct Named {
        host: String,
      }

      let named: Named = read(&[("CONFIG", "/etc/compote/config.toml"), ("HOST", "localhost")], || {
        Env::prefixed(PREFIX).ignore(&["CONFIG"])
      });

      assert_eq!(
        named,
        Named {
          host: "localhost".to_owned(),
        }
      );
    }
  }

  mod split {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_nests_three_levels_deep() {
      #[derive(Debug, Deserialize, PartialEq)]
      struct Tls {
        ciphers: Vec<String>,
        enabled: bool,
      }

      #[derive(Debug, Deserialize, PartialEq)]
      struct Server {
        max_body_bytes: u64,
        tls: Tls,
      }

      #[derive(Debug, Deserialize, PartialEq)]
      struct Nested {
        server: Server,
      }

      let nested: Nested = read(
        &[
          ("SERVER__MAX_BODY_BYTES", "4294967296"),
          ("SERVER__TLS__CIPHERS", "TLS_AES_128_GCM_SHA256, TLS_AES_256_GCM_SHA384"),
          ("SERVER__TLS__ENABLED", "on"),
        ],
        || Env::prefixed(PREFIX).split("__"),
      );

      assert_eq!(
        nested,
        Nested {
          server: Server {
            max_body_bytes: 4_294_967_296,
            tls: Tls {
              ciphers: vec!["TLS_AES_128_GCM_SHA256".to_owned(), "TLS_AES_256_GCM_SHA384".to_owned(),],
              enabled: true,
            },
          },
        }
      );
    }

    #[test]
    fn it_lets_a_nested_name_win_over_a_scalar_at_the_same_path() {
      #[derive(Debug, Deserialize, PartialEq)]
      struct Server {
        host: String,
      }

      #[derive(Debug, Deserialize, PartialEq)]
      struct Nested {
        server: Server,
      }

      let nested: Nested = read(&[("SERVER", "off"), ("SERVER__HOST", "localhost")], || {
        Env::prefixed(PREFIX).split("__")
      });

      assert_eq!(
        nested,
        Nested {
          server: Server {
            host: "localhost".to_owned(),
          },
        }
      );
    }
  }

  mod layering {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reaches_every_corner_of_a_deeply_nested_settings_type() {
      let settings: Settings = temp_env::with_vars(
        vars(&[
          ("DATABASE__POOL__IDLE_TIMEOUT", "45.5"),
          ("DATABASE__URL", "postgres://app@db.internal/compote"),
          ("LOGGING__LEVEL", "error"),
          ("MILESTONES", "2026-01-01, 2026-06-15"),
          ("SERVER__TLS__ENABLED", "1"),
          ("SERVER__TLS__MIN_VERSION", "1.3"),
          ("TAGS", "api, web"),
        ]),
        || {
          Compote::from(Serialized::defaults(Settings::default()))
            .merge(Env::prefixed(PREFIX).split("__"))
            .extract()
            .unwrap()
        },
      );

      assert_eq!(settings.database.pool.idle_timeout, 45.5);
      assert_eq!(settings.database.url, "postgres://app@db.internal/compote");
      assert_eq!(settings.logging.level, Level::Error);
      assert_eq!(settings.milestones, vec!["2026-01-01", "2026-06-15"]);
      assert!(settings.server.tls.enabled);
      assert_eq!(settings.server.tls.min_version.as_deref(), Some("1.3"));
      assert_eq!(settings.tags, vec!["api", "web"]);

      assert_eq!(
        settings.database.pool.max, 8,
        "keys the environment never names keep the default"
      );
      assert_eq!(settings.logging.targets, vec![Target::Stdout]);
      assert_eq!(settings.server.port, 80);
    }
  }
}
