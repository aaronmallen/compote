#![cfg(feature = "yaml")]

mod common;

use compote::{Compote, Yaml};
use serde::Deserialize;

use crate::common::{Settings, complete, fixture};

fn settings() -> Settings {
  Compote::from(Yaml::path(fixture("complete/config.yaml")))
    .extract()
    .unwrap()
}

mod yaml {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_reads_a_document_that_uses_anchors_folding_and_flow_style() {
      assert_eq!(settings(), complete());
    }

    #[test]
    fn it_resolves_an_alias_to_the_value_it_points_at() {
      assert_eq!(
        settings().server.tls.ciphers,
        vec!["TLS_AES_128_GCM_SHA256", "TLS_AES_256_GCM_SHA384"]
      );
      assert_eq!(settings().database.replicas[0].weight, 0.75);
    }

    #[test]
    fn it_folds_a_folded_scalar_and_strips_its_trailing_newline() {
      assert_eq!(settings().name, "Compote — ünïcode ✓");
    }

    #[test]
    fn it_reads_flow_style_the_same_as_block_style() {
      let replicas = settings().database.replicas;

      assert_eq!(replicas[0].host, "replica-1.internal");
      assert_eq!(replicas[1].host, "replica-2.internal");
      assert_eq!(settings().tags, vec!["api", "web"]);
    }

    #[test]
    fn it_reads_a_tilde_as_none() {
      assert_eq!(settings().owners[1].email, None);
    }

    #[test]
    fn it_ignores_a_key_the_settings_type_does_not_declare() {
      assert_eq!(settings(), complete());
    }

    #[test]
    fn it_reads_the_yml_extension_too() {
      #[derive(Debug, Deserialize, PartialEq)]
      struct Tls {
        enabled: bool,
      }

      #[derive(Debug, Deserialize, PartialEq)]
      struct Server {
        host: String,
        port: u16,
        tls: Tls,
      }

      #[derive(Debug, Deserialize, PartialEq)]
      struct Partial {
        server: Server,
      }

      let partial: Partial = Compote::from(Yaml::path(fixture("complete/config.yml")))
        .extract()
        .unwrap();

      assert_eq!(
        partial,
        Partial {
          server: Server {
            host: "0.0.0.0".to_owned(),
            port: 8443,
            tls: Tls {
              enabled: true,
            },
          },
        }
      );
    }

    #[test]
    fn it_reports_the_path_of_a_file_it_cannot_read() {
      let error = Compote::from(Yaml::path(fixture("complete/missing.yaml")))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("missing.yaml"), "{error}");
    }
  }
}
