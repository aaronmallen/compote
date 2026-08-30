use std::{
  collections::{BTreeMap, btree_map::Entry},
  path::PathBuf,
};

use roxmltree::{Document, Node, ParsingOptions};

use crate::{Provider, Result, Value};

/// Where an element's own text lands when it also carries attributes or children.
///
/// `$` cannot begin an XML name, so this key can never be an element's or an attribute's.
const TEXT_KEY: &str = "$text";

/// Configuration read from an XML file.
///
/// Every value is text, the way an environment variable is, and the field it lands on coerces it on
/// the way out. XML says nothing about types, so nothing here pretends otherwise:
/// `<port>8443</port>` fills a `u16` and `tls="yes"` fills a `bool`.
///
/// The root element is the document. Its name is thrown away, since a name that every path begins
/// with says nothing, and what it holds is the table you extract from.
///
/// A child element is a key, and an attribute is a key beside it, so
/// `<server host="0.0.0.0" port="8443"/>` fills a struct with those two fields. See
/// [`attribute_prefix`](Xml::attribute_prefix) for a document where the two have to be told apart.
///
/// Repetition is the list XML does not otherwise have:
///
/// ```xml
/// <tags>api</tags>
/// <tags>web</tags>
/// ```
///
/// fills a `Vec<String>`, and so does one comma-separated value. A repeated element brings its own
/// children along, so a list of tables is as sayable as a list of strings and `Vec<Owner>` is
/// spellable, which is what INI could not manage.
///
/// An element carrying neither attributes nor children is its text, and one carrying nothing at all
/// is the empty string, which fills an empty list, an empty map, or a `None`. An element carrying
/// attributes or children is a table, and its own text, if it has any, lands under
/// [`text_key`](Xml::text_key).
///
/// Text is trimmed, and text that is only whitespace is the indentation between two elements rather
/// than a value. A comment, a processing instruction, and the XML declaration are all skipped. A
/// namespace prefix is dropped and the local name kept, since a prefix is the document's own
/// shorthand for a namespace rather than part of the name.
///
/// A `<!DOCTYPE>` is refused until [`allow_doctype`](Xml::allow_doctype) says otherwise, since a DTD
/// can define entities that expand into far more than the file appears to hold.
///
/// An empty file reads as an empty table rather than an error, so an optional file costs nothing. A
/// file holding anything other than one root element is not XML, and is reported as a parse
/// failure.
///
/// ```no_run
/// use compote::{Compote, Xml};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Settings {
///   host: String,
///   port: u16,
/// }
///
/// let settings: Settings = Compote::from(Xml::path("config.xml")).extract().unwrap();
/// ```
pub struct Xml {
  attribute_prefix: String,
  doctype: bool,
  path: PathBuf,
  text_key: String,
}

impl Xml {
  /// Reads a `<!DOCTYPE>` rather than refusing the document that carries one.
  ///
  /// A DTD is how XML declares its own entities, and an entity is how a small file expands into a
  /// large one. Nothing outside the file is ever fetched, and the parser's own limits still stand,
  /// but the expansion a DTD asks for is worth asking for rather than assuming.
  pub fn allow_doctype(mut self) -> Self {
    self.doctype = true;

    self
  }

  /// Puts every attribute under `prefix` followed by its name.
  ///
  /// Without one an attribute is an ordinary key, which is what makes
  /// `<server host="0.0.0.0" port="8443"/>` fill a struct. A prefix is for the document where an
  /// attribute and a child element share a name and have to stay apart. `@` is the usual spelling,
  /// and no XML name may hold one, so a prefixed key can never be an element's.
  ///
  /// ```
  /// use compote::Xml;
  ///
  /// // `<server port="8443"/>` is `server.port`.
  /// let xml = Xml::path("config.xml");
  ///
  /// // The same document is `server.@port`.
  /// let xml = Xml::path("config.xml").attribute_prefix("@");
  /// ```
  pub fn attribute_prefix(mut self, prefix: &str) -> Self {
    self.attribute_prefix = prefix.to_owned();

    self
  }

  /// Refuses a document carrying a `<!DOCTYPE>`. On unless allowed.
  pub fn deny_doctype(mut self) -> Self {
    self.doctype = false;

    self
  }

