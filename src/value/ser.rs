use std::collections::BTreeMap;

use serde::{
  Serialize,
  ser::{
    Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
  },
};

use crate::{Error, Result, Value};

struct KeySerializer;

impl serde::Serializer for KeySerializer {
  type Error = Error;
  type Ok = String;
  type SerializeMap = Impossible<String, Error>;
  type SerializeSeq = Impossible<String, Error>;
  type SerializeStruct = Impossible<String, Error>;
  type SerializeStructVariant = Impossible<String, Error>;
  type SerializeTuple = Impossible<String, Error>;
  type SerializeTupleStruct = Impossible<String, Error>;
  type SerializeTupleVariant = Impossible<String, Error>;

  fn collect_str<T>(self, value: &T) -> Result<String>
  where
    T: ?Sized + std::fmt::Display,
  {
    Ok(value.to_string())
  }

  fn serialize_bool(self, value: bool) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_bytes(self, _value: &[u8]) -> Result<String> {
    Err(unsupported_key())
  }

  fn serialize_char(self, value: char) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_f32(self, value: f32) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_f64(self, value: f64) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_i8(self, value: i8) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_i16(self, value: i16) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_i32(self, value: i32) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_i64(self, value: i64) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_i128(self, value: i128) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
    Err(unsupported_key())
  }

  fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<String>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_newtype_variant<T>(
    self,
    _name: &'static str,
    _index: u32,
    _variant: &'static str,
    _value: &T,
  ) -> Result<String>
  where
    T: ?Sized + Serialize,
  {
    Err(unsupported_key())
  }

  fn serialize_none(self) -> Result<String> {
    Err(unsupported_key())
  }

  fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
    Err(unsupported_key())
  }

  fn serialize_some<T>(self, value: &T) -> Result<String>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_str(self, value: &str) -> Result<String> {
    Ok(value.to_owned())
  }

  fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
    Err(unsupported_key())
  }

  fn serialize_struct_variant(
    self,
    _name: &'static str,
    _index: u32,
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStructVariant> {
    Err(unsupported_key())
  }

  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
    Err(unsupported_key())
  }

  fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct> {
    Err(unsupported_key())
  }

  fn serialize_tuple_variant(
    self,
    _name: &'static str,
    _index: u32,
    _variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleVariant> {
    Err(unsupported_key())
  }

  fn serialize_u8(self, value: u8) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_u16(self, value: u16) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_u32(self, value: u32) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_u64(self, value: u64) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_u128(self, value: u128) -> Result<String> {
    Ok(value.to_string())
  }

  fn serialize_unit(self) -> Result<String> {
    Err(unsupported_key())
  }

  fn serialize_unit_struct(self, _name: &'static str) -> Result<String> {
    Err(unsupported_key())
  }

  fn serialize_unit_variant(self, _name: &'static str, _index: u32, variant: &'static str) -> Result<String> {
    Ok(variant.to_owned())
  }
}

struct SerializeList {
  items: Vec<Value>,
  name: Option<&'static str>,
}

impl SerializeList {
  fn finish(self) -> Result<Value> {
    let list = Value::List(self.items);

    Ok(match self.name {
      Some(name) => Value::Table(BTreeMap::from([(name.to_owned(), list)])),
      None => list,
    })
  }

  fn push<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.items.push(value.serialize(Serializer)?);

    Ok(())
  }
}

impl SerializeSeq for SerializeList {
  type Error = Error;
  type Ok = Value;

  fn end(self) -> Result<Value> {
    self.finish()
  }

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.push(value)
  }
}

impl SerializeTuple for SerializeList {
  type Error = Error;
  type Ok = Value;

  fn end(self) -> Result<Value> {
    self.finish()
  }

  fn serialize_element<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.push(value)
  }
}

impl SerializeTupleStruct for SerializeList {
  type Error = Error;
  type Ok = Value;

  fn end(self) -> Result<Value> {
    self.finish()
  }

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.push(value)
  }
}

