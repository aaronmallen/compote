#[cfg(feature = "env")]
mod env;
#[cfg(feature = "json")]
mod json;
mod serialized;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "yaml")]
mod yaml;

#[cfg(feature = "env")]
pub use env::Env;
#[cfg(feature = "json")]
pub use json::Json;
pub use serialized::Serialized;
#[cfg(feature = "toml")]
pub use toml::Toml;
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

#[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
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

  let value = parse(&source).map_err(|source| crate::Error::Parse {
    path: path.to_path_buf(),
    source: Box::new(source),
  })?;

  Ok(match value {
    Value::Null => Value::table(),
    other => other,
  })
}
