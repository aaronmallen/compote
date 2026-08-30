use std::collections::BTreeMap;

use keyring_core::{Entry, Error as KeyringError};

use crate::{Error, Provider, Result, Value};

/// Configuration read from the platform's keyring.
///
/// A keyring holds one secret per name, so this is a list of them: each secret you name lands at
/// the key you give it, and everything else in the configuration comes from somewhere else. Nothing
/// is read until the source is merged, and it is read again each time it is, so a secret rotated
/// underneath a long-running process is picked up on the next read.
///
/// ```no_run
/// use compote::{Compote, Keyring, Toml};
/// # use serde::Deserialize;
/// # #[derive(Deserialize)]
/// # struct Settings { database: Database }
/// # #[derive(Deserialize)]
/// # struct Database { url: String }
///
/// let settings: Settings = Compote::from(Toml::path("config.toml"))
///   .merge(Keyring::service("compote").secret_named("database.url", "db-url").split("."))
///   .extract()
///   .unwrap();
/// ```
///
/// A secret is text, and the field it lands on coerces it on the way out, so a stored `8443` fills
/// a `u16` the way an environment variable does.
///
/// Keys are flat until you call [`split`](Keyring::split), which is the same choice
/// [`Env`](crate::Env) and [`Dotenv`](crate::Dotenv) make. Unlike those, a name here is used exactly
/// as you write it: you typed it yourself rather than inheriting it from a shell, so there is
/// nothing to lowercase.
///
/// # Installing a store
///
/// Compote does not choose where your secrets live, the same way it does not go looking for your
/// configuration files. It reads through whatever credential store your application has installed
/// with [`keyring_core::set_default_store`], and until something installs one every read fails with
/// `NoDefaultStore`.
///
/// Nothing here is tied to an operating system. This crate links no credential store at all, so
/// which platforms you support is a question about what you install rather than about this
/// provider.
///
/// The usual way to get the platform's own store is the [`keyring`](https://crates.io/crates/keyring)
/// crate with its `v1` feature, whose `Entry::store_status()` installs it and reports whether it
/// came up:
///
/// ```ignore
/// keyring::Entry::store_status().as_ref().expect("no platform keyring");
/// ```
///
/// That covers three platforms and refuses the rest rather than guessing:
///
/// | Target | Store |
/// | --- | --- |
/// | macOS | Keychain |
/// | Windows | Credential Manager |
/// | Linux, and other non-Apple Unix | Secret Service, over D-Bus |
///
/// Anywhere else, `v1` fails with `NoDefaultStore` and the store has to be named directly. Each one
/// installs the same way, and the choice is yours to make rather than the platform's:
///
/// ```ignore
/// keyring_core::set_default_store(linux_keyutils_keyring_store::Store::new()?);
/// ```
///
/// `apple_native_keyring_store::protected::Store` covers iOS, `android_native_keyring_store::Store`
/// covers Android, and `db_keystore` is a file on any 64-bit target. The one worth knowing about
/// even where `v1` works is `linux_keyutils_keyring_store::Store`: a headless Linux has no D-Bus
/// session and so no Secret Service to talk to, which is a container or a CI runner failing at
/// runtime rather than at compile time.
///
/// Anything implementing `keyring_core::CredentialStore` will do, which is what lets a test stand a
/// mock store up in place of the real one.
///
/// # When a secret is missing
///
/// A secret you named and the store does not have is an error, because naming it said you expected
/// it. [`optional`](Keyring::optional) turns that into an absent key instead, which lays over the
/// layer beneath without taking anything from it, for the machine that has not been given the
/// secrets yet.
pub struct Keyring {
  optional: bool,
  secrets: Vec<Secret>,
  separator: String,
  service: String,
}

impl Keyring {
  /// Reads a secret the store does not have as an absent key rather than an error.
  ///
  /// For the developer's machine, where the file beneath this one holds a stand-in and nobody has
  /// provisioned the real thing. A store that fails for any other reason is still an error, so this
  /// hides a secret that was never set and never a keyring that will not open.
  pub fn optional(mut self) -> Self {
    self.optional = true;

    self
  }

  /// Reads a secret the store does not have as an error. On unless [`optional`](Keyring::optional)
  /// says otherwise.
  pub fn required(mut self) -> Self {
    self.optional = false;

    self
  }

  /// Reads the secret stored under `key`, and puts it at `key`.
  ///
  /// For the store whose names are already the names your configuration uses. See
  /// [`secret_named`](Keyring::secret_named) when they differ.
  pub fn secret(mut self, key: &str) -> Self {
    self.secrets.push(Secret {
      key: key.to_owned(),
      user: key.to_owned(),
    });

    self
  }

  /// Reads the secret stored under `user`, and puts it at `key`.
  ///
  /// A keyring name is often shorter and flatter than the path it fills, so
  /// `secret_named("database.url", "db-url")` reads `db-url` from the store and lands it at
  /// `database.url`.
  pub fn secret_named(mut self, key: &str, user: &str) -> Self {
    self.secrets.push(Secret {
      key: key.to_owned(),
      user: user.to_owned(),
    });

    self
  }

  /// Reads from the keyring named `service`.
  ///
  /// Names no secrets on its own, so this alone reads an empty table. Add them with
  /// [`secret`](Keyring::secret) and [`secret_named`](Keyring::secret_named).
  pub fn service(service: &str) -> Self {
    Self {
      optional: false,
      secrets: Vec::new(),
      separator: String::new(),
      service: service.to_owned(),
    }
  }

