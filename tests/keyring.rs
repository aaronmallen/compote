//! The keyring is the layer that never goes in a file.
//!
//! It is not a whole configuration and there is no fixture for it. A keyring holds one secret per
//! name, so what it does is replace the handful of values a committed file has to lie about: the
//! database URL without its password, the token that is not there at all. Everything else comes
//! from the file underneath, and the merge does not care that the two came from different places.

#![cfg(all(feature = "keyring", feature = "toml"))]

mod common;

use std::sync::Once;

use ::compote::{Compote, Keyring, Toml};

use crate::common::{Settings, complete, fixture};

const SERVICE: &str = "compote-integration";

/// Stands a mock store up in place of the platform's, once for the whole test binary.
///
/// Compote does not choose where secrets live, which is what lets this put a store of its own in
/// front of the real keyring rather than writing to the machine running the tests.
fn store() {
  static INSTALLED: Once = Once::new();

  INSTALLED.call_once(|| {
    keyring_core::set_default_store(keyring_core::mock::Store::new().expect("the mock store stands up"));

    for (user, password) in [
      ("db-url", "postgres://app:s3cret@db.internal/compote"),
      ("tls-min-version", "1.2"),
    ] {
      keyring_core::Entry::new(SERVICE, user)
        .expect("the mock store is installed")
        .set_password(password)
        .expect("the mock store takes a password");
    }
  });
}

fn settings() -> Settings {
  store();

  Compote::from(Toml::path(fixture("complete/config.toml")))
    .merge(
      Keyring::service(SERVICE)
        .secret_named("database.url", "db-url")
        .secret_named("server.tls.min_version", "tls-min-version")
        .split("."),
    )
    .extract()
    .unwrap()
}

mod keyring {
  use super::*;

  mod data {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_replaces_the_values_the_committed_file_cannot_hold() {
      assert_eq!(
        settings().database.url,
        "postgres://app:s3cret@db.internal/compote",
        "the file names a database, the keyring names one with credentials"
      );
      assert_eq!(settings().server.tls.min_version.as_deref(), Some("1.2"));
    }

    #[test]
    fn it_leaves_everything_it_did_not_name_alone() {
      let settings = settings();

      assert_eq!(settings.database.pool, complete().database.pool);
      assert_eq!(settings.database.replicas, complete().database.replicas);
      assert_eq!(settings.server.port, 8443);
      assert_eq!(settings.owners, complete().owners);
      assert_eq!(settings.features, complete().features);
    }

    #[test]
    fn it_coerces_a_secret_onto_the_field_that_asks_for_it() {
      store();

      keyring_core::Entry::new(SERVICE, "port")
        .unwrap()
        .set_password("9443")
        .unwrap();

      let settings: Settings = Compote::from(Toml::path(fixture("complete/config.toml")))
        .merge(Keyring::service(SERVICE).secret_named("server.port", "port").split("."))
        .extract()
        .unwrap();

      assert_eq!(settings.server.port, 9443, "a secret is text, like every other source");
    }

    #[test]
    fn it_reports_a_secret_it_was_told_to_expect_and_could_not_find() {
      store();

      let error = Compote::from(Toml::path(fixture("complete/config.toml")))
        .merge(Keyring::service(SERVICE).secret_named("database.url", "never-set"))
        .extract::<Settings>()
        .unwrap_err();

      assert!(error.to_string().starts_with("failed to read"), "{error}");
      assert!(error.to_string().contains("never-set"), "{error}");
    }

    #[test]
    fn it_lets_the_file_underneath_stand_when_a_secret_is_optional() {
      store();

      let settings: Settings = Compote::from(Toml::path(fixture("complete/config.toml")))
        .merge(
          Keyring::service(SERVICE)
            .secret_named("database.url", "never-set")
            .optional()
            .split("."),
        )
        .extract()
        .unwrap();

      assert_eq!(
        settings.database.url,
        complete().database.url,
        "an absent secret takes nothing from the layer beneath it"
      );
    }
  }
}
