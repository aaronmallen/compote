use std::collections::BTreeMap;

use crate::{Provider, Result, Value};

/// Configuration read from environment variables.
///
/// Only variables carrying the prefix are read, and the prefix is stripped from what remains. Names
/// are lowercased. Every value arrives as text, and the target type coerces it on the way out, so
/// `APP_PORT=8080` fills a `u16` field and `APP_TLS=yes` fills a `bool`.
///
/// Nothing nests until you call [`split`](Env::split). With `split("__")` the variable
/// `APP_SERVER__HOST` becomes the key `host` inside the table `server`.
///
/// ```
/// use compote::Env;
///
/// let env = Env::prefixed("APP_").ignore(&["CONFIG"]).split("__");
/// ```
pub struct Env {
  ignore: Vec<String>,
  prefix: String,
  separator: String,
}

impl Env {
  /// Drops the named keys.
  ///
  /// Names are matched after the prefix comes off, and case is ignored, so `ignore(&["CONFIG"])`
  /// drops `APP_CONFIG`. Use it to keep a variable that points at your configuration from becoming
  /// part of it.
  pub fn ignore(mut self, keys: &[&str]) -> Self {
    self.ignore = keys.iter().map(|key| key.to_ascii_lowercase()).collect();

    self
  }

  /// Reads the variables whose names start with `prefix`.
  ///
  /// Keys are flat until you call [`split`](Env::split).
  pub fn prefixed(prefix: &str) -> Self {
    Self {
      ignore: Vec::new(),
      prefix: prefix.to_owned(),
      separator: String::new(),
    }
  }

  /// Nests keys wherever `separator` appears.
  ///
  /// A nested key beats a scalar already sitting at the same path.
  pub fn split(mut self, separator: &str) -> Self {
    self.separator = separator.to_owned();

    self
  }

  fn overlay(&self, vars: impl IntoIterator<Item = (String, String)>) -> Value {
    let mut pairs: Vec<(String, String)> = vars.into_iter().collect();
    pairs.sort();

    let mut table = BTreeMap::new();

    for (key, value) in pairs {
      let Some(name) = key.strip_prefix(&self.prefix).map(str::to_ascii_lowercase) else {
        continue;
      };

      if name.is_empty() || self.ignore.contains(&name) {
        continue;
      }

      insert(&mut table, &name, &self.separator, value);
    }

    Value::Table(table)
  }
}

impl Provider for Env {
  fn data(&self) -> Result<Value> {
    let vars =
      std::env::vars_os().filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)));

    Ok(self.overlay(vars))
  }
}

fn insert(table: &mut BTreeMap<String, Value>, key: &str, separator: &str, value: String) {
  let segments: Vec<&str> = if separator.is_empty() {
    vec![key]
  } else {
    key.split(separator).filter(|segment| !segment.is_empty()).collect()
  };

  let Some((leaf, parents)) = segments.split_last() else {
    return;
  };

  let mut current = table;

  for segment in parents {
    let entry = current.entry((*segment).to_owned()).or_insert_with(Value::table);

    if !matches!(entry, Value::Table(_)) {
      *entry = Value::table();
    }

    let Value::Table(nested) = entry else {
      unreachable!("the entry was just replaced with a table")
    };

    current = nested;
  }

  current.insert((*leaf).to_owned(), Value::String(value));
}

#[cfg(test)]
mod tests {
  use super::*;

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

  fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
      .iter()
      .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
      .collect()
  }

  mod env {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_the_process_environment() {
        // SAFETY: every test runs in its own process, so nothing else is reading the environment.
        unsafe {
          std::env::set_var("COMPOTE_ENV_DATA_HOST", "localhost");
        }

        assert_eq!(
          Env::prefixed("COMPOTE_ENV_DATA_").data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }
    }

    mod ignore {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_drops_the_named_keys_whatever_their_case() {
        let provider = Env::prefixed("MY_CRATE_").ignore(&["CONFIG"]);

        assert_eq!(
          provider.overlay(vars(&[
            ("MY_CRATE_CONFIG", "/etc/my_crate"),
            ("MY_CRATE_HOST", "localhost")
          ])),
          table(vec![("host", string("localhost"))])
        );
      }
    }

    mod overlay {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_keys_flat_until_a_separator_is_set() {
        assert_eq!(
          Env::prefixed("MY_CRATE_").overlay(vars(&[("MY_CRATE_SERVER__HOST", "localhost")])),
          table(vec![("server__host", string("localhost"))])
        );
      }

      #[test]
      fn it_lowercases_the_key_it_keeps() {
        assert_eq!(
          Env::prefixed("MY_CRATE_").overlay(vars(&[("MY_CRATE_HOST", "localhost")])),
          table(vec![("host", string("localhost"))])
        );
      }

      #[test]
      fn it_skips_a_bare_prefix() {
        assert_eq!(
          Env::prefixed("MY_CRATE_").overlay(vars(&[("MY_CRATE_", "x")])),
          Value::table()
        );
      }

      #[test]
      fn it_skips_vars_without_the_prefix() {
        assert_eq!(
          Env::prefixed("MY_CRATE_").overlay(vars(&[("PATH", "/usr/bin"), ("MY_CRATE_HOST", "localhost")])),
          table(vec![("host", string("localhost"))])
        );
      }
    }

    mod split {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_lets_a_nested_key_win_over_a_scalar_at_the_same_path() {
        let provider = Env::prefixed("MY_CRATE_").split("__");
        let value = provider.overlay(vars(&[
          ("MY_CRATE_SERVER", "off"),
          ("MY_CRATE_SERVER__HOST", "localhost"),
        ]));

        assert_eq!(
          value,
          table(vec![("server", table(vec![("host", string("localhost"))]))])
        );
      }

      #[test]
      fn it_nests_on_the_separator() {
        let provider = Env::prefixed("MY_CRATE_").split("__");

        assert_eq!(
          provider.overlay(vars(&[("MY_CRATE_SERVER__HOST", "localhost")])),
          table(vec![("server", table(vec![("host", string("localhost"))]))])
        );
      }
    }
  }

  mod insert {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_drops_empty_segments() {
      let mut entries = BTreeMap::new();

      insert(&mut entries, "server____host", "__", "localhost".to_owned());

      assert_eq!(
        Value::Table(entries),
        table(vec![("server", table(vec![("host", string("localhost"))]))])
      );
    }

    #[test]
    fn it_ignores_a_key_that_is_nothing_but_separators() {
      let mut entries = BTreeMap::new();

      insert(&mut entries, "__", "__", "localhost".to_owned());

      assert_eq!(Value::Table(entries), Value::table());
    }
  }
}
