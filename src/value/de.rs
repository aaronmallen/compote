use std::{collections::BTreeMap, fmt};

use serde::{
  Deserialize, Deserializer,
  de::{
    DeserializeSeed, EnumAccess, Error as _, IntoDeserializer, MapAccess, SeqAccess, Unexpected, VariantAccess,
    Visitor,
    value::{MapDeserializer, SeqDeserializer},
  },
  forward_to_deserialize_any,
};

use crate::{Error, Result, Value};

impl Value {
  fn boolean(&self) -> Option<bool> {
    match self {
      Self::Bool(value) => Some(*value),
      Self::String(value) => match value.trim().to_ascii_lowercase().as_str() {
        "1" | "on" | "true" | "yes" => Some(true),
        "0" | "off" | "false" | "no" => Some(false),
        _ => None,
      },
      _ => None,
    }
  }

  fn float(&self) -> Option<f64> {
    match self {
      Self::Float(value) => Some(*value),
      Self::Integer(value) => Some(*value as f64),
      Self::String(value) => value.trim().parse().ok(),
      _ => None,
    }
  }

  fn int<T>(&self) -> Result<T>
  where
    T: TryFrom<i128>,
  {
    let value = self
      .integer()
      .ok_or_else(|| Error::invalid_type(self.unexpected(), &"an integer"))?;

    T::try_from(value).map_err(|_| Error::custom(format!("{value} is out of range")))
  }

  fn integer(&self) -> Option<i128> {
    match self {
      Self::Integer(value) => Some(*value),
      Self::String(value) => value.trim().parse().ok(),
      _ => None,
    }
  }

  fn unexpected(&self) -> Unexpected<'_> {
    match self {
      Self::Bool(value) => Unexpected::Bool(*value),
      Self::Float(value) => Unexpected::Float(*value),
      Self::Integer(value) => i64::try_from(*value).map_or(Unexpected::Other("integer"), Unexpected::Signed),
      Self::List(_) => Unexpected::Seq,
      Self::Null => Unexpected::Unit,
      Self::String(value) => Unexpected::Str(value),
      Self::Table(_) => Unexpected::Map,
    }
  }
}

impl<'de> Deserialize<'de> for Value {
  fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    deserializer.deserialize_any(ValueVisitor)
  }
}

impl<'de> Deserializer<'de> for Value {
  type Error = Error;

  forward_to_deserialize_any! {
    bytes byte_buf char identifier ignored_any str string tuple tuple_struct unit unit_struct
  }

  fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self {
      Self::Bool(value) => visitor.visit_bool(value),
      Self::Float(value) => visitor.visit_f64(value),
      Self::Integer(value) => match i64::try_from(value) {
        Ok(value) => visitor.visit_i64(value),
        Err(_) => match u64::try_from(value) {
          Ok(value) => visitor.visit_u64(value),
          Err(_) => visitor.visit_i128(value),
        },
      },
      Self::List(values) => visitor.visit_seq(SeqDeserializer::new(values.into_iter())),
      Self::Null => visitor.visit_unit(),
      Self::String(value) => visitor.visit_string(value),
      Self::Table(entries) => visitor.visit_map(MapDeserializer::new(entries.into_iter())),
    }
  }

  fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self.boolean() {
      Some(value) => visitor.visit_bool(value),
      None => Err(Error::invalid_type(self.unexpected(), &visitor)),
    }
  }

  fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_enum(self)
  }

  fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self.float() {
      Some(value) => visitor.visit_f32(value as f32),
      None => Err(Error::invalid_type(self.unexpected(), &visitor)),
    }
  }

  fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self.float() {
      Some(value) => visitor.visit_f64(value),
      None => Err(Error::invalid_type(self.unexpected(), &visitor)),
    }
  }

  fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i8(self.int()?)
  }

  fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i16(self.int()?)
  }

  fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i32(self.int()?)
  }

  fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i64(self.int()?)
  }

  fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_i128(self.int()?)
  }

  fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self {
      Self::Table(entries) => visitor.visit_map(MapDeserializer::new(entries.into_iter())),
      other => Err(Error::invalid_type(other.unexpected(), &visitor)),
    }
  }

  fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_newtype_struct(self)
  }

  fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self {
      Self::Null => visitor.visit_none(),
      other => visitor.visit_some(other),
    }
  }

  fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    match self {
      Self::List(values) => visitor.visit_seq(SeqDeserializer::new(values.into_iter())),
      Self::String(value) => visitor.visit_seq(SeqDeserializer::new(split(&value).into_iter())),
      other => Err(Error::invalid_type(other.unexpected(), &visitor)),
    }
  }

  fn deserialize_struct<V>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_map(visitor)
  }

  fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u8(self.int()?)
  }

  fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u16(self.int()?)
  }

  fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u32(self.int()?)
  }

  fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u64(self.int()?)
  }

  fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    visitor.visit_u128(self.int()?)
  }
}

