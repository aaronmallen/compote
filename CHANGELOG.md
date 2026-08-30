# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic Versioning].

## [Unreleased]

### Changed

- A string holding nothing now fills an empty map, the way it already filled an empty list. A format
  that is only text has one spelling for a value that holds nothing, and it has to serve both. Only
  the empty string: there is no splitting a string into named values, so anything else is still
  refused.

### Added

- An `xml` feature and an `Xml` provider, reading XML through `roxmltree`. The second file format
  that is only text, and the one that gets the most out of it. The root element names the file
  rather than anything in it and is thrown away, a child element is a key, an attribute is a key
  beside it, and an element said twice is a list. A repeated element brings its own children along,
  so that list holds tables as readily as strings, which is the one thing `Ini` cannot say.
- `Xml::attribute_prefix`, which puts every attribute under a prefix of your choosing, for the
  document where an attribute and a child element share a name and have to stay apart.
- `Xml::text_key`, which names the key an element's own text lands under when it also carries
  attributes or children. `$text` unless renamed, which no XML name can be.
- `Xml::allow_doctype` and `Xml::deny_doctype`. A `<!DOCTYPE>` is refused until asked for, since a
  DTD can define entities that expand into far more than the file appears to hold. Nothing outside
  the file is ever fetched either way.

- An `ini` feature and an `Ini` provider, reading INI through `rust-ini`. The one file format that is
  only text, which is the model the environment already uses. A section is a table, and past that
  nothing nests until `Ini::split` says what to nest on, since INI has no depth of its own to borrow.
- `Ini::split`, which nests section names and keys wherever a separator appears, so `[server.tls]`
  under `split(".")` is `tls` inside `server` rather than one key with a dot in it.
- An `allow_` and a `deny_` method for each syntax `Ini` can be taught: quotes, backslash escapes,
  and values continued onto the indented lines beneath them.

## [v0.2.0] - 2026-08-28

### Changed

- `Json` now reads JSON with comments and trailing commas, so a `.json` file may carry comments and a
  `.jsonc` file need not. The extension decides nothing.
- JSON is parsed straight into a `Value` by `jsonc-parser`, rather than by way of an intermediate
  tree.
- Comments and trailing commas are the only things allowed beyond strict JSON by default.
  Single-quoted strings, hexadecimal and unary-plus numbers, unquoted property names, and missing
  commas between values were all accepted before, and now have to be asked for.

### Added

- A `cbor` feature and a `Cbor` provider, reading CBOR (RFC 8949) through `ciborium`. The second
  binary format, and the one with a standards body behind it. Like MessagePack it wants string keys,
  and it refuses byte strings and tagged values, the standard date and time tags among them.
- A `msgpack` feature and a `MsgPack` provider, reading MessagePack through `rmp-serde`. The one
  binary format, for a file something else writes: a build step, a cache, a sidecar. It wants string
  keys, which is what `rmp_serde::to_vec_named` writes and the compact `rmp_serde::to_vec` does not,
  and it refuses the `bin` and `ext` families rather than guessing at them.
- `Json::strict` and `Json::lenient`, for JSON alone and for everything the parser knows.
- An `allow_` and a `deny_` method for each syntax `Json` can be taught: comments, trailing commas,
  single-quoted strings, hexadecimal numbers, unary-plus numbers, loose object property names, and
  missing commas.

### Removed

- The `Jsonc` provider and the `jsonc` feature. Use `Json` and the `json` feature.
- The `serde_json` dependency.

## v0.1.0 - 2026-08-26

Initial release

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

[Unreleased]: https://github.com/aaronmallen/compote/compare/0.2.0...HEAD
[v0.2.0]: https://github.com/aaronmallen/compote/compare/3892304c...0.2.0