impl SerializeTupleVariant for SerializeList {
  type Error = Error;
  type Ok = Value;

  fn end(self) -> Result<Value> {
    self.finish()
  }

  fn serialize_field<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.push(value)
  }
}

struct SerializeTable {
  entries: BTreeMap<String, Value>,
  key: Option<String>,
  name: Option<&'static str>,
}

impl SerializeTable {
  fn finish(self) -> Result<Value> {
    let table = Value::Table(self.entries);

    Ok(match self.name {
      Some(name) => Value::Table(BTreeMap::from([(name.to_owned(), table)])),
      None => table,
    })
  }

  fn insert<T>(&mut self, key: String, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.entries.insert(key, value.serialize(Serializer)?);

    Ok(())
  }
}

impl SerializeMap for SerializeTable {
  type Error = Error;
  type Ok = Value;

  fn end(self) -> Result<Value> {
    self.finish()
  }

  fn serialize_key<T>(&mut self, key: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.key = Some(key.serialize(KeySerializer)?);

    Ok(())
  }

  fn serialize_value<T>(&mut self, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    let key = self
      .key
      .take()
      .ok_or_else(|| Error::Serialize("a map value arrived before its key".to_owned()))?;

    self.insert(key, value)
  }
}

impl SerializeStruct for SerializeTable {
  type Error = Error;
  type Ok = Value;

  fn end(self) -> Result<Value> {
    self.finish()
  }

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.insert(key.to_owned(), value)
  }
}

impl SerializeStructVariant for SerializeTable {
  type Error = Error;
  type Ok = Value;

  fn end(self) -> Result<Value> {
    self.finish()
  }

  fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
  where
    T: ?Sized + Serialize,
  {
    self.insert(key.to_owned(), value)
  }
}

struct Serializer;

impl serde::Serializer for Serializer {
  type Error = Error;
  type Ok = Value;
  type SerializeMap = SerializeTable;
  type SerializeSeq = SerializeList;
  type SerializeStruct = SerializeTable;
  type SerializeStructVariant = SerializeTable;
  type SerializeTuple = SerializeList;
  type SerializeTupleStruct = SerializeList;
  type SerializeTupleVariant = SerializeList;

  fn serialize_bool(self, value: bool) -> Result<Value> {
    Ok(Value::Bool(value))
  }

  fn serialize_bytes(self, value: &[u8]) -> Result<Value> {
    Ok(Value::List(
      value.iter().map(|byte| Value::Integer((*byte).into())).collect(),
    ))
  }

  fn serialize_char(self, value: char) -> Result<Value> {
    Ok(Value::String(value.to_string()))
  }

  fn serialize_f32(self, value: f32) -> Result<Value> {
    Ok(Value::Float(value.into()))
  }

  fn serialize_f64(self, value: f64) -> Result<Value> {
    Ok(Value::Float(value))
  }

  fn serialize_i8(self, value: i8) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_i16(self, value: i16) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_i32(self, value: i32) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_i64(self, value: i64) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_i128(self, value: i128) -> Result<Value> {
    Ok(Value::Integer(value))
  }

  fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
    Ok(SerializeTable {
      entries: BTreeMap::new(),
      key: None,
      name: None,
    })
  }

  fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_newtype_variant<T>(
    self,
    _name: &'static str,
    _index: u32,
    variant: &'static str,
    value: &T,
  ) -> Result<Value>
  where
    T: ?Sized + Serialize,
  {
    Ok(Value::Table(BTreeMap::from([(
      variant.to_owned(),
      value.serialize(self)?,
    )])))
  }

  fn serialize_none(self) -> Result<Value> {
    Ok(Value::Null)
  }

  fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
    Ok(SerializeList {
      items: Vec::new(),
      name: None,
    })
  }

  fn serialize_some<T>(self, value: &T) -> Result<Value>
  where
    T: ?Sized + Serialize,
  {
    value.serialize(self)
  }

  fn serialize_str(self, value: &str) -> Result<Value> {
    Ok(Value::String(value.to_owned()))
  }

  fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
    Ok(SerializeTable {
      entries: BTreeMap::new(),
      key: None,
      name: None,
    })
  }

  fn serialize_struct_variant(
    self,
    _name: &'static str,
    _index: u32,
    variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeStructVariant> {
    Ok(SerializeTable {
      entries: BTreeMap::new(),
      key: None,
      name: Some(variant),
    })
  }

  fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
    Ok(SerializeList {
      items: Vec::new(),
      name: None,
    })
  }

  fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct> {
    Ok(SerializeList {
      items: Vec::new(),
      name: None,
    })
  }

  fn serialize_tuple_variant(
    self,
    _name: &'static str,
    _index: u32,
    variant: &'static str,
    _len: usize,
  ) -> Result<Self::SerializeTupleVariant> {
    Ok(SerializeList {
      items: Vec::new(),
      name: Some(variant),
    })
  }

  fn serialize_u8(self, value: u8) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_u16(self, value: u16) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_u32(self, value: u32) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_u64(self, value: u64) -> Result<Value> {
    Ok(Value::Integer(value.into()))
  }

  fn serialize_u128(self, value: u128) -> Result<Value> {
    i128::try_from(value)
      .map(Value::Integer)
      .map_err(|_| Error::Serialize(format!("{value} is out of range")))
  }

  fn serialize_unit(self) -> Result<Value> {
    Ok(Value::Null)
  }

  fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
    Ok(Value::Null)
  }

  fn serialize_unit_variant(self, _name: &'static str, _index: u32, variant: &'static str) -> Result<Value> {
    Ok(Value::String(variant.to_owned()))
  }
}

pub fn to_value(value: impl Serialize) -> Result<Value> {
  value.serialize(Serializer)
}

fn unsupported_key() -> Error {
  Error::Serialize("a table key must be a string or a scalar".to_owned())
}

#[cfg(test)]
mod tests {
  use serde::{Serialize, ser::Serializer as _};

  use super::*;

  /// Writes bytes, which nothing `serde` derives does.
  struct Bytes(&'static [u8]);

  #[derive(Serialize)]
  struct Marker;

  #[derive(Serialize)]
  enum Mode {
    Detailed { level: u8 },
    Fast,
    Range(u8, u8),
    Timeout(u16),
  }

  #[derive(Serialize)]
  struct Pair(u8, u8);

  #[derive(Serialize)]
  struct Port(u16);

  #[derive(Serialize)]
  struct Server {
    host: String,
    port: u16,
    tags: Vec<String>,
  }

  impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
      S: serde::Serializer,
    {
      serializer.serialize_bytes(self.0)
    }
  }

