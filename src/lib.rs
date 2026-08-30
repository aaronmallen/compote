//! Layered configuration.
//!
//! Compote reads configuration from files and the environment, merges it in the order you choose, and
//! returns one typed value. You give it the sources. It does not go looking for them, so where your
//! configuration lives, and which file wins, stays your decision.
//!
//! ```
//! use std::collections::BTreeMap;
//!
//! use compote::{Compote, Serialized, Value};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize, Serialize)]
//! struct Settings {
//!   host: String,
//!   port: u16,
//! }
//!
//! impl Default for Settings {
//!   fn default() -> Self {
//!     Self { host: "127.0.0.1".to_owned(), port: 80 }
//!   }
//! }
//!
//! let from_file = Value::Table(BTreeMap::from([("port".to_owned(), Value::String("8080".to_owned()))]));
//!
//! let settings: Settings = Compote::from(Serialized::defaults(Settings::default()))
//!   .merge(from_file)
//!   .extract()
//!   .unwrap();
//!
//! assert_eq!(settings.host, "127.0.0.1");
//! assert_eq!(settings.port, 8080);
//! ```
//!
//! # Order
//!
//! Every [`merge`](Compote::merge) beats the one before it. Every [`join`](Compote::join) fills a gap
//! without taking a key that is already set, which is what you want when walking outward from the
//! nearest configuration file to the furthest.
//!
//! # Coercion
//!
//! A string becomes whatever the field asks for, so `"8080"` fills a `u16` and `"yes"` fills a `bool`,
//! and a string holding nothing fills an empty list or an empty map. It does not work the other way,
//! so a number never quietly fills a `String` field and hides a mistake. This is what lets
//! environment variables, which are always text, sit beside typed files.
//!
//! # Formats
//!
//! Each format sits behind a feature of the same name, and none are on by default.
//!
//! [`Cbor`] and [`MsgPack`] are the binary formats, for a file something else writes rather than a
//! person. Both want string keys, and both refuse raw bytes and tagged or extension values rather
//! than guessing at them. [`MsgPack`] wants its keys from `rmp_serde::to_vec_named` rather than the
//! compact `rmp_serde::to_vec`, which turns a struct into an array of values in declaration order
//! and leaves no field names to merge on.
//!
//! [`Dotenv`] is [`Env`] kept in a file: the same shell names, lowercased and nested the same way,
//! so one `.env` file lands the same whether this reads it or the shell sourced it first. A name
//! holds one value and saying it twice replaces it, which is all an environment variable can be. A
//! name is a shell variable name, so a key with a hyphen in it has no spelling.
//!
//! [`Ini`] is the one file format that is only text, which is the model the environment already
//! uses and the one coercion was built for. A section is a table, and past that nothing nests until
//! you say [`split`](Ini::split), since INI has no depth of its own to borrow. A key said twice is
//! the list it has no other way to spell.
//!
//! [`Xml`] is the other format that is only text, and the one that gets the most out of it. A child
//! element is a key and an attribute is a key beside it, so `<server port="8443"/>` fills a struct.
//! An element said twice is a list, and because a repeated element brings its own children along it
//! is a list of tables as readily as a list of strings, which is the one thing [`Ini`] cannot say.
//!
//! [`Json`] covers both spellings, since JSON with comments is a superset of JSON. Comments and
//! trailing commas are allowed and nothing else is, so the extension decides nothing and a file that
//! uses neither is still held to strict JSON. Say [`strict`](Json::strict) to refuse even those, or
//! [`lenient`](Json::lenient) to take everything the parser knows, and name any single syntax to
//! allow or deny it on its own.
//!
//! | Feature | Reads | Parser |
//! | --- | --- | --- |
//! | `cbor` | `.cbor` | `ciborium` |
//! | `dotenv` | `.env` | `dotenvy` |
//! | `env` | environment variables | |
//! | `ini` | `.ini` | `rust-ini` |
//! | `json` | `.json`, `.jsonc` | `jsonc-parser` |
//! | `msgpack` | `.msgpack`, `.mpk` | `rmp-serde` |
//! | `toml` | `.toml` | `toml_edit` |
//! | `xml` | `.xml` | `roxmltree` |
//! | `yaml` | `.yaml`, `.yml` | `yaml_serde` |
#![warn(missing_docs)]

mod compote;
mod error;
mod provider;
mod value;

pub use compote::Compote;
pub use error::{Error, Result};
#[cfg(feature = "cbor")]
pub use provider::Cbor;
#[cfg(feature = "dotenv")]
pub use provider::Dotenv;
#[cfg(feature = "env")]
pub use provider::Env;
#[cfg(feature = "ini")]
pub use provider::Ini;
#[cfg(feature = "json")]
pub use provider::Json;
#[cfg(feature = "msgpack")]
pub use provider::MsgPack;
#[cfg(feature = "toml")]
pub use provider::Toml;
#[cfg(feature = "xml")]
pub use provider::Xml;
#[cfg(feature = "yaml")]
pub use provider::Yaml;
pub use provider::{Provider, Serialized};
pub use value::Value;
