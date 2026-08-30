# Compote

Compote reads your configuration from files and the environment, merges it in the order you choose, and
returns one typed value.

You give it sources. It does not go looking for them. Where your config files live, and which one wins,
stays your decision.

## Example

```rust
use compote::{Compote, Env, Serialized, Toml};

let settings: Settings = Compote::from(Serialized::defaults(Settings::default()))
  .merge(Toml::path(parent_path))
  .merge(Toml::path(child_path))
  .merge(Env::prefixed("MY_CRATE_").ignore(&["CONFIG"]).split("__"))
  .extract()?;
```

Every `merge` beats the one before it. Here the child file beats its parent, and the environment beats both.

## Formats

Each format sits behind a feature of the same name. None are on by default, so you pay for what you read.

| Feature   | Reads                 | Parser                       |
|-----------|-----------------------|------------------------------|
| `cbor`    | `.cbor`               | `ciborium`                   |
| `dotenv`  | `.env`                | `dotenvy`                    |
| `env`     | environment variables |                              |
| `ini`     | `.ini`                | `rust-ini`                   |
| `json`    | `.json`, `.jsonc`     | `jsonc-parser`               |
| `msgpack` | `.msgpack`, `.mpk`    | `rmp-serde`                  |
| `toml`    | `.toml`               | `toml_edit`                  |
| `xml`     | `.xml`                | `roxmltree`                  |
| `yaml`    | `.yaml`, `.yml`       | `yaml_serde`                 |

`Json` reads both spellings, since JSON with comments is a superset of JSON. Comments and trailing
commas are allowed and nothing else is, so the extension decides nothing and a file that uses neither
is still held to strict JSON. What it accepts is yours to set:

```rust
Json::path("config.json")                                             // JSON, comments, trailing commas
Json::path("config.json").allow_hexadecimal_numbers().deny_comments() // the same, adjusted
Json::path("config.json").strict()                                    // JSON and nothing else
Json::path("config.json").lenient()                                   // everything the parser knows
```

`Dotenv` is `Env` kept in a file. The same shell names, lowercased and nested the same way, so one
`.env` file lands the same whether Compote reads it or the shell sourced it first. A name holds one
value and saying it twice replaces it, which is all an environment variable can be, and a name is a
shell variable name, so a key with a hyphen in it has no spelling.

```rust
Dotenv::path(".env")                                  // `SERVER__HOST` is one key
Dotenv::path(".env").prefixed("APP_").split("__")     // `APP_SERVER__HOST` nests under `server`
```

`Ini` and `Xml` are the file formats that are only text, which is the model the environment already
uses and the one coercion was built for.

For `Ini` a section is a table, and past that nothing nests until you say so, since INI has no depth
of its own to borrow. A key said twice is the list it has no other way to spell.

```rust
Ini::path("config.ini")            // `[server.tls]` is one key with a dot in it
Ini::path("config.ini").split(".") // `[server.tls]` and `pool.max` both nest
```

`Xml` has depth already. The root element names the file rather than anything in it and is thrown
away. A child element is a key, an attribute is a key beside it, and an element said twice is a
list. Because a repeated element brings its own children along, that list holds tables as readily as
strings, which is the one thing `Ini` cannot say.

```rust
Xml::path("config.xml")                       // `<server port="8443"/>` is `server.port`
Xml::path("config.xml").attribute_prefix("@") // the same document is `server.@port`
Xml::path("config.xml").allow_doctype()       // read a `<!DOCTYPE>` and the entities it declares
```

`Cbor` and `MsgPack` are the binary formats, for a file something else writes rather than a person.
Both want string keys, and both refuse raw bytes and tagged or extension values rather than guessing
at them. `MsgPack` wants its keys from `rmp_serde::to_vec_named` rather than the compact
`rmp_serde::to_vec`, which turns a struct into an array of values in declaration order and leaves no
field names to merge on.

## Roadmap

Nine sources read today, and the shape of the crate is settled. What follows is about breadth, not
about changing how any of it works.

**Maybe: Java properties.** The same shape `.env` has, for a narrower audience, and the decision is
already made: flat until a separator says otherwise. What it would add over `Dotenv` is a different
escape and comment dialect rather than a different model, which is the argument both for doing it
cheaply and for not bothering.

**Maybe: KDL.** The appeal is real and so is the problem. A KDL node carries a name, positional
arguments, named properties, and children, all at once:

```kdl
server host="0.0.0.0" port=8443 {
  tls enabled=#true
}
```

Nothing there says how it should become a table of named values. Do arguments collect under a
reserved key? Do repeated sibling nodes become a list? How do a node's arguments and its children
share one place? The KDL project publishes JSON-in-KDL as a convention rather than a spec, which is
the same admission. The crates say it too: `kdl` parses documents without a serde layer, and `knus`
deserializes into types you declare through its own derive, not into an untyped value. KDL is a
design decision first and a parser second, and it waits on that decision being worth making.

## Credits

Compote owes its API to [Figment] by Sergio Benitez. Figment got the shape right: name your sources in
precedence order, then `extract` once at the end. Compote reads JSON with comments and keeps its YAML on a
parser that is still maintained, but the design is Figment's.

## License

Compote is licensed under the [MIT License]

[Figment]: https://github.com/SergioBenitez/Figment
[MIT License]: LICENSE