  fn list(items: Vec<Value>) -> Value {
    Value::List(items)
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

  mod key_serializer {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_keeps_a_string_as_it_is() {
      assert_eq!(KeySerializer.serialize_str("host").unwrap(), "host");
    }

    #[test]
    fn it_names_a_unit_variant() {
      assert_eq!(KeySerializer.serialize_unit_variant("Mode", 0, "Fast").unwrap(), "Fast");
    }

    #[test]
    fn it_reads_through_a_newtype_and_an_option() {
      assert_eq!(Port(8080).serialize(KeySerializer).unwrap(), "8080");
      assert_eq!(Some(8080_u16).serialize(KeySerializer).unwrap(), "8080");
    }

    #[test]
    fn it_rejects_everything_that_is_not_a_scalar() {
      assert!(KeySerializer.serialize_bytes(b"binary").is_err());
      assert!(KeySerializer.serialize_map(None).is_err());
      assert!(
        KeySerializer
          .serialize_newtype_variant("Mode", 0, "Timeout", &30_u16)
          .is_err()
      );
      assert!(KeySerializer.serialize_none().is_err());
      assert!(KeySerializer.serialize_seq(None).is_err());
      assert!(KeySerializer.serialize_struct("Server", 1).is_err());
      assert!(
        KeySerializer
          .serialize_struct_variant("Mode", 0, "Detailed", 1)
          .is_err()
      );
      assert!(KeySerializer.serialize_tuple(2).is_err());
      assert!(KeySerializer.serialize_tuple_struct("Pair", 2).is_err());
      assert!(KeySerializer.serialize_tuple_variant("Mode", 0, "Range", 2).is_err());
      assert!(KeySerializer.serialize_unit().is_err());
      assert!(KeySerializer.serialize_unit_struct("Marker").is_err());

      let error = KeySerializer.serialize_unit().unwrap_err();

      assert!(error.to_string().contains("must be a string or a scalar"), "{error}");
    }

    #[test]
    fn it_writes_every_scalar_as_text() {
      assert_eq!(KeySerializer.collect_str("8080").unwrap(), "8080");
      assert_eq!(KeySerializer.serialize_bool(true).unwrap(), "true");
      assert_eq!(KeySerializer.serialize_char('a').unwrap(), "a");
      assert_eq!(KeySerializer.serialize_f32(1.5).unwrap(), "1.5");
      assert_eq!(KeySerializer.serialize_f64(1.5).unwrap(), "1.5");
      assert_eq!(KeySerializer.serialize_i8(-8).unwrap(), "-8");
      assert_eq!(KeySerializer.serialize_i16(-16).unwrap(), "-16");
      assert_eq!(KeySerializer.serialize_i32(-32).unwrap(), "-32");
      assert_eq!(KeySerializer.serialize_i64(-64).unwrap(), "-64");
      assert_eq!(KeySerializer.serialize_i128(-128).unwrap(), "-128");
      assert_eq!(KeySerializer.serialize_u8(8).unwrap(), "8");
      assert_eq!(KeySerializer.serialize_u16(16).unwrap(), "16");
      assert_eq!(KeySerializer.serialize_u32(32).unwrap(), "32");
      assert_eq!(KeySerializer.serialize_u64(64).unwrap(), "64");
      assert_eq!(KeySerializer.serialize_u128(128).unwrap(), "128");
    }
  }

  mod serialize_table {
    use super::*;

    mod serialize_value {
      use super::*;

      #[test]
      fn it_rejects_a_value_that_arrives_before_its_key() {
        let mut table = SerializeTable {
          entries: BTreeMap::new(),
          key: None,
          name: None,
        };

        let error = SerializeMap::serialize_value(&mut table, &8080_u16).unwrap_err();

        assert!(error.to_string().contains("before its key"), "{error}");
      }
    }
  }

  mod to_value {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn it_rejects_a_table_key_that_is_not_a_scalar() {
      let mut entries = BTreeMap::new();
      entries.insert((1_u8, 2_u8), 3_u8);

      let error = to_value(entries).unwrap_err();

      assert!(error.to_string().contains("must be a string or a scalar"), "{error}");
    }

    #[test]
    fn it_rejects_an_unsigned_number_too_wide_for_an_i128() {
      let error = to_value(u128::MAX).unwrap_err();

      assert!(error.to_string().contains("out of range"), "{error}");
    }

    #[test]
    fn it_turns_a_map_into_a_table() {
      assert_eq!(
        to_value(BTreeMap::from([("port", 8080_u16)])).unwrap(),
        table(vec![("port", Value::Integer(8080))])
      );
    }

    #[test]
    fn it_turns_a_newtype_struct_into_the_value_it_wraps() {
      assert_eq!(to_value(Port(8080)).unwrap(), Value::Integer(8080));
    }

    #[test]
    fn it_turns_a_newtype_variant_into_a_table_with_one_key() {
      assert_eq!(
        to_value(Mode::Timeout(30)).unwrap(),
        table(vec![("Timeout", Value::Integer(30))])
      );
    }

    #[test]
    fn it_turns_a_sequence_into_a_list() {
      assert_eq!(
        to_value(vec!["web", "api"]).unwrap(),
        list(vec![string("web"), string("api")])
      );
    }

    #[test]
    fn it_turns_a_struct_into_a_table() {
      let server = Server {
        host: "localhost".to_owned(),
        port: 8080,
        tags: vec!["web".to_owned()],
      };

      assert_eq!(
        to_value(server).unwrap(),
        table(vec![
          ("host", string("localhost")),
          ("port", Value::Integer(8080)),
          ("tags", list(vec![string("web")])),
        ])
      );
    }

    #[test]
    fn it_turns_a_struct_variant_into_a_nested_table() {
      assert_eq!(
        to_value(Mode::Detailed {
          level: 2,
        })
        .unwrap(),
        table(vec![("Detailed", table(vec![("level", Value::Integer(2))]))])
      );
    }

    #[test]
    fn it_turns_a_tuple_and_a_tuple_struct_into_a_list() {
      let expected = list(vec![Value::Integer(1), Value::Integer(2)]);

      assert_eq!(to_value((1_u8, 2_u8)).unwrap(), expected);
      assert_eq!(to_value(Pair(1, 2)).unwrap(), expected);
    }

    #[test]
    fn it_turns_a_tuple_variant_into_a_table_holding_a_list() {
      assert_eq!(
        to_value(Mode::Range(1, 2)).unwrap(),
        table(vec![("Range", list(vec![Value::Integer(1), Value::Integer(2)]))])
      );
    }

    #[test]
    fn it_turns_a_unit_variant_into_a_string() {
      assert_eq!(to_value(Mode::Fast).unwrap(), string("Fast"));
    }

    #[test]
    fn it_turns_bytes_into_a_list_of_numbers() {
      assert_eq!(
        to_value(Bytes(b"hi")).unwrap(),
        list(vec![Value::Integer(104), Value::Integer(105)])
      );
    }

    #[test]
    fn it_turns_every_signed_width_into_an_integer() {
      assert_eq!(to_value(-8_i8).unwrap(), Value::Integer(-8));
      assert_eq!(to_value(-16_i16).unwrap(), Value::Integer(-16));
      assert_eq!(to_value(-32_i32).unwrap(), Value::Integer(-32));
      assert_eq!(to_value(-64_i64).unwrap(), Value::Integer(-64));
      assert_eq!(to_value(-128_i128).unwrap(), Value::Integer(-128));
    }

    #[test]
    fn it_turns_every_unsigned_width_into_an_integer() {
      assert_eq!(to_value(8_u8).unwrap(), Value::Integer(8));
      assert_eq!(to_value(16_u16).unwrap(), Value::Integer(16));
      assert_eq!(to_value(32_u32).unwrap(), Value::Integer(32));
      assert_eq!(to_value(64_u64).unwrap(), Value::Integer(64));
      assert_eq!(to_value(128_u128).unwrap(), Value::Integer(128));
    }

    #[test]
    fn it_turns_none_into_null() {
      assert_eq!(to_value(Option::<u16>::None).unwrap(), Value::Null);
    }

    #[test]
    fn it_turns_text_into_a_string() {
      assert_eq!(to_value('a').unwrap(), string("a"));
      assert_eq!(to_value("localhost").unwrap(), string("localhost"));
    }

    #[test]
    fn it_turns_unit_and_a_unit_struct_into_null() {
      assert_eq!(to_value(()).unwrap(), Value::Null);
      assert_eq!(to_value(Marker).unwrap(), Value::Null);
    }

    #[test]
    fn it_unwraps_some() {
      assert_eq!(to_value(Some(8080_u16)).unwrap(), Value::Integer(8080));
    }

    #[test]
    fn it_writes_a_bool_and_a_float() {
      assert_eq!(to_value(true).unwrap(), Value::Bool(true));
      assert_eq!(to_value(1.5_f32).unwrap(), Value::Float(1.5));
      assert_eq!(to_value(1.5_f64).unwrap(), Value::Float(1.5));
    }
  }
}