  /// Nests keys wherever `separator` appears.
  ///
  /// A nested key beats a scalar already sitting at the same path.
  pub fn split(mut self, separator: &str) -> Self {
    self.separator = separator.to_owned();

    self
  }

  /// Reads one secret, and reports the name it was looked for under when that fails.
  fn read(&self, user: &str) -> Result<Option<String>> {
    let found = Entry::new(&self.service, user).and_then(|entry| entry.get_password());

    match found {
      Ok(password) => Ok(Some(password)),
      Err(KeyringError::NoEntry) if self.optional => Ok(None),
      Err(source) => Err(Error::Secret {
        service: self.service.clone(),
        source: Box::new(source),
        user: user.to_owned(),
      }),
    }
  }
}

impl Provider for Keyring {
  fn data(&self) -> Result<Value> {
    let mut table = BTreeMap::new();

    for secret in &self.secrets {
      if let Some(password) = self.read(&secret.user)? {
        super::insert(&mut table, &secret.key, &self.separator, password);
      }
    }

    Ok(Value::Table(table))
  }
}

/// One secret: where it lives in the store, and where it lands in the configuration.
struct Secret {
  key: String,
  user: String,
}

#[cfg(test)]
mod tests {
  use std::sync::Once;

  use super::*;

  /// Stands a mock store up in place of the platform's, once for the whole test binary.
  ///
  /// The default store is process-wide, so every test shares this one and takes a service name of
  /// its own rather than a store of its own.
  fn store() {
    static INSTALLED: Once = Once::new();

    INSTALLED.call_once(|| {
      keyring_core::set_default_store(keyring_core::mock::Store::new().expect("the mock store stands up"));
    });
  }

  fn write(service: &str, user: &str, password: &str) {
    store();

    Entry::new(service, user)
      .expect("the mock store is installed")
      .set_password(password)
      .expect("the mock store takes a password");
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

  mod keyring {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_a_secret_as_text() {
        write("compote-data-text", "port", "8443");

        assert_eq!(
          Keyring::service("compote-data-text").secret("port").data().unwrap(),
          table(vec![("port", string("8443"))])
        );
      }

      #[test]
      fn it_reads_every_secret_it_was_given() {
        write("compote-data-every", "host", "localhost");
        write("compote-data-every", "port", "8443");

        assert_eq!(
          Keyring::service("compote-data-every")
            .secret("host")
            .secret("port")
            .data()
            .unwrap(),
          table(vec![("host", string("localhost")), ("port", string("8443"))])
        );
      }

      #[test]
      fn it_reads_an_empty_table_when_no_secret_was_named() {
        store();

        assert_eq!(Keyring::service("compote-data-none").data().unwrap(), Value::table());
      }

      #[test]
      fn it_keeps_two_services_apart() {
        write("compote-data-first", "token", "first");
        write("compote-data-second", "token", "second");

        assert_eq!(
          Keyring::service("compote-data-first").secret("token").data().unwrap(),
          table(vec![("token", string("first"))])
        );
      }

      #[test]
      fn it_reports_the_name_a_missing_secret_was_looked_for_under() {
        store();

        let error = Keyring::service("compote-data-missing")
          .secret("db-url")
          .data()
          .unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
        assert!(error.to_string().contains("db-url"), "{error}");
        assert!(error.to_string().contains("compote-data-missing"), "{error}");
      }
    }

    mod optional {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_leaves_a_missing_secret_out_rather_than_failing() {
        write("compote-optional", "host", "localhost");

        assert_eq!(
          Keyring::service("compote-optional")
            .secret("host")
            .secret("port")
            .optional()
            .data()
            .unwrap(),
          table(vec![("host", string("localhost"))]),
          "the one that is there arrives, and the one that is not is simply absent"
        );
      }
    }

    mod required {
      use super::*;

      #[test]
      fn it_takes_the_forgiveness_back_away() {
        store();

        let error = Keyring::service("compote-required")
          .secret("token")
          .optional()
          .required()
          .data()
          .unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
      }
    }

    mod secret {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_uses_the_key_as_the_name_in_the_store() {
        write("compote-secret", "database.url", "postgres://localhost/compote");

        assert_eq!(
          Keyring::service("compote-secret")
            .secret("database.url")
            .data()
            .unwrap(),
          table(vec![("database.url", string("postgres://localhost/compote"))]),
          "one key with a dot in it, since nothing nests until split"
        );
      }
    }

    mod secret_named {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_one_name_and_lands_on_another() {
        write("compote-named", "db-url", "postgres://localhost/compote");

        assert_eq!(
          Keyring::service("compote-named")
            .secret_named("database.url", "db-url")
            .split(".")
            .data()
            .unwrap(),
          table(vec![(
            "database",
            table(vec![("url", string("postgres://localhost/compote"))])
          )])
        );
      }
    }

    mod split {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_nests_the_key_on_the_separator() {
        write("compote-split", "server.tls.key", "----BEGIN----");

        assert_eq!(
          Keyring::service("compote-split")
            .secret("server.tls.key")
            .split(".")
            .data()
            .unwrap(),
          table(vec![(
            "server",
            table(vec![("tls", table(vec![("key", string("----BEGIN----"))]))])
          )])
        );
      }

      #[test]
      fn it_keeps_the_key_whole_until_a_separator_is_set() {
        write("compote-split-off", "server.tls.key", "----BEGIN----");

        assert_eq!(
          Keyring::service("compote-split-off")
            .secret("server.tls.key")
            .data()
            .unwrap(),
          table(vec![("server.tls.key", string("----BEGIN----"))])
        );
      }
    }
  }
}
