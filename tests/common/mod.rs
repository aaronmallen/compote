//! The settings type every fixture in `tests/fixtures` describes.
//!
//! Each format under `complete/` spells the same configuration in its own idiom, so a format test
//! is a single comparison against [`complete`]. Anything a format cannot say natively it says the
//! nearest way it can, and the typed value it lands on has to match all the others.

// Every test binary pulls in this module, and none of them use all of it.
#![allow(dead_code)]

use std::{
  collections::BTreeMap,
  path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Settings {
  pub database: Database,
  pub extra: BTreeMap<String, String>,
  pub features: BTreeMap<String, Feature>,
  pub logging: Logging,
  pub milestones: Vec<String>,
  pub name: String,
  pub notes: Vec<String>,
  pub owners: Vec<Owner>,
  pub released: String,
  pub server: Server,
  pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Database {
  pub pool: Pool,
  pub replicas: Vec<Replica>,
  pub url: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Pool {
  pub idle_timeout: f64,
  pub max: u32,
  pub min: u32,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Replica {
  pub host: String,
  pub port: u16,
  pub weight: f64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Feature {
  pub enabled: bool,
  pub rollout: f64,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Logging {
  pub level: Level,
  pub targets: Vec<Target>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
  Debug,
  Error,
  Info,
  Warn,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
  File { path: String, rotate: bool },
  Stdout,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Owner {
  pub email: Option<String>,
  pub name: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Server {
  pub backlog: i32,
  pub headers: BTreeMap<String, String>,
  pub host: String,
  pub max_body_bytes: u64,
  pub port: u16,
  pub tls: Tls,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Tls {
  pub ciphers: Vec<String>,
  pub enabled: bool,
  pub min_version: Option<String>,
}

impl Default for Settings {
  fn default() -> Self {
    Self {
      database: Database {
        pool: Pool {
          idle_timeout: 10.0,
          max: 8,
          min: 1,
        },
        replicas: Vec::new(),
        url: "postgres://localhost/postgres".to_owned(),
      },
      extra: BTreeMap::new(),
      features: BTreeMap::new(),
      logging: Logging {
        level: Level::Warn,
        targets: vec![Target::Stdout],
      },
      milestones: Vec::new(),
      name: "compote".to_owned(),
      notes: Vec::new(),
      owners: Vec::new(),
      released: "1970-01-01".to_owned(),
      server: Server {
        backlog: 128,
        headers: BTreeMap::new(),
        host: "127.0.0.1".to_owned(),
        max_body_bytes: 1024,
        port: 80,
        tls: Tls {
          ciphers: Vec::new(),
          enabled: false,
          min_version: None,
        },
      },
      tags: Vec::new(),
    }
  }
}

/// The value every fixture under `tests/fixtures/complete` has to produce.
pub fn complete() -> Settings {
  Settings {
    database: Database {
      pool: Pool {
        idle_timeout: 30.5,
        max: 32,
        min: 4,
      },
      replicas: vec![
        Replica {
          host: "replica-1.internal".to_owned(),
          port: 5432,
          weight: 0.75,
        },
        Replica {
          host: "replica-2.internal".to_owned(),
          port: 5433,
          weight: 0.25,
        },
      ],
      url: "postgres://localhost/compote".to_owned(),
    },
    extra: BTreeMap::new(),
    features: map(vec![
      (
        "beta-ui",
        Feature {
          enabled: true,
          rollout: 0.05,
        },
      ),
      (
        "metrics",
        Feature {
          enabled: true,
          rollout: 1.0,
        },
      ),
      (
        "tracing",
        Feature {
          enabled: false,
          rollout: 0.0,
        },
      ),
    ]),
    logging: Logging {
      level: Level::Info,
      targets: vec![
        Target::Stdout,
        Target::File {
          path: "/var/log/compote.log".to_owned(),
          rotate: true,
        },
      ],
    },
    milestones: vec!["2026-01-01".to_owned(), "2026-06-15".to_owned()],
    name: "Compote — ünïcode ✓".to_owned(),
    notes: Vec::new(),
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
    released: "2026-08-28".to_owned(),
    server: Server {
      backlog: -1,
      headers: strings(vec![("x-greeting", "hello\tworld"), ("x-powered-by", "compote")]),
      host: "0.0.0.0".to_owned(),
      max_body_bytes: 4_294_967_296,
      port: 8443,
      tls: Tls {
        ciphers: vec!["TLS_AES_128_GCM_SHA256".to_owned(), "TLS_AES_256_GCM_SHA384".to_owned()],
        enabled: true,
        min_version: Some("1.3".to_owned()),
      },
    },
    tags: vec!["api".to_owned(), "web".to_owned()],
  }
}

/// The path to a file under `tests/fixtures`.
pub fn fixture(name: &str) -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

pub fn map<T>(entries: Vec<(&str, T)>) -> BTreeMap<String, T> {
  entries
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect()
}

pub fn strings(entries: Vec<(&str, &str)>) -> BTreeMap<String, String> {
  entries
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect()
}