  /// Reads the file at `path`.
  ///
  /// Nothing is read until the source is merged, and it is read again each time it is.
  pub fn path(path: impl Into<PathBuf>) -> Self {
    Self {
      attribute_prefix: String::new(),
      doctype: false,
      path: path.into(),
      text_key: TEXT_KEY.to_owned(),
    }
  }

  /// Names the key an element's own text lands under when it also carries attributes or children.
  ///
  /// `$text` unless renamed, which no XML name can be, so it can never take a key the document
  /// meant to have. An element carrying only text is that text and never reaches this.
  ///
  /// ```
  /// use compote::Xml;
  ///
  /// // `<name lang="en">compote</name>` is `name.$text` and `name.lang`.
  /// let xml = Xml::path("config.xml");
  ///
  /// // The same document is `name.#text` and `name.lang`.
  /// let xml = Xml::path("config.xml").text_key("#text");
  /// ```
  pub fn text_key(mut self, key: &str) -> Self {
    self.text_key = key.to_owned();

    self
  }

  fn element(&self, node: Node) -> Value {
    let mut table = BTreeMap::new();

    for attribute in node.attributes() {
      let key = format!("{}{}", self.attribute_prefix, attribute.name());

      insert(&mut table, key, Value::String(attribute.value().to_owned()));
    }

    for child in node.children().filter(Node::is_element) {
      let key = child.tag_name().name().to_owned();

      insert(&mut table, key, self.element(child));
    }

    let text = text(node);

    if table.is_empty() {
      return Value::String(text.unwrap_or_default());
    }

    if let Some(text) = text {
      insert(&mut table, self.text_key.clone(), Value::String(text));
    }

    Value::Table(table)
  }

  /// Names every field `roxmltree` would hand over from `Default`, so a new one upstream has to be
  /// ruled on here rather than quietly let in.
  fn options<'a>(&self) -> ParsingOptions<'a> {
    ParsingOptions {
      allow_dtd: self.doctype,
      // An external entity is a file read, or a request, made on a config file's say-so. Whatever
      // a document declares, nothing beyond it is fetched.
      entity_resolver: None,
      nodes_limit: u32::MAX,
    }
  }

  /// Reads the root element as the whole document.
  ///
  /// The root is the one element every other one sits inside, so its name names the file rather
  /// than anything in it and is dropped. A root holding nothing is an empty document rather than
  /// the empty string an inner element would be.
  fn overlay(&self, document: &Document) -> Value {
    match self.element(document.root_element()) {
      Value::String(text) if text.is_empty() => Value::table(),
      other => other,
    }
  }
}

impl Provider for Xml {
  fn data(&self) -> Result<Value> {
    super::load(&self.path, |source| {
      Document::parse_with_options(source, self.options()).map(|document| self.overlay(&document))
    })
  }
}

/// Puts `value` at `key`, turning what is already there into a list rather than replacing it.
///
/// Saying a name twice is the only way XML has of saying two of something, and it is the same
/// whether the two are attributes, elements, or one of each.
fn insert(table: &mut BTreeMap<String, Value>, key: String, value: Value) {
  match table.entry(key) {
    Entry::Occupied(mut occupied) => match occupied.get_mut() {
      Value::List(items) => items.push(value),
      slot => {
        let first = std::mem::replace(slot, Value::Null);

        *slot = Value::List(vec![first, value]);
      }
    },
    Entry::Vacant(vacant) => {
      vacant.insert(value);
    }
  }
}

