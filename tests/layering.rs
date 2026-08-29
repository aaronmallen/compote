#![cfg(all(feature = "json", feature = "toml"))]

use std::io::Write;

use ::compote::{Compote, Json, Serialized, Toml};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Settings {
  host: String,
  port: u16,
  tags: Vec<String>,
  tls: bool,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      host: "127.0.0.1".to_owned(),
      port: 80,
      tags: Vec::new(),
      tls: false,
    }
  }
}

fn file(source: &str, extension: &str) -> NamedTempFile {
  let mut handle = tempfile::Builder::new().suffix(extension).tempfile().unwrap();
  handle.write_all(source.as_bytes()).unwrap();

  handle
}

mod compote {
  use super::*;

  mod extract {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_carries_a_parse_failure_to_the_end_of_the_chain() {
      let broken = file("{ nope", ".json");

      let error = Compote::from(Serialized::defaults(Settings::default()))
        .merge(Json::path(broken.path()))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to parse"), "{error}");
    }

    #[test]
    fn it_lets_each_layer_beat_the_one_before_it() {
      let parent = file("host = \"parent\"\nport = 8080\n", ".toml");
      let child = file(r#"{"host": "child", "tags": "web, api"}"#, ".json");

      let settings: Settings = Compote::from(Serialized::defaults(Settings::default()))
        .merge(Toml::path(parent.path()))
        .merge(Json::path(child.path()))
        .extract()
        .unwrap();

      assert_eq!(
        settings,
        Settings {
          host: "child".to_owned(),
          port: 8080,
          tags: vec!["web".to_owned(), "api".to_owned()],
          tls: false,
        }
      );
    }

    #[test]
    fn it_lets_join_fill_a_gap_without_taking_a_key_that_is_set() {
      let parent = file("host = \"parent\"\nport = 8080\n", ".toml");
      let child = file(r#"{"host": "child", "tags": "web", "tls": true}"#, ".json");

      let settings: Settings = Compote::from(Json::path(child.path()))
        .join(Toml::path(parent.path()))
        .extract()
        .unwrap();

      assert_eq!(
        settings,
        Settings {
          host: "child".to_owned(),
          port: 8080,
          tags: vec!["web".to_owned()],
          tls: true,
        }
      );
    }
  }
}
