use serde::Serialize;

use crate::{Error, Provider, Result, Value, value::ser};

/// Configuration read from any value that can be serialized.
///
/// The usual way to supply defaults: hand it your settings type's [`Default`], name it first in the
/// chain, and let every later source override what it needs to.
///
/// ```
/// use compote::{Compote, Provider, Serialized};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Deserialize, Serialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// impl Default for Settings {
///   fn default() -> Self {
///     Self { host: "127.0.0.1".to_owned(), port: 80 }
///   }
/// }
///
/// let settings: Settings = Compote::from(Serialized::defaults(Settings::default()))
///   .extract()
///   .unwrap();
///
/// assert_eq!(settings.port, 80);
/// ```
pub struct Serialized {
  state: std::result::Result<Value, String>,
}

impl Serialized {
  /// Reads `value` now and holds the result.
  ///
  /// Serializing happens once, here. If it fails, the failure is reported every time this source is
  /// read, and surfaces at [`extract`](crate::Compote::extract) like any other.
  pub fn defaults(value: impl Serialize) -> Self {
    Self {
      state: ser::to_value(value).map_err(|error| error.to_string()),
    }
  }
}

impl Provider for Serialized {
  fn data(&self) -> Result<Value> {
    self.state.clone().map_err(Error::Serialize)
  }
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use super::*;

  #[derive(Serialize)]
  struct Broken {
    entries: BTreeMap<(u8, u8), u8>,
  }

  #[derive(Serialize)]
  struct Settings {
    host: String,
    port: u16,
  }

  fn settings() -> Settings {
    Settings {
      host: "localhost".to_owned(),
      port: 8080,
    }
  }

  mod serialized {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_can_be_read_more_than_once() {
        let provider = Serialized::defaults(settings());

        assert_eq!(provider.data().unwrap(), provider.data().unwrap());
      }

      #[test]
      fn it_reports_a_value_that_cannot_be_serialized() {
        let provider = Serialized::defaults(Broken {
          entries: BTreeMap::from([((1, 2), 3)]),
        });

        let error = provider.data().unwrap_err();

        assert!(error.to_string().contains("must be a string or a scalar"), "{error}");
      }
    }

    mod defaults {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_turns_the_value_into_a_table() {
        assert_eq!(
          Serialized::defaults(settings()).data().unwrap(),
          Value::Table(BTreeMap::from([
            ("host".to_owned(), Value::String("localhost".to_owned())),
            ("port".to_owned(), Value::Integer(8080)),
          ]))
        );
      }
    }
  }
}