impl<'de> EnumAccess<'de> for Value {
  type Error = Error;
  type Variant = Self;

  fn variant_seed<S>(self, seed: S) -> Result<(S::Value, Self::Variant)>
  where
    S: DeserializeSeed<'de>,
  {
    match self {
      Self::String(_) => Ok((seed.deserialize(self)?, Self::Null)),
      Self::Table(entries) if entries.len() == 1 => {
        let (key, value) = entries.into_iter().next().expect("table holds one entry");

        Ok((seed.deserialize(Self::String(key))?, value))
      }
      other => Err(Error::invalid_type(
        other.unexpected(),
        &"a string or a table with one key",
      )),
    }
  }
}

impl<'de> IntoDeserializer<'de, Error> for Value {
  type Deserializer = Self;

  fn into_deserializer(self) -> Self {
    self
  }
}

impl<'de> VariantAccess<'de> for Value {
  type Error = Error;

  fn newtype_variant_seed<S>(self, seed: S) -> Result<S::Value>
  where
    S: DeserializeSeed<'de>,
  {
    seed.deserialize(self)
  }

  fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_map(visitor)
  }

  fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
  where
    V: Visitor<'de>,
  {
    self.deserialize_seq(visitor)
  }

  fn unit_variant(self) -> Result<()> {
    Ok(())
  }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
  type Value = Value;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("any configuration value")
  }

  fn visit_bool<E>(self, value: bool) -> std::result::Result<Value, E> {
    Ok(Value::Bool(value))
  }

  fn visit_f64<E>(self, value: f64) -> std::result::Result<Value, E> {
    Ok(Value::Float(value))
  }

  fn visit_i64<E>(self, value: i64) -> std::result::Result<Value, E> {
    Ok(Value::Integer(i128::from(value)))
  }

  fn visit_i128<E>(self, value: i128) -> std::result::Result<Value, E> {
    Ok(Value::Integer(value))
  }

  fn visit_map<A>(self, mut access: A) -> std::result::Result<Value, A::Error>
  where
    A: MapAccess<'de>,
  {
    let mut entries = BTreeMap::new();

    while let Some((key, value)) = access.next_entry()? {
      entries.insert(key, value);
    }

    Ok(Value::Table(entries))
  }

  fn visit_none<E>(self) -> std::result::Result<Value, E> {
    Ok(Value::Null)
  }

  fn visit_seq<A>(self, mut access: A) -> std::result::Result<Value, A::Error>
  where
    A: SeqAccess<'de>,
  {
    let mut values = Vec::new();

    while let Some(value) = access.next_element()? {
      values.push(value);
    }

    Ok(Value::List(values))
  }

  fn visit_some<D>(self, deserializer: D) -> std::result::Result<Value, D::Error>
  where
    D: Deserializer<'de>,
  {
    Value::deserialize(deserializer)
  }

  fn visit_str<E>(self, value: &str) -> std::result::Result<Value, E> {
    Ok(Value::String(value.to_owned()))
  }

  fn visit_string<E>(self, value: String) -> std::result::Result<Value, E> {
    Ok(Value::String(value))
  }

  fn visit_u64<E>(self, value: u64) -> std::result::Result<Value, E> {
    Ok(Value::Integer(i128::from(value)))
  }

  fn visit_u128<E>(self, value: u128) -> std::result::Result<Value, E>
  where
    E: serde::de::Error,
  {
    i128::try_from(value)
      .map(Value::Integer)
      .map_err(|_| E::custom(format!("{value} is out of range")))
  }

  fn visit_unit<E>(self) -> std::result::Result<Value, E> {
    Ok(Value::Null)
  }
}

fn split(value: &str) -> Vec<Value> {
  value
    .split(',')
    .map(str::trim)
    .filter(|item| !item.is_empty())
    .map(|item| Value::String(item.to_owned()))
    .collect()
}

