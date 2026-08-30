#[cfg(feature = "cbor")]
mod cbor;
#[cfg(feature = "env")]
mod env;
#[cfg(feature = "ini")]
mod ini;
#[cfg(feature = "json")]
mod json;
#[cfg(feature = "msgpack")]
mod msgpack;
mod serialized;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "xml")]
mod xml;
#[cfg(feature = "yaml")]
mod yaml;

#[cfg(feature = "cbor")]
pub use cbor::Cbor;
#[cfg(feature = "env")]
pub use env::Env;
#[cfg(feature = "ini")]
pub use ini::Ini;
#[cfg(feature = "json")]
pub use json::Json;
#[cfg(feature = "msgpack")]
pub use msgpack::MsgPack;
pub use serialized::Serialized;
#[cfg(feature = "toml")]
pub use toml::Toml;
#[cfg(feature = "xml")]
pub use xml::Xml;
#[cfg(feature = "yaml")]
pub use yaml::Yaml;

use crate::{Result, Value};

/// A source of configuration.
///
/// Implement this to teach [`Compote`](crate::Compote) to read from somewhere it does not already
/// know about.
///
/// ```
/// use compote::{Provider, Result, Value};
///
/// struct Fixed;
///
/// impl Provider for Fixed {
///   fn data(&self) -> Result<Value> {
///     Ok(Value::String("hello".to_owned()))
///   }
/// }
/// ```
pub trait Provider {
  /// Reads this source and returns what it holds.
  ///
  /// Called once for every [`merge`](crate::Compote::merge) or [`join`](crate::Compote::join), and
  /// may be called more than once for the same source.
  fn data(&self) -> Result<Value>;
}

/// Names the file a parser failed on, and reads a document holding nothing as an empty table.
///
/// A source that says only `null` has nothing to contribute to a merge, which is the same thing an
/// absent key says, so it lays over the layer beneath without taking anything from it.
#[cfg(any(
  feature = "cbor",
  feature = "ini",
  feature = "json",
  feature = "msgpack",
  feature = "toml",
  feature = "xml",
  feature = "yaml"
))]
fn finish<E>(path: &std::path::Path, parsed: std::result::Result<Value, E>) -> Result<Value>
where
  E: std::error::Error + Send + Sync + 'static,
{
  let value = parsed.map_err(|source| crate::Error::Parse {
    path: path.to_path_buf(),
    source: Box::new(source),
  })?;

  Ok(match value {
    Value::Null => Value::table(),
    other => other,
  })
}

/// Reads a text file and hands its contents to `parse`.
#[cfg(any(
  feature = "ini",
  feature = "json",
  feature = "toml",
  feature = "xml",
  feature = "yaml"
))]
fn load<E>(path: &std::path::Path, parse: impl FnOnce(&str) -> std::result::Result<Value, E>) -> Result<Value>
where
  E: std::error::Error + Send + Sync + 'static,
{
  let source = std::fs::read_to_string(path).map_err(|source| crate::Error::Read {
    path: path.to_path_buf(),
    source,
  })?;

  if source.trim().is_empty() {
    return Ok(Value::table());
  }

  finish(path, parse(&source))
}

/// Reads a binary file and hands its bytes to `parse`.
#[cfg(any(feature = "cbor", feature = "msgpack"))]
fn load_bytes<E>(path: &std::path::Path, parse: impl FnOnce(&[u8]) -> std::result::Result<Value, E>) -> Result<Value>
where
  E: std::error::Error + Send + Sync + 'static,
{
  let source = std::fs::read(path).map_err(|source| crate::Error::Read {
    path: path.to_path_buf(),
    source,
  })?;

  if source.is_empty() {
    return Ok(Value::table());
  }

  finish(path, parse(&source))
}
