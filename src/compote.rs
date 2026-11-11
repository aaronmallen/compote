use serde::de::DeserializeOwned;

use crate::{Provider, Result, Value};

/// Gathers configuration from any number of sources and hands back one typed value.
///
/// Name the sources in order. Each [`merge`](Compote::merge) beats the one before it, and each
/// [`join`](Compote::join) fills a gap without taking a key that is already set. An error from any
/// source is held until [`extract`](Compote::extract), so the chain never breaks in the middle.
///
/// ```
/// use std::collections::BTreeMap;
///
/// use compote::{Compote, Value};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let defaults = Value::Table(BTreeMap::from([
///   ("host".to_owned(), Value::String("127.0.0.1".to_owned())),
///   ("port".to_owned(), Value::Integer(80)),
/// ]));
/// let file = Value::Table(BTreeMap::from([("port".to_owned(), Value::String("8080".to_owned()))]));
///
/// let settings: Settings = Compote::from(defaults).merge(file).extract().unwrap();
///
/// assert_eq!(settings.host, "127.0.0.1");
/// assert_eq!(settings.port, 8080);
/// ```
#[derive(Debug)]
pub struct Compote {
  state: Result<Value>,
}

impl Compote {
  /// Merges everything gathered so far and reads it into `T`.
  ///
  /// This is where an error from any source in the chain finally surfaces.
  pub fn extract<T>(self) -> Result<T>
  where
    T: DeserializeOwned,
  {
    T::deserialize(self.state?)
  }

  /// Starts a chain from one source.
  ///
  /// The same as [`new`](Compote::new) followed by [`merge`](Compote::merge).
  pub fn from(provider: impl Provider) -> Self {
    Self::new().merge(provider)
  }

  /// Adds a source beneath what is already gathered.
  ///
  /// Keys already set keep their value, and keys not yet seen are filled in. Use this to walk
  /// outward from the nearest configuration file to the furthest without reversing the list.
  pub fn join(self, provider: impl Provider) -> Self {
    let state = match (self.state, provider.data()) {
      (Ok(base), Ok(mut data)) => {
        data.merge(base);

        Ok(data)
      }
      (Err(error), _) | (Ok(_), Err(error)) => Err(error),
    };

    Self {
      state,
    }
  }

  /// Adds a source on top of what is already gathered.
  ///
  /// Tables merge key by key, and anything else replaces what was there.
  pub fn merge(self, provider: impl Provider) -> Self {
    let state = match (self.state, provider.data()) {
      (Ok(mut base), Ok(data)) => {
        base.merge(data);

        Ok(base)
      }
      (Err(error), _) | (Ok(_), Err(error)) => Err(error),
    };

    Self {
      state,
    }
  }

  /// Starts an empty chain.
  pub fn new() -> Self {
    Self {
      state: Ok(Value::table()),
    }
  }
}

impl Default for Compote {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use serde::Deserialize;

  use super::*;
  use crate::Error;

  struct Broken;

  impl Provider for Broken {
    fn data(&self) -> Result<Value> {
      Err(Error::Deserialize("boom".to_owned()))
    }
  }

  #[derive(Debug, Deserialize, PartialEq)]
  struct Settings {
    host: String,
    port: u16,
  }

  fn string(value: &str) -> Value {
    Value::String(value.to_owned())
  }

  fn table(entries: Vec<(&str, Value)>) -> Value {
    Value::Table(
      entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect(),
    )
  }

  mod compote {
    use super::*;

    mod default {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_starts_from_an_empty_table() {
        assert_eq!(Compote::default().extract::<Value>().unwrap(), Value::table());
      }
    }

    mod extract {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_deserializes_the_merged_value() {
        let settings: Settings = Compote::new()
          .merge(table(vec![("host", string("localhost"))]))
          .merge(table(vec![("port", Value::Integer(8080))]))
          .extract()
          .unwrap();

        assert_eq!(
          settings,
          Settings {
            host: "localhost".to_owned(),
            port: 8080,
          }
        );
      }

      #[test]
      fn it_keeps_the_first_error_when_a_later_provider_works() {
        let error = Compote::new()
          .merge(Broken)
          .merge(table(vec![("host", string("localhost")), ("port", Value::Integer(80))]))
          .extract::<Settings>()
          .unwrap_err();

        assert_eq!(error.to_string(), "boom");
      }

      #[test]
      fn it_reports_a_provider_error() {
        let error = Compote::new().merge(Broken).extract::<Settings>().unwrap_err();

        assert_eq!(error.to_string(), "boom");
      }
    }

    mod from {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_starts_from_the_given_provider() {
        let settings: Settings =
          Compote::from(table(vec![("host", string("localhost")), ("port", Value::Integer(80))]))
            .extract()
            .unwrap();

        assert_eq!(
          settings,
          Settings {
            host: "localhost".to_owned(),
            port: 80,
          }
        );
      }
    }

    mod join {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_adds_keys_the_base_is_missing() {
        let settings: Settings = Compote::new()
          .merge(table(vec![("host", string("localhost"))]))
          .join(table(vec![("port", Value::Integer(80))]))
          .extract()
          .unwrap();

        assert_eq!(settings.port, 80);
      }

      #[test]
      fn it_keeps_an_error_already_gathered() {
        let error = Compote::new()
          .merge(Broken)
          .join(table(vec![("host", string("localhost")), ("port", Value::Integer(80))]))
          .extract::<Settings>()
          .unwrap_err();

        assert_eq!(error.to_string(), "boom");
      }

      #[test]
      fn it_keeps_the_value_already_present() {
        let settings: Settings = Compote::new()
          .merge(table(vec![("host", string("child")), ("port", Value::Integer(8080))]))
          .join(table(vec![("host", string("parent"))]))
          .extract()
          .unwrap();

        assert_eq!(settings.host, "child");
      }

      #[test]
      fn it_reports_an_error_from_the_joined_source() {
        let error = Compote::new()
          .merge(table(vec![("host", string("localhost")), ("port", Value::Integer(80))]))
          .join(Broken)
          .extract::<Settings>()
          .unwrap_err();

        assert_eq!(error.to_string(), "boom");
      }
    }

    mod merge {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_lets_the_new_provider_win() {
        let settings: Settings = Compote::new()
          .merge(table(vec![("host", string("parent")), ("port", Value::Integer(80))]))
          .merge(table(vec![("host", string("child")), ("port", Value::Integer(8080))]))
          .extract()
          .unwrap();

        assert_eq!(
          settings,
          Settings {
            host: "child".to_owned(),
            port: 8080,
          }
        );
      }
    }

    mod new {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_starts_from_an_empty_table() {
        assert_eq!(Compote::new().extract::<Value>().unwrap(), Value::table());
      }
    }
  }
}
