#![cfg(feature = "toml")]

mod common;

use compote::{Compote, Toml};
use serde::Deserialize;

use crate::common::{Settings, complete, fixture};

fn settings() -> Settings {
  Compote::from(Toml::path(fixture("complete/config.toml")))
    .extract()
    .unwrap()
}

mod toml {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_file_that_uses_every_shape_toml_offers() {
      assert_eq!(settings(), complete());
    }

    #[test]
    fn it_reads_an_array_of_tables_as_a_list_of_structs() {
      let database = settings().database;

      assert_eq!(database.replicas.len(), 2);
      assert_eq!(database.replicas[0].host, "replica-1.internal");
      assert_eq!(database.replicas[1].port, 5433);
    }

    #[test]
    fn it_reads_dotted_keys_as_a_nested_table() {
      let pool = settings().database.pool;

      assert_eq!(pool.max, 32);
      assert_eq!(pool.min, 4);
      assert_eq!(pool.idle_timeout, 30.5);
    }

    #[test]
    fn it_reads_an_empty_table_header_as_an_empty_map() {
      assert!(settings().extra.is_empty());
    }

    #[test]
    fn it_trims_the_first_newline_of_a_multi_line_string() {
      assert_eq!(settings().name, "Compote — ünïcode ✓");
    }

    #[test]
    fn it_turns_a_bare_date_into_a_string() {
      assert_eq!(settings().released, "2026-08-28");
    }

    #[test]
    fn it_turns_the_dates_inside_a_list_into_strings() {
      assert_eq!(settings().milestones, vec!["2026-01-01", "2026-06-15"]);
    }

    #[test]
    fn it_reads_a_heterogeneous_array_into_an_enum() {
      use crate::common::Target;

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
    fn it_leaves_an_absent_key_as_none() {
      let owners = settings().owners;

      assert_eq!(owners[0].email.as_deref(), Some("hello@aaronmallen.me"));
      assert_eq!(owners[1].email, None);
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Toml::path(fixture("complete/missing.toml")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.toml"), "{error}");
    }

    #[test]
    fn it_keeps_a_literal_string_verbatim() {
      #[derive(Deserialize)]
      struct Database {
        url: String,
      }

      #[derive(Deserialize)]
      struct Partial {
        database: Database,
      }

      let partial: Partial = Compote::from(Toml::path(fixture("complete/config.toml")))
        .extract()
        .unwrap();

      assert_eq!(partial.database.url, "postgres://localhost/compote");
    }
  }
}