/// Joins an element's own text, and reports whitespace alone as no text at all.
///
/// The pieces are joined without anything between them, since a character reference or a CDATA
/// section splits one value across several of them and `a&#38;b` is `a&b` rather than `a & b`.
fn text(node: Node) -> Option<String> {
  let mut joined = String::new();

  for child in node.children().filter(Node::is_text) {
    joined.push_str(child.text().unwrap_or_default());
  }

  let trimmed = joined.trim();

  (!trimmed.is_empty()).then(|| trimmed.to_owned())
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

  fn list(values: Vec<Value>) -> Value {
    Value::List(values)
  }

  fn string(value: &str) -> Value {
    Value::String(value.to_owned())
  }

  fn strings(values: Vec<&str>) -> Value {
    Value::List(values.into_iter().map(string).collect())
  }

  fn table(entries: Vec<(&str, Value)>) -> Value {
    Value::Table(
      entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect(),
    )
  }

  mod xml {
    use super::*;

    mod data {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_every_value_as_text() {
        let handle = file("<config><host>localhost</host><port>8080</port><tls>true</tls></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![
            ("host", string("localhost")),
            ("port", string("8080")),
            ("tls", string("true")),
          ])
        );
      }

      #[test]
      fn it_throws_the_root_element_name_away() {
        let named = file("<config><host>localhost</host></config>");
        let other = file("<settings><host>localhost</host></settings>");

        assert_eq!(
          Xml::path(named.path()).data().unwrap(),
          Xml::path(other.path()).data().unwrap()
        );
      }

      #[test]
      fn it_reads_a_child_element_as_a_key() {
        let handle = file("<config><server><host>localhost</host></server></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("server", table(vec![("host", string("localhost"))]))])
        );
      }

      #[test]
      fn it_reads_an_attribute_as_a_key_beside_the_children() {
        let handle = file(r#"<config><server host="0.0.0.0" port="8443"><backlog>512</backlog></server></config>"#);

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![(
            "server",
            table(vec![
              ("backlog", string("512")),
              ("host", string("0.0.0.0")),
              ("port", string("8443")),
            ])
          )])
        );
      }

      #[test]
      fn it_turns_an_element_that_appears_twice_into_a_list() {
        let handle = file("<config><tags>api</tags><tags>web</tags><tags>cli</tags></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("tags", strings(vec!["api", "web", "cli"]))])
        );
      }

      #[test]
      fn it_reads_a_repeated_element_with_children_as_a_list_of_tables() {
        let handle = file("<config><owners><name>Aaron</name></owners><owners><name>Ops</name></owners></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![(
            "owners",
            list(vec![
              table(vec![("name", string("Aaron"))]),
              table(vec![("name", string("Ops"))]),
            ])
          )])
        );
      }

      #[test]
      fn it_counts_an_attribute_and_an_element_sharing_a_name_as_a_repeat() {
        let handle = file(r#"<config><server host="0.0.0.0"><host>localhost</host></server></config>"#);

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![(
            "server",
            table(vec![("host", strings(vec!["0.0.0.0", "localhost"]))])
          )])
        );
      }

      #[test]
      fn it_reads_an_element_holding_nothing_as_an_empty_string() {
        let handle = file("<config><notes/><extra></extra></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("extra", string("")), ("notes", string(""))])
        );
      }

      #[test]
      fn it_reads_a_root_holding_nothing_as_an_empty_table() {
        let handle = file("<config/>");

        assert_eq!(Xml::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_trims_an_element_of_the_whitespace_around_it() {
        let handle = file("<config>\n  <host>\n    localhost\n  </host>\n</config>\n");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }

      #[test]
      fn it_joins_the_pieces_a_character_reference_splits_a_value_into() {
        let handle = file("<config><url>a&#38;b</url><greeting>hello&#9;world</greeting></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("greeting", string("hello\tworld")), ("url", string("a&b"))])
        );
      }

      #[test]
      fn it_reads_a_cdata_section_as_text() {
        let handle = file("<config><url><![CDATA[postgres://localhost/compote]]></url></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("url", string("postgres://localhost/compote"))])
        );
      }

      #[test]
      fn it_skips_a_comment_and_a_processing_instruction() {
        let handle =
          file("<?xml version=\"1.0\"?><config><!-- the socket --><?ignore me?><host>localhost</host></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }

      #[test]
      fn it_keeps_the_local_name_and_drops_the_namespace_prefix() {
        let handle = file(r#"<c:config xmlns:c="urn:compote"><c:host>localhost</c:host></c:config>"#);

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("host", string("localhost"))])
        );
      }

      #[test]
      fn it_reads_an_empty_file_as_an_empty_table() {
        let handle = file("   \n");

        assert_eq!(Xml::path(handle.path()).data().unwrap(), Value::table());
      }

      #[test]
      fn it_reports_a_file_it_cannot_read() {
        let error = Xml::path("/does/not/exist.xml").data().unwrap_err();

        assert!(error.to_string().starts_with("failed to read"), "{error}");
      }

      #[test]
      fn it_reports_the_path_when_parsing_fails() {
        let handle = file("<config><host>localhost</config>");
        let error = Xml::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
        assert!(error.to_string().contains(handle.path().to_str().unwrap()), "{error}");
      }

      #[test]
      fn it_refuses_a_file_holding_no_root_element() {
        let handle = file("<!-- nothing here -->\n");
        let error = Xml::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }
    }

    mod allow_doctype {
      use pretty_assertions::assert_eq;

      use super::*;

      const DOCUMENT: &str = "<!DOCTYPE config [<!ENTITY who \"compote\">]><config><name>&who;</name></config>";

      #[test]
      fn it_reads_the_entities_a_dtd_declares() {
        let handle = file(DOCUMENT);

        assert_eq!(
          Xml::path(handle.path()).allow_doctype().data().unwrap(),
          table(vec![("name", string("compote"))])
        );
      }

      #[test]
      fn it_refuses_a_doctype_until_asked() {
        let handle = file(DOCUMENT);
        let error = Xml::path(handle.path()).data().unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }
    }

    mod attribute_prefix {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_an_attribute_apart_from_an_element_of_the_same_name() {
        let handle = file(r#"<config><server host="0.0.0.0"><host>localhost</host></server></config>"#);

        assert_eq!(
          Xml::path(handle.path()).attribute_prefix("@").data().unwrap(),
          table(vec![(
            "server",
            table(vec![("@host", string("0.0.0.0")), ("host", string("localhost"))])
          )])
        );
      }
    }

    mod deny_doctype {
      use super::*;

      #[test]
      fn it_takes_the_doctype_back_away() {
        let handle = file("<!DOCTYPE config><config><host>localhost</host></config>");
        let error = Xml::path(handle.path())
          .allow_doctype()
          .deny_doctype()
          .data()
          .unwrap_err();

        assert!(error.to_string().starts_with("failed to parse"), "{error}");
      }
    }

    mod text_key {
      use pretty_assertions::assert_eq;

      use super::*;

      const DOCUMENT: &str = r#"<config><name lang="en">compote</name></config>"#;

      #[test]
      fn it_puts_the_text_of_an_element_that_also_has_attributes_under_the_default_key() {
        let handle = file(DOCUMENT);

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![(
            "name",
            table(vec![("$text", string("compote")), ("lang", string("en"))])
          )])
        );
      }

      #[test]
      fn it_names_the_key_itself() {
        let handle = file(DOCUMENT);

        assert_eq!(
          Xml::path(handle.path()).text_key("#text").data().unwrap(),
          table(vec![(
            "name",
            table(vec![("#text", string("compote")), ("lang", string("en"))])
          )])
        );
      }

      #[test]
      fn it_leaves_an_element_that_is_only_text_alone() {
        let handle = file("<config><name>compote</name></config>");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("name", string("compote"))])
        );
      }

      #[test]
      fn it_drops_the_whitespace_between_two_elements_rather_than_keying_it() {
        let handle = file("<config>\n  <server>\n    <host>localhost</host>\n  </server>\n</config>\n");

        assert_eq!(
          Xml::path(handle.path()).data().unwrap(),
          table(vec![("server", table(vec![("host", string("localhost"))]))])
        );
      }
    }
  }

  mod insert {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_turns_the_value_already_there_into_a_list() {
      let mut root = BTreeMap::new();

      insert(&mut root, "tags".to_owned(), string("api"));
      insert(&mut root, "tags".to_owned(), string("web"));

      assert_eq!(Value::Table(root), table(vec![("tags", strings(vec!["api", "web"]))]));
    }

    #[test]
    fn it_adds_to_a_list_already_there() {
      let mut root = BTreeMap::new();

      for tag in ["api", "web", "cli"] {
        insert(&mut root, "tags".to_owned(), string(tag));
      }

      assert_eq!(
        Value::Table(root),
        table(vec![("tags", strings(vec!["api", "web", "cli"]))])
      );
    }

    #[test]
    fn it_keeps_a_table_and_a_scalar_side_by_side_in_one_list() {
      let mut root = BTreeMap::new();

      insert(&mut root, "targets".to_owned(), string("stdout"));
      insert(
        &mut root,
        "targets".to_owned(),
        table(vec![("path", string("/tmp/log"))]),
      );

      assert_eq!(
        Value::Table(root),
        table(vec![(
          "targets",
          list(vec![string("stdout"), table(vec![("path", string("/tmp/log"))])])
        )])
      );
    }
  }
}
