# Contributing to Compote

Thanks for taking the time. Compote has a narrow job: read configuration from the sources you name, merge
them in the order you choose, and hand back one typed value. Changes that keep it narrow are the most
welcome.

## Getting started

Compote uses [mise] to pin every tool the project needs, including the Rust toolchains.

```sh
git clone https://github.com/aaronmallen/compote.git
cd compote
mise trust
mise install
```

That installs a stable toolchain with `clippy`, a nightly toolchain for `rustfmt`, and the linters and test
tools. You need nothing else.

## Tasks

Everything runs through `mise run`. `mise tasks` lists them all.

| Task | Does |
| --- | --- |
| `mise run build` | Builds the crate. Takes `--all-features` |
| `mise run check` | Checks for compile errors without building. Takes `--all-features` |
| `mise run test` | Runs the tests under coverage and writes a report to `coverage/` |
| `mise run format` | Formats Rust, Markdown, shell, TOML, and YAML |
| `mise run lint` | Lints all of the above, plus EditorConfig |
| `mise run audit` | Checks dependencies for security advisories and newer versions |

`mise run test --filter <name>` narrows the run to tests whose name matches.

`mise run test` uses nextest, which skips doc examples. Run `cargo test --doc --all-features` when you touch
one. CI runs both.

## Before you open a pull request

```sh
mise run format
mise run lint
mise run test
```

CI runs those three, the doc examples, a build with no features and with every feature, and a build of each
format feature on its own. Clippy runs with `-D warnings`, so one warning fails the build.

## How the code is laid out

| Path | Holds |
| --- | --- |
| `src/lib.rs` | Crate docs and the public re-exports |
| `src/compote.rs` | `Compote`, the chain of sources and the final `extract` |
| `src/error.rs` | `Error` and `Result` |
| `src/provider.rs` | The `Provider` trait and the shared file-reading helper |
| `src/provider/` | One module per source, each behind a feature of the same name |
| `src/value.rs` | `Value` and its merge rules |
| `src/value/de.rs` | Reading a `Value` into a target type, string coercion included |
| `src/value/ser.rs` | Turning a `Serialize` type into a `Value` |
| `tests/layering.rs` | Integration tests for the ordering rules |

## Conventions

**Formatting.** Two spaces, 120 columns, `rustfmt` on nightly. Run `mise run format` rather than matching it
by hand.

**Ordering.** Items within a module, and tests within a test module, go in alphabetical order.

**Tests.** Unit tests live in the module they cover, inside `#[cfg(test)] mod tests`, nested one module per
item and one per method:

```rust
mod tests {
  mod value {
    mod deserialize_bool {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_coerces_the_strings_it_knows() {
        // ...
      }
    }
  }
}
```

Names read as sentences and start with `it_`. Test private helpers directly rather than through the public
surface. The crate covers close to every line. `mise run test` prints the numbers, and a change that drops
them needs a reason.

**Docs.** Every public item carries a doc comment, and `#![warn(missing_docs)]` enforces it.

## Adding a format

1. Add `src/provider/<name>.rs` with a struct, a constructor, and `impl Provider`. File-backed formats go
   through `super::load`, which reads the file, turns I/O and parse failures into `Error`, and reads an empty
   file as an empty table.
2. Declare the module and re-export the type from `src/provider.rs`, both behind `#[cfg(feature = "<name>")]`.
3. Re-export it from `src/lib.rs` behind the same feature.
4. Add the feature to `Cargo.toml` as `<name> = ["dep:<parser>"]`, keep the parser `optional = true`, and turn
   on the parser features you rely on. Several parsers hide their `serde` support behind a feature that is off
   by default.
5. Add a row to the table in `README.md` and to the matching table in `src/lib.rs`.
6. Cover it: the scalar types the format declares, a nested table, an empty file reading as an empty table,
   and a parse failure naming the path.
7. Add a line to the changelog.

## Changelog

`CHANGELOG.md` follows [Keep a Changelog], and the crate follows [Semantic Versioning]. Put anything a user
would notice under `Unreleased`.

## Reporting a bug

Open an issue with the configuration that triggered it, the type you extracted into, what you expected, and
what you got. Better still, a failing test.

For anything with a security impact, follow the [security policy](SECURITY.md) instead.

## Code of conduct

Taking part in this project means following the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

The MIT License covers contributions, the same as the rest of the crate.

[Keep a Changelog]: https://keepachangelog.com/en/1.1.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html
[mise]: https://mise.jdx.dev
