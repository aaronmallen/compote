use std::{
  collections::{BTreeMap, btree_map::Entry},
  path::PathBuf,
};

use rust_ini::{Ini as Document, ParseOption};

use crate::{Provider, Result, Value};

/// What [`Ini`] accepts unless told otherwise.
///
/// `rust-ini` would hand these over from `Default`. Every field is named here instead, so a new one
/// upstream has to be ruled on rather than quietly let in.
const DEFAULT: Syntax = Syntax {
  escapes: true,
  indented_multiline_values: false,
  quotes: true,
};

/// Configuration read from an INI file.
///
/// Every value is text, the way an environment variable is, and the field it lands on coerces it on
/// the way out. INI says nothing about types, so nothing here pretends otherwise: `port = 8443`
/// fills a `u16` and `tls = yes` fills a `bool`.
///
/// A section is a table. Past that nothing nests until you call [`split`](Ini::split). With
/// `split(".")` the section `[server.tls]` is the table `tls` inside the table `server`, and the
/// key `pool.max` nests the same way.
///
/// ```
/// use compote::Ini;
///
/// // Sections and nothing else: `[server.tls]` is one key with a dot in it.
/// let ini = Ini::path("config.ini");
///
/// // `[server.tls]` and `pool.max` both nest.
/// let ini = Ini::path("config.ini").split(".");
/// ```
///
/// INI has no list of its own, so a key said twice is one:
///
/// ```ini
/// cipher = TLS_AES_128_GCM_SHA256
/// cipher = TLS_AES_256_GCM_SHA384
/// ```
///
/// fills a `Vec<String>`, and so does one comma-separated value. A section said twice is not a
/// list. Its keys join the ones already under that name, which is what an INI reader is expected to
/// do with it.
///
/// A `;` or a `#` opens a comment only at the start of a line. Further along a value it belongs to
/// the value, since a password or a URL is as likely to hold one as not, and on a section's own
/// line it is refused outright rather than guessed at. Quote a value to keep the whitespace at
/// either end of it, and see [`deny_escapes`](Ini::deny_escapes) for a file that means its
/// backslashes, a Windows path among them.
///
/// An empty file, or one holding only comments, reads as an empty table rather than an error, so an
/// optional file costs nothing.
///
/// ```no_run
/// use compote::{Compote, Ini};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Ini::path("config.ini")).extract().unwrap();
/// ```
pub struct Ini {
  path: PathBuf,
  separator: String,
  syntax: Syntax,
}

impl Ini {
  /// Reads `\` as an escape, so `\t` is a tab and `\x41` is an `A`. On unless refused.
  pub fn allow_escapes(mut self) -> Self {
    self.syntax.escapes = true;

    self
  }

  /// Continues a value onto the indented lines beneath it.
  ///
  /// ```ini
  /// motd =
  ///   the first line
  ///   the second
  /// ```
  pub fn allow_indented_multiline_values(mut self) -> Self {
    self.syntax.indented_multiline_values = true;

    self
  }

  /// Reads `"` and `'` as quotes rather than as text, which is how a value keeps the whitespace at
  /// either end of it. On unless refused.
  pub fn allow_quotes(mut self) -> Self {
    self.syntax.quotes = true;

    self
  }

  /// Reads `\` as a backslash.
  ///
  /// The place for a file holding Windows paths, where `C:\temp` means what it says rather than
  /// `C:temp`. It costs the escapes, so a tab has to be a real tab.
  pub fn deny_escapes(mut self) -> Self {
    self.syntax.escapes = false;

    self
  }

  /// Ends a value at the end of its line.
  pub fn deny_indented_multiline_values(mut self) -> Self {
    self.syntax.indented_multiline_values = false;

    self
  }

