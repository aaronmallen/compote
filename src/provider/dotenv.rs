use std::path::PathBuf;

use crate::{Provider, Result, Value};

/// Configuration read from a `.env` file.
///
/// The same shape [`Env`](crate::Env) reads, kept in a file. Names are lowercased and the prefix, if
/// there is one, comes off, so `DATABASE_URL` is `database_url` whether this reads it or the shell
/// sourced it first and [`Env`](crate::Env) read it after. Every value is text, and the field it
/// lands on coerces it on the way out.
///
/// A `.env` file is flat, so nothing nests until you call [`split`](Dotenv::split). With
/// `split("__")` the line `SERVER__HOST=localhost` is the key `host` inside the table `server`.
///
/// A name is a shell variable name: a letter or an underscore, then letters, digits, underscores,
/// and dots. A hyphen is not one of them, so a key like `beta-ui` has no spelling here and a line
/// that tries is refused rather than truncated. That is the format's limit rather than this
/// reader's, since the same file has to survive being sourced.
///
/// ```
/// use compote::Dotenv;
///
/// // Flat: `SERVER__HOST` is one key.
/// let dotenv = Dotenv::path(".env");
///
/// // `APP_SERVER__HOST=localhost` is `host` inside `server`.
/// let dotenv = Dotenv::path(".env").prefixed("APP_").split("__");
/// ```
///
/// A name said twice keeps its last value rather than becoming a list, which is the one thing an
/// environment variable can be. This is where a `.env` file parts company with [`Ini`](crate::Ini)
/// and [`Xml`](crate::Xml), whose repeated keys are the only list those formats have.
///
/// `export FOO=bar` reads as `FOO=bar`, so a file meant to be sourced needs no editing. A `#` opens
/// a comment. A single-quoted value is literal, and a double-quoted one takes the escapes `\\`,
/// `\'`, `\"`, `\$`, `\ `, and `\n`. There is no `\t`, and a file that writes one is refused rather
/// than read as a `t`, so a tab has to be a real tab inside single quotes.
///
/// `$NAME` and `${NAME}` are filled in, which is the one place this reads something the file does
/// not hold: the process environment answers first, an earlier line of the same file answers what
/// the environment does not, and a name neither knows is the empty string rather than an error. A
/// value that means a literal `$` wants single quotes around it.
///
/// Prefer the braces. A bare `$NAME` ends at the first character that is not a letter or a digit,
/// so `$DATABASE__HOST` is the name `DATABASE` followed by the text `__HOST`, while
/// `${DATABASE__HOST}` is the one name it looks like.
///
/// An empty file, or one holding only comments, reads as an empty table rather than an error, so an
/// optional file costs nothing.
///
/// ```no_run
/// use compote::{Compote, Dotenv};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Dotenv::path(".env")).extract().unwrap();
/// ```
pub struct Dotenv {
  ignore: Vec<String>,
  path: PathBuf,
  prefix: String,
  separator: String,
}

impl Dotenv {
  /// Drops the named keys.
  ///
  /// Names are matched after the prefix comes off, and case is ignored, so `ignore(&["CONFIG"])`
  /// drops `CONFIG`. Use it to keep a line that points at your configuration from becoming part of
  /// it.
  pub fn ignore(mut self, keys: &[&str]) -> Self {
    self.ignore = keys.iter().map(|key| key.to_ascii_lowercase()).collect();

    self
  }

  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      ignore: Vec::new(),
      path: path.into(),
      prefix: String::new(),
      separator: String::new(),
    }
  }

  /// Reads only the names starting with `prefix`, and takes the prefix off what it keeps.
  ///
  /// Unlike [`Env`](crate::Env), which needs one to pick your variables out of the whole machine's,
  /// a `.env` file is already yours and needs no prefix. This is for the file that carries one
  /// anyway, because the same names are meant to be exported.
  pub fn prefixed(mut self, prefix: &str) -> Self {
    self.prefix = prefix.to_owned();

    self
  }

  /// Nests keys wherever `separator` appears.
  ///
  /// A nested key beats a scalar already sitting at the same path.
  pub fn split(mut self, separator: &str) -> Self {
    self.separator = separator.to_owned();

    self
  }
}

impl Provider for Dotenv {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, |source| {
      dotenvy::Iter::new(source.as_bytes())
        .collect::<std::result::Result<Vec<(String, String)>, _>>()
        .map(|pairs| super::overlay(pairs, &self.prefix, &self.ignore, &self.separator))
    })
  }
}

#[cfg(test)]
mod tests {
  use std::io::Write;

  use tempfile::NamedTempFile;

  use super::*;

  fn file(source: &str) -> NamedTempFile {
    let mut handle = NamedTempFile::new().unwrap();
    handle.write_all(source.as_bytes()).unwrap();

    handle
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

  mod dotenv {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_every_value_as_text() {
        let handle = file("HOST=localhost\nPORT=8080\nTLS=true\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![
            ("host", string("localhost")),
            ("port", string("8080")),
            ("tls", string("true")),
          ])
        );
      }

      #[test]
      fn it_lowercases_the_names() {
        let handle = file("DATABASE_URL=sqlite\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("database_url", string("sqlite"))])
        );
      }

