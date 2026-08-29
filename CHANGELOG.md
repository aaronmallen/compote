# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic Versioning].

## [Unreleased]

### Changed

- `Json` now reads JSON with comments and trailing commas, so a `.json` file may carry comments and a
  `.jsonc` file need not. The extension decides nothing.
- JSON is parsed straight into a `Value` by `jsonc-parser`, rather than by way of an intermediate
  tree.
- Comments and trailing commas are the only things allowed beyond strict JSON by default.
  Single-quoted strings, hexadecimal and unary-plus numbers, unquoted property names, and missing
  commas between values were all accepted before, and now have to be asked for.

### Added

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

[Unreleased]: https://github.com/aaronmallen/fig/tree/main/crates/compote