  /// Reads `"` and `'` as text, so a value that opens with one keeps it.
  pub fn deny_quotes(mut self) -> Self {
    self.syntax.quotes = false;

    self
  }

  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into(),
      separator: String::new(),
      syntax: DEFAULT,
    }
  }

  /// Nests section names and keys wherever `separator` appears.
  ///
  /// A section is a table with or without this. What it adds is depth: `[server.tls]` under
  /// `split(".")` is `tls` inside `server` rather than a single key spelled `server.tls`.
  ///
  /// A nested key beats a scalar already sitting at the same path.
  pub fn split(mut self, separator: &str) -> Self {
    self.separator = separator.to_owned();

    self
  }

  fn overlay(&self, document: Document) -> Value {
    let mut root = BTreeMap::new();

    for (section, properties) in document {
      let segments = section.as_deref().map(|name| self.segments(name)).unwrap_or_default();
      let table = descend(&mut root, &segments);

      for (key, value) in properties {
        insert(table, &self.segments(&key), value);
      }
    }

    Value::Table(root)
  }

  fn segments<'a>(&self, name: &'a str) -> Vec<&'a str> {
    if self.separator.is_empty() {
      return vec![name];
    }

    name
      .split(self.separator.as_str())
      .filter(|segment| !segment.is_empty())
      .collect()
  }
}

impl Provider for Ini {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, |source| {
      Document::load_from_str_opt(source, self.syntax.into()).map(|document| self.overlay(document))
    })
  }
}

/// What the parser is allowed to read.
///
/// `ParseOption` is not `Clone` and every read needs one of its own, so the answer is kept here and
/// turned into one at the last moment.
#[derive(Clone, Copy)]
struct Syntax {
  escapes: bool,
  indented_multiline_values: bool,
  quotes: bool,
}

impl From<Syntax> for ParseOption {
  fn from(syntax: Syntax) -> Self {
    Self {
      enabled_escape: syntax.escapes,
      enabled_indented_mutiline_value: syntax.indented_multiline_values,
      // Whitespace before a key is indentation, never part of the name.
      enabled_preserve_key_leading_whitespace: false,
      enabled_quote: syntax.quotes,
    }
  }
}

/// Walks to the table `segments` names, making each one along the way that is missing.
///
/// A section holding nothing still gets its table, so `[extra]` reads as an empty map rather than as
/// a key that was never there.
fn descend<'a>(table: &'a mut BTreeMap<String, Value>, segments: &[&str]) -> &'a mut BTreeMap<String, Value> {
  let mut current = table;

  for segment in segments {
    let entry = current.entry((*segment).to_owned()).or_insert_with(Value::table);

    if !matches!(entry, Value::Table(_)) {
      *entry = Value::table();
    }

    let Value::Table(nested) = entry else {
      unreachable!("the entry was just replaced with a table")
    };

    current = nested;
  }

  current
}

fn insert(table: &mut BTreeMap<String, Value>, segments: &[&str], value: String) {
  let Some((leaf, parents)) = segments.split_last() else {
    return;
  };

  match descend(table, parents).entry((*leaf).to_owned()) {
    Entry::Occupied(mut occupied) => repeat(occupied.get_mut(), value),
    Entry::Vacant(vacant) => {
      vacant.insert(Value::String(value));
    }
  }
}

