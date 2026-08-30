use std::{error::Error as StdError, fmt::Display, io, path::PathBuf};

/// Anything that can go wrong while reading configuration.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// A value did not fit the type it was read into.
  #[error("{0}")]
  Deserialize(String),
  /// A file was read but its contents could not be parsed.
  #[error("failed to parse {}: {source}", path.display())]
  Parse {
    /// The file that could not be parsed.
    path: PathBuf,
    /// What the parser for that format reported.
    source: Box<dyn StdError + Send + Sync>,
  },
  /// A file could not be read.
  #[error("failed to read {}: {source}", path.display())]
  Read {
    /// The file that could not be read.
    path: PathBuf,
    /// What the operating system reported.
    source: io::Error,
  },
  /// A secret could not be read from a keyring.
  #[error("failed to read {user} from the {service} keyring: {source}")]
  Secret {
    /// The keyring the secret was looked for in.
    service: String,
    /// What the credential store reported.
    source: Box<dyn StdError + Send + Sync>,
    /// The name the secret was looked for under.
    user: String,
  },
  /// A value could not be turned into configuration.
  #[error("{0}")]
  Serialize(String),
}

impl serde::de::Error for Error {
  fn custom<T: Display>(message: T) -> Self {
    Self::Deserialize(message.to_string())
  }
}

impl serde::ser::Error for Error {
  fn custom<T: Display>(message: T) -> Self {
    Self::Serialize(message.to_string())
  }
}

/// The result of reading configuration.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
  use super::*;

  mod error {
    use super::*;

    mod custom {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reports_a_deserialize_failure() {
        let error = <Error as serde::de::Error>::custom("boom");

        assert!(matches!(error, Error::Deserialize(_)), "{error:?}");
        assert_eq!(error.to_string(), "boom");
      }

      #[test]
      fn it_reports_a_serialize_failure() {
        let error = <Error as serde::ser::Error>::custom("boom");

        assert!(matches!(error, Error::Serialize(_)), "{error:?}");
        assert_eq!(error.to_string(), "boom");
      }
    }
  }
}
