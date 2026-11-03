//! The one reader a load reads a save through: every request for an integer is
//! answered with the nearest value that integer's type can hold, so a mistyped
//! number in a hand edit costs its field's exactness rather than the save.
//!
//! It wraps the parsed JSON rather than naming each integer field, so the
//! journal's events, a report's counts, and a field added later hold the rule
//! without anyone remembering to ask. Everything else reads as JSON always does:
//! a string where a number belongs is still an error.

use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, VariantAccess,
    Visitor,
};
use serde::forward_to_deserialize_any;
use serde_json::{Error, Map, Number, Value};

/// A parsed value, read as a save is.
pub(super) struct Wide(pub(super) Value);

/// The nearest whole number from zero to `max`, a fraction rounding down.
// @spec SAVE-032
fn nearest(n: &Number, max: u64) -> u64 {
    if let Some(whole) = n.as_u64() {
        return whole.min(max);
    }
    if n.is_i64() {
        return 0; // a whole number that is not a u64 is a negative one
    }
    let float = n.as_f64().unwrap_or(0.0);
    if float <= 0.0 {
        0
    } else if float >= max as f64 {
        max
    } else {
        float as u64
    }
}

/// Answer one integer width's requests, leaving a non-number to fail as it would.
macro_rules! nearest_integer {
    ($($request:ident $visit:ident $width:ty),*) => {$(
        fn $request<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            match &self.0 {
                Value::Number(n) => visitor.$visit(nearest(n, u64::from(<$width>::MAX)) as $width),
                _ => self.deserialize_any(visitor),
            }
        }
    )*};
}

impl<'de> de::Deserializer<'de> for Wide {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Array(items) => visitor.visit_seq(Items(items.into_iter())),
            Value::Object(fields) => visitor.visit_map(Fields {
                fields: fields.into_iter(),
                value: None,
            }),
            other => de::Deserializer::deserialize_any(other, visitor),
        }
    }

    // A usize asks for a u64, so every unsigned width the save holds is here.
    nearest_integer!(
        deserialize_u8 visit_u8 u8,
        deserialize_u16 visit_u16 u16,
        deserialize_u32 visit_u32 u32,
        deserialize_u64 visit_u64 u64
    );

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }

    /// An enum is its variant's name, alone or as the one key over its content.
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.0 {
            Value::String(variant) => visitor.visit_enum(Variant {
                variant,
                content: None,
            }),
            Value::Object(fields) if fields.len() == 1 => {
                let (variant, content) = fields.into_iter().next().expect("length checked");
                visitor.visit_enum(Variant {
                    variant,
                    content: Some(content),
                })
            }
            other => Err(de::Error::custom(format!(
                "expected an enum, found {other}"
            ))),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u128 f32 f64 char str string bytes byte_buf unit unit_struct
        seq tuple tuple_struct map struct identifier ignored_any
    }
}

struct Items(std::vec::IntoIter<Value>);

impl<'de> SeqAccess<'de> for Items {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        self.0
            .next()
            .map(|item| seed.deserialize(Wide(item)))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}

struct Fields {
    fields: <Map<String, Value> as IntoIterator>::IntoIter,
    value: Option<Value>,
}

impl<'de> MapAccess<'de> for Fields {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        let Some((key, value)) = self.fields.next() else {
            return Ok(None);
        };
        self.value = Some(value);
        seed.deserialize(key.into_deserializer()).map(Some)
    }

    fn next_value_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, Error> {
        let value = self.value.take().expect("a key comes before its value");
        seed.deserialize(Wide(value))
    }
}

struct Variant {
    variant: String,
    content: Option<Value>,
}

impl<'de> EnumAccess<'de> for Variant {
    type Error = Error;
    type Variant = Content;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Content), Error> {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((variant, Content(self.content)))
    }
}

struct Content(Option<Value>);

impl Content {
    fn held(self) -> Result<Value, Error> {
        self.0
            .ok_or_else(|| de::Error::custom("a variant with content names none"))
    }
}

impl<'de> VariantAccess<'de> for Content {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.0 {
            None | Some(Value::Null) => Ok(()),
            Some(other) => Err(de::Error::custom(format!(
                "a variant with no content holds {other}"
            ))),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(Wide(self.held()?))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_any(Wide(self.held()?), visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_any(Wide(self.held()?), visitor)
    }
}