#[cfg(test)]
mod tests {
  use serde::de::value::{BytesDeserializer, I128Deserializer, StrDeserializer, U128Deserializer};

  use super::*;

  #[derive(Debug, Deserialize, PartialEq)]
  enum Mode {
    Detailed { level: u8 },
    Fast,
    Range(u8, u8),
    Timeout(u16),
  }

  /// Hands a visitor an optional value, which nothing in `serde` does.
  struct Optional(Option<Value>);

  #[derive(Debug, Deserialize, PartialEq)]
  struct Port(u16);

  #[derive(Debug, Deserialize, PartialEq)]
  struct Server {
    host: String,
    port: u16,
    tags: Vec<String>,
    tls: bool,
  }

  impl<'de> Deserializer<'de> for Optional {
    type Error = Error;

    forward_to_deserialize_any! {
      bool byte_buf bytes char enum f32 f64 i8 i16 i32 i64 i128 identifier ignored_any map newtype_struct option
      seq str string struct tuple tuple_struct u8 u16 u32 u64 u128 unit unit_struct
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
      V: Visitor<'de>,
    {
      match self.0 {
        Some(value) => visitor.visit_some(value),
        None => visitor.visit_none(),
      }
    }
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

  mod value {
    use super::*;

    mod deserialize_any {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_hands_every_shape_back_unchanged() {
        for value in [
          Value::Bool(true),
          Value::Float(1.5),
          Value::Integer(8080),
          Value::List(vec![string("web")]),
          Value::Null,
          string("localhost"),
          table(vec![("host", string("localhost"))]),
        ] {
          assert_eq!(Value::deserialize(value.clone()).unwrap(), value);
        }
      }

      #[test]
      fn it_keeps_an_integer_too_wide_for_an_i64() {
        let value = Value::Integer(i128::from(u64::MAX));

        assert_eq!(Value::deserialize(value.clone()).unwrap(), value);
      }

      #[test]
      fn it_keeps_an_integer_too_wide_for_a_u64() {
        let value = Value::Integer(i128::MIN);

        assert_eq!(Value::deserialize(value.clone()).unwrap(), value);
      }
    }

    mod deserialize_bool {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_accepts_a_native_bool() {
        assert_eq!(bool::deserialize(Value::Bool(true)).unwrap(), true);
      }

      #[test]
      fn it_coerces_the_strings_it_knows() {
        for truthy in ["1", "on", "TRUE", " yes "] {
          assert_eq!(bool::deserialize(string(truthy)).unwrap(), true, "{truthy}");
        }

        for falsy in ["0", "off", "FALSE", " no "] {
          assert_eq!(bool::deserialize(string(falsy)).unwrap(), false, "{falsy}");
        }
      }

      #[test]
      fn it_rejects_a_string_that_is_not_a_bool() {
        let error = bool::deserialize(string("maybe")).unwrap_err();

        assert!(error.to_string().contains("invalid type"), "{error}");
      }
    }

    mod deserialize_enum {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_a_newtype_variant_from_a_table_with_one_key() {
        let value = table(vec![("Timeout", Value::Integer(30))]);

        assert_eq!(Mode::deserialize(value).unwrap(), Mode::Timeout(30));
      }

      #[test]
      fn it_reads_a_struct_variant_from_a_nested_table() {
        let value = table(vec![("Detailed", table(vec![("level", Value::Integer(2))]))]);

        assert_eq!(
          Mode::deserialize(value).unwrap(),
          Mode::Detailed {
            level: 2,
          }
        );
      }

      #[test]
      fn it_reads_a_tuple_variant_from_a_list() {
        let value = table(vec![("Range", Value::List(vec![Value::Integer(1), Value::Integer(2)]))]);

        assert_eq!(Mode::deserialize(value).unwrap(), Mode::Range(1, 2));
      }

      #[test]
      fn it_reads_a_unit_variant_from_a_string() {
        assert_eq!(Mode::deserialize(string("Fast")).unwrap(), Mode::Fast);
      }

      #[test]
      fn it_rejects_a_table_holding_more_than_one_key() {
        let value = table(vec![("Fast", Value::Null), ("Timeout", Value::Integer(30))]);
        let error = Mode::deserialize(value).unwrap_err();

        assert!(error.to_string().contains("a table with one key"), "{error}");
      }

      #[test]
      fn it_rejects_a_value_that_names_no_variant() {
        let error = Mode::deserialize(Value::Integer(1)).unwrap_err();

        assert!(error.to_string().contains("a table with one key"), "{error}");
      }
    }

    mod deserialize_float {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_a_float_an_integer_or_a_string() {
        assert_eq!(f32::deserialize(Value::Float(1.5)).unwrap(), 1.5);
        assert_eq!(f32::deserialize(Value::Integer(2)).unwrap(), 2.0);
        assert_eq!(f32::deserialize(string(" 1.5 ")).unwrap(), 1.5);

        assert_eq!(f64::deserialize(Value::Float(1.5)).unwrap(), 1.5);
        assert_eq!(f64::deserialize(Value::Integer(2)).unwrap(), 2.0);
        assert_eq!(f64::deserialize(string(" 1.5 ")).unwrap(), 1.5);
      }

      #[test]
      fn it_rejects_a_value_that_is_not_a_number() {
        let error = f32::deserialize(Value::Bool(true)).unwrap_err();

        assert!(error.to_string().contains("invalid type"), "{error}");

        let error = f64::deserialize(Value::Bool(true)).unwrap_err();

        assert!(error.to_string().contains("invalid type"), "{error}");
      }
    }

    mod deserialize_map {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_a_table_into_a_map() {
        assert_eq!(
          BTreeMap::<String, u16>::deserialize(table(vec![("port", Value::Integer(8080))])).unwrap(),
          BTreeMap::from([("port".to_owned(), 8080)])
        );
      }

      #[test]
      fn it_rejects_a_value_that_is_not_a_table() {
        let error = BTreeMap::<String, u16>::deserialize(Value::Bool(true)).unwrap_err();

        assert!(error.to_string().contains("invalid type"), "{error}");
      }
    }

    mod deserialize_newtype_struct {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_the_value_the_struct_wraps() {
        assert_eq!(Port::deserialize(Value::Integer(8080)).unwrap(), Port(8080));
      }
    }

    mod deserialize_option {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_maps_null_to_none() {
        assert_eq!(Option::<u16>::deserialize(Value::Null).unwrap(), None);
      }

      #[test]
      fn it_wraps_a_present_value_in_some() {
        assert_eq!(Option::<u16>::deserialize(Value::Integer(8080)).unwrap(), Some(8080));
      }
    }

    mod deserialize_seq {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_a_native_list() {
        let value = Value::List(vec![string("web"), string("api")]);

        assert_eq!(Vec::<String>::deserialize(value).unwrap(), vec!["web", "api"]);
      }

      #[test]
      fn it_rejects_a_value_that_is_not_a_list() {
        let error = Vec::<String>::deserialize(Value::Bool(true)).unwrap_err();

        assert!(error.to_string().contains("invalid type"), "{error}");
      }

      #[test]
      fn it_splits_a_comma_separated_string() {
        assert_eq!(
          Vec::<String>::deserialize(string("a, b ,c")).unwrap(),
          vec!["a", "b", "c"]
        );
      }

      #[test]
      fn it_wraps_a_bare_string_in_a_single_element_list() {
        assert_eq!(Vec::<String>::deserialize(string("solo")).unwrap(), vec!["solo"]);
      }
    }

    mod deserialize_struct {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_coerces_strings_into_the_target_type() {
        let value = table(vec![
          ("host", string("localhost")),
          ("port", string("8080")),
          ("tags", string("web, api")),
          ("tls", string("yes")),
        ]);

        assert_eq!(
          Server::deserialize(value).unwrap(),
          Server {
            host: "localhost".to_owned(),
            port: 8080,
            tags: vec!["web".to_owned(), "api".to_owned()],
            tls: true,
          }
        );
      }

      #[test]
      fn it_reads_native_types_unchanged() {
        let value = table(vec![
          ("host", string("localhost")),
          ("port", Value::Integer(8080)),
          ("tags", Value::List(vec![string("web")])),
          ("tls", Value::Bool(true)),
        ]);

        assert_eq!(
          Server::deserialize(value).unwrap(),
          Server {
            host: "localhost".to_owned(),
            port: 8080,
            tags: vec!["web".to_owned()],
            tls: true,
          }
        );
      }
    }

    mod int {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_accepts_a_number_inside_the_target_range() {
        assert_eq!(u16::deserialize(Value::Integer(8080)).unwrap(), 8080);
      }

      #[test]
      fn it_parses_a_string_that_holds_a_number() {
        assert_eq!(u16::deserialize(string(" 8080 ")).unwrap(), 8080);
      }

      #[test]
      fn it_reads_every_signed_width() {
        assert_eq!(i8::deserialize(Value::Integer(-8)).unwrap(), -8);
        assert_eq!(i16::deserialize(Value::Integer(-16)).unwrap(), -16);
        assert_eq!(i32::deserialize(Value::Integer(-32)).unwrap(), -32);
        assert_eq!(i64::deserialize(Value::Integer(-64)).unwrap(), -64);
        assert_eq!(i128::deserialize(Value::Integer(-128)).unwrap(), -128);
      }

      #[test]
      fn it_reads_every_unsigned_width() {
        assert_eq!(u8::deserialize(Value::Integer(8)).unwrap(), 8);
        assert_eq!(u16::deserialize(Value::Integer(16)).unwrap(), 16);
        assert_eq!(u32::deserialize(Value::Integer(32)).unwrap(), 32);
        assert_eq!(u64::deserialize(Value::Integer(64)).unwrap(), 64);
        assert_eq!(u128::deserialize(Value::Integer(128)).unwrap(), 128);
      }

      #[test]
      fn it_rejects_a_number_outside_the_target_range() {
        let error = u16::deserialize(Value::Integer(70_000)).unwrap_err();

        assert!(error.to_string().contains("out of range"), "{error}");
      }

      #[test]
      fn it_rejects_a_value_that_holds_no_number() {
        let error = u16::deserialize(Value::Bool(true)).unwrap_err();

        assert!(error.to_string().contains("an integer"), "{error}");
      }
    }

    mod unexpected {
      use super::*;

      #[test]
      fn it_names_the_shape_it_found() {
        for (value, shape) in [
          (Value::Float(1.5), "floating point"),
          (Value::Integer(1), "integer"),
          (Value::List(Vec::new()), "sequence"),
          (Value::Null, "unit value"),
          (table(Vec::new()), "map"),
        ] {
          let error = bool::deserialize(value).unwrap_err();

          assert!(error.to_string().contains(shape), "{error}");
        }
      }

      #[test]
      fn it_stays_vague_about_an_integer_too_wide_for_an_i64() {
        let error = bool::deserialize(Value::Integer(i128::from(u64::MAX))).unwrap_err();

        assert!(error.to_string().contains("integer"), "{error}");
        assert!(!error.to_string().contains(&u64::MAX.to_string()), "{error}");
      }
    }
  }

  mod value_visitor {
    use super::*;

    mod visit_bytes {
      use super::*;

      #[test]
      fn it_says_what_it_wanted_instead() {
        let error = Value::deserialize(BytesDeserializer::<Error>::new(b"binary")).unwrap_err();

        assert!(error.to_string().contains("any configuration value"), "{error}");
      }
    }

    mod visit_i128 {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_the_number() {
        assert_eq!(
          Value::deserialize(I128Deserializer::<Error>::new(i128::MIN)).unwrap(),
          Value::Integer(i128::MIN)
        );
      }
    }

    mod visit_none {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_an_absent_value_as_null() {
        assert_eq!(Value::deserialize(Optional(None)).unwrap(), Value::Null);
      }
    }

    mod visit_some {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_reads_the_value_inside() {
        assert_eq!(
          Value::deserialize(Optional(Some(string("localhost")))).unwrap(),
          string("localhost")
        );
      }
    }

    mod visit_str {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_the_text() {
        assert_eq!(
          Value::deserialize(StrDeserializer::<Error>::new("localhost")).unwrap(),
          string("localhost")
        );
      }
    }

    mod visit_u128 {
      use pretty_assertions::assert_eq;

      use super::*;

      #[test]
      fn it_keeps_a_number_that_fits() {
        let value = u128::from(u64::MAX) + 1;

        assert_eq!(
          Value::deserialize(U128Deserializer::<Error>::new(value)).unwrap(),
          Value::Integer(i128::from(u64::MAX) + 1)
        );
      }

      #[test]
      fn it_rejects_a_number_too_wide_for_an_i128() {
        let error = Value::deserialize(U128Deserializer::<Error>::new(u128::MAX)).unwrap_err();

        assert!(error.to_string().contains("out of range"), "{error}");
      }
    }
  }

  mod split {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_drops_empty_items() {
      assert_eq!(split("a,,b,"), vec![string("a"), string("b")]);
    }

    #[test]
    fn it_trims_each_item() {
      assert_eq!(split(" a , b "), vec![string("a"), string("b")]);
    }
  }
}
