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

| Feature | Reads                                            | Parser         |
|---------|--------------------------------------------------|----------------|
| `env`   | environment variables                            |                |
| `json`  | `.json`                                          | `serde_json`   |
| `jsonc` | `.jsonc`, JSON with comments and trailing commas | `jsonc-parser` |
| `toml`  | `.toml`                                          | `toml_edit`    |
| `yaml`  | `.yaml`, `.yml`                                  | `yaml_serde`   |

## Credits

Compote owes its API to [Figment] by Sergio Benitez. Figment got the shape right: name your sources in
precedence order, then `extract` once at the end. Compote reads JSON with comments and keeps its YAML on a
parser that is still maintained, but the design is Figment's.

## License

Compote is licensed under the [MIT License]

[Figment]: https://github.com/SergioBenitez/Figment
[MIT License]: LICENSE