      #[test]
      fn it_reads_an_export_prefix_as_if_it_were_not_there() {
        let handle = file("export HOST=localhost\nPORT=8080\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("host", string("localhost")), ("port", string("8080"))])
        );
      }

      #[test]
      fn it_keeps_the_last_value_a_name_is_given() {
        let handle = file("HOST=first\nHOST=last\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("host", string("last"))]),
          "one name holds one value, unlike ini and xml where saying it twice is a list"
        );
      }

      #[test]
      fn it_reads_a_name_with_no_value_as_an_empty_string() {
        let handle = file("NOTES=\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("notes", string(""))])
        );
      }

      #[test]
      fn it_keeps_a_quoted_value_whole() {
        let handle = file("GREETING=\"first\\nsecond\"\nLITERAL='  padded  '\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![
            ("greeting", string("first\nsecond")),
            ("literal", string("  padded  ")),
          ])
        );
      }

      #[test]
      fn it_keeps_a_tab_that_is_written_as_one() {
        let handle = file("GREETING='hello\tworld'\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("greeting", string("hello\tworld"))])
        );
      }

      #[test]
      fn it_refuses_a_name_holding_a_hyphen() {
        let handle = file("BETA-UI=true\n");
        let error = Dotenv::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }

      #[test]
      fn it_refuses_an_escape_the_parser_does_not_know() {
        let handle = file("GREETING=\"hello\\tworld\"\n");
        let error = Dotenv::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }

      #[test]
      fn it_drops_a_comment() {
        let handle = file("# the socket\nHOST=localhost # where it listens\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }

      #[test]
      fn it_fills_in_a_name_an_earlier_line_set() {
        let handle = file("ROOT=/srv\nLOGS=${ROOT}/logs\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("logs", string("/srv/logs")), ("root", string("/srv"))])
        );
      }

      #[test]
      fn it_fills_in_a_name_the_process_environment_holds() {
        // SAFETY: every test runs in its own process, so nothing else is reading the environment.
        unsafe {
          std::env::set_var("COMPOTE_DOTENV_ROOT", "/from-the-environment");
        }

        let handle = file("LOGS=${COMPOTE_DOTENV_ROOT}/logs\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("logs", string("/from-the-environment/logs"))])
        );
      }

      #[test]
      fn it_reads_a_name_nothing_knows_as_the_empty_string() {
        let handle = file("LOGS=${COMPOTE_DOTENV_NOTHING_KNOWS_THIS}/logs\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("logs", string("/logs"))])
        );
      }

      #[test]
      fn it_leaves_a_dollar_inside_single_quotes_alone() {
        let handle = file("PASSWORD='s3cret$ROOT'\n");

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("password", string("s3cret$ROOT"))])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file("   \n");

        assert_eq!(Dotenv::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reads_a_comment_only_file_as_an_empty_table() {
        let handle = file("# nothing here\n# nor here\n");

        assert_eq!(Dotenv::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_a_file_it_cannot_read() {
        let error = Dotenv::path("/does/not/exist.env").data().unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file("this is not a name=value line\n");
        let error = Dotenv::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }
    }

    mod ignore {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_drops_the_named_keys_whatever_their_case() {
        let handle = file("CONFIG=/etc/compote.toml\nHOST=localhost\n");

        assert_eq!(
          Dotenv::path(handle.path()).ignore(&["CONFIG"]).data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }
    }

    mod prefixed {
      use pretty_assertions::assert_eq;

      use super::*;

      const DOCUMENT: &str = "APP_HOST=localhost\nPATH=/usr/bin\n";

      #[test]
      fn it_reads_every_name_until_a_prefix_is_set() {
        let handle = file(DOCUMENT);

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![("app_host", string("localhost")), ("path", string("/usr/bin"))])
        );
      }

      #[test]
      fn it_keeps_only_the_names_carrying_the_prefix_and_takes_it_off() {
        let handle = file(DOCUMENT);

        assert_eq!(
          Dotenv::path(handle.path()).prefixed("APP_").data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }
    }

    mod split {
      use pretty_assertions::assert_eq;

      use super::*;

      const DOCUMENT: &str = "SERVER__HOST=localhost\nSERVER__TLS__ENABLED=true\n";

      #[test]
      fn it_keeps_a_name_whole_until_a_separator_is_set() {
        let handle = file(DOCUMENT);

        assert_eq!(
          Dotenv::path(handle.path()).data().unwrap(),
          table(vec![
            ("server__host", string("localhost")),
            ("server__tls__enabled", string("true")),
          ])
        );
      }

      #[test]
      fn it_nests_on_the_separator() {
        let handle = file(DOCUMENT);

        assert_eq!(
          Dotenv::path(handle.path()).split("__").data().unwrap(),
          table(vec![(
            "server",
            table(vec![
              ("host", string("localhost")),
              ("tls", table(vec![("enabled", string("true"))])),
            ])
          )])
        );
      }

      #[test]
      fn it_nests_on_a_dot_the_parser_allows_in_a_name() {
        let handle = file("DATABASE.POOL.MAX=32\n");

        assert_eq!(
          Dotenv::path(handle.path()).split(".").data().unwrap(),
          table(vec![(
            "database",
            table(vec![("pool", table(vec![("max", string("32"))]))])
          )])
        );
      }
    }
  }
}