/// Adds a value to a key that already has one.
///
/// The second one turns the first into a list rather than replacing it, since saying a key twice is
/// the only way INI has of saying two of something. A table already at the path is a nested key,
/// and a nested key beats a scalar, as it does for the environment.
fn repeat(slot: &mut Value, value: String) {
  match slot {
    Value::List(items) => items.push(Value::String(value)),
    Value::Table(_) => {}
    other => {
      let first = std::mem::replace(other, Value::Null);
      *other = Value::List(vec![first, Value::String(value)]);
    }
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

  fn list(values: Vec<&str>) -> Value {
    Value::List(values.into_iter().map(string).collect())
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

  mod ini {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_every_value_as_text() {
        let handle = file("host = localhost\nport = 8080\ntls = true\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![
            ("host", string("localhost")),
            ("port", string("8080")),
            ("tls", string("true")),
          ])
        );
      }

      #[test]
      fn it_reads_a_section_as_a_table() {
        let handle = file("[server]\nhost = localhost\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![("server", table(vec![("host", string("localhost"))]))])
        );
      }

      #[test]
      fn it_keeps_the_keys_above_the_first_section_at_the_root() {
        let handle = file("name = compote\n\n[server]\nhost = localhost\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![
            ("name", string("compote")),
            ("server", table(vec![("host", string("localhost"))])),
          ])
        );
      }

      #[test]
      fn it_reads_a_section_holding_nothing_as_an_empty_table() {
        let handle = file("[extra]\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![("extra", Value::table())])
        );
      }

      #[test]
      fn it_joins_a_section_that_appears_twice() {
        let handle = file("[server]\nhost = localhost\n\n[database]\nurl = sqlite\n\n[server]\nport = 8080\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![
            ("database", table(vec![("url", string("sqlite"))])),
            (
              "server",
              table(vec![("host", string("localhost")), ("port", string("8080"))])
            ),
          ])
        );
      }

      #[test]
      fn it_turns_a_key_that_appears_twice_into_a_list() {
        let handle = file("[tls]\ncipher = first\ncipher = second\ncipher = third\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![(
            "tls",
            table(vec![("cipher", list(vec!["first", "second", "third"]))])
          )])
        );
      }

      #[test]
      fn it_keeps_a_number_sign_that_does_not_start_a_line() {
        let handle = file("[app]\nurl = https://example.test/#anchor\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![(
            "app",
            table(vec![("url", string("https://example.test/#anchor"))])
          )])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file("   \n");

        assert_eq!(Ini::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reads_a_comment_only_file_as_an_empty_table() {
        let handle = file("; nothing here\n# nor here\n");

        assert_eq!(Ini::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reads_a_key_with_no_value_as_an_empty_string() {
        let handle = file("notes =\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![("notes", string(""))])
        );
      }

      #[test]
      fn it_reports_a_file_it_cannot_read() {
        let error = Ini::path("/does/not/exist.ini").data().unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file("[unterminated\n");
        let error = Ini::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }

      #[test]
      fn it_refuses_a_comment_sharing_a_line_with_a_section() {
        let handle = file("[server] ; the socket\nhost = localhost\n");
        let error = Ini::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }
    }

    mod allow_escapes {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_hands_the_escapes_back() {
        let handle = file("[app]\nlogs = C:\\temp\n");

        assert_eq!(
          Ini::path(handle.path()).deny_escapes().allow_escapes().data().unwrap(),
          table(vec![("app", table(vec![("logs", string("C:\temp"))]))])
        );
      }
    }

    mod allow_quotes {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_hands_the_quotes_back() {
        let handle = file("[app]\nname = \"  padded  \"\n");

        assert_eq!(
          Ini::path(handle.path()).deny_quotes().allow_quotes().data().unwrap(),
          table(vec![("app", table(vec![("name", string("  padded  "))]))])
        );
      }
    }

    mod deny_escapes {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_a_backslash_the_default_would_eat() {
        let handle = file("[app]\nlogs = C:\\temp\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![("app", table(vec![("logs", string("C:\temp"))]))]),
          "the default reads the t as a tab"
        );
        assert_eq!(
          Ini::path(handle.path()).deny_escapes().data().unwrap(),
          table(vec![("app", table(vec![("logs", string("C:\\temp"))]))])
        );
      }
    }

    mod deny_quotes {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_a_quote_as_part_of_the_value() {
        let handle = file("[app]\nname = \"  padded  \"\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![("app", table(vec![("name", string("  padded  "))]))])
        );
        assert_eq!(
          Ini::path(handle.path()).deny_quotes().data().unwrap(),
          table(vec![("app", table(vec![("name", string("\"  padded  \""))]))])
        );
      }
    }

    mod allow_indented_multiline_values {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_continues_a_value_onto_the_lines_beneath_it() {
        let handle = file("[app]\nmotd =\n  the first line\n  the second\n");

        assert_eq!(
          Ini::path(handle.path())
            .allow_indented_multiline_values()
            .data()
            .unwrap(),
          table(vec![(
            "app",
            table(vec![("motd", string("the first line\nthe second"))])
          )])
        );
      }
    }

    mod deny_indented_multiline_values {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_takes_the_continuation_back_away() {
        let handle = file("[app]\nmotd =\n  the first line\n  the second\n");

        assert_eq!(
          Ini::path(handle.path())
            .allow_indented_multiline_values()
            .deny_indented_multiline_values()
            .data()
            .unwrap(),
          table(vec![("app", table(vec![("motd", string(""))]))]),
          "the value ends at its own line rather than taking the two beneath it"
        );
      }
    }

    mod split {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_a_dotted_section_name_whole_until_a_separator_is_set() {
        let handle = file("[server.tls]\nenabled = true\n");

        assert_eq!(
          Ini::path(handle.path()).data().unwrap(),
          table(vec![("server.tls", table(vec![("enabled", string("true"))]))])
        );
      }

      #[test]
      fn it_nests_a_section_name_on_the_separator() {
        let handle = file("[server.tls]\nenabled = true\n");

        assert_eq!(
          Ini::path(handle.path()).split(".").data().unwrap(),
          table(vec![(
            "server",
            table(vec![("tls", table(vec![("enabled", string("true"))]))])
          )])
        );
      }

      #[test]
      fn it_nests_a_key_on_the_separator() {
        let handle = file("[database]\npool.max = 32\npool.min = 4\n");

        assert_eq!(
          Ini::path(handle.path()).split(".").data().unwrap(),
          table(vec![(
            "database",
            table(vec![("pool", table(vec![("max", string("32")), ("min", string("4"))]))])
          )])
        );
      }

      #[test]
      fn it_lets_a_nested_key_win_over_a_scalar_at_the_same_path() {
        let handle = file("[app]\nlog = off\nlog.level = debug\n");

        assert_eq!(
          Ini::path(handle.path()).split(".").data().unwrap(),
          table(vec![(
            "app",
            table(vec![("log", table(vec![("level", string("debug"))]))])
          )])
        );
      }

      #[test]
      fn it_drops_empty_segments() {
        let handle = file("[server..tls]\n..enabled.. = true\n");

        assert_eq!(
          Ini::path(handle.path()).split(".").data().unwrap(),
          table(vec![(
            "server",
            table(vec![("tls", table(vec![("enabled", string("true"))]))])
          )])
        );
      }

      #[test]
      fn it_ignores_a_section_that_is_nothing_but_separators() {
        let handle = file("[.]\nhost = localhost\n");

        assert_eq!(
          Ini::path(handle.path()).split(".").data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }

      #[test]
      fn it_ignores_a_key_that_is_nothing_but_separators() {
        let handle = file("[app]\n. = localhost\n");

        assert_eq!(
          Ini::path(handle.path()).split(".").data().unwrap(),
          table(vec![("app", Value::table())])
        );
      }
    }
  }

  mod descend {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_makes_every_table_it_walks_through() {
      let mut root = BTreeMap::new();

      descend(&mut root, &["server", "tls"]).insert("enabled".to_owned(), string("true"));

      assert_eq!(
        Value::Table(root),
        table(vec![(
          "server",
          table(vec![("tls", table(vec![("enabled", string("true"))]))])
        )])
      );
    }

    #[test]
    fn it_replaces_a_scalar_standing_where_a_table_belongs() {
      let mut root = BTreeMap::from([("server".to_owned(), string("off"))]);

      descend(&mut root, &["server"]).insert("host".to_owned(), string("localhost"));

      assert_eq!(
        Value::Table(root),
        table(vec![("server", table(vec![("host", string("localhost"))]))])
      );
    }

    #[test]
    fn it_hands_back_the_table_itself_when_there_is_nowhere_to_walk() {
      let mut root = BTreeMap::new();

      descend(&mut root, &[]).insert("host".to_owned(), string("localhost"));

      assert_eq!(Value::Table(root), table(vec![("host", string("localhost"))]));
    }
  }

  mod insert {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_ignores_a_key_with_no_segments() {
      let mut root = BTreeMap::new();

      insert(&mut root, &[], "localhost".to_owned());

      assert_eq!(Value::Table(root), Value::table());
    }
  }

  mod repeat {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_turns_the_value_already_there_into_a_list() {
      let mut slot = string("first");

      repeat(&mut slot, "second".to_owned());

      assert_eq!(slot, list(vec!["first", "second"]));
    }

    #[test]
    fn it_adds_to_a_list_already_there() {
      let mut slot = list(vec!["first", "second"]);

      repeat(&mut slot, "third".to_owned());

      assert_eq!(slot, list(vec!["first", "second", "third"]));
    }

    #[test]
    fn it_leaves_a_table_alone() {
      let mut slot = table(vec![("level", string("debug"))]);

      repeat(&mut slot, "off".to_owned());

      assert_eq!(slot, table(vec![("level", string("debug"))]));
    }
  }
}
