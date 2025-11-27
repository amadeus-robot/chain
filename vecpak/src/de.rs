use serde::de::{self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use crate::{error::{Error, Result}, decode_varint};

pub struct Deserializer<'de> {
    input: &'de [u8],
    pos: usize,
}

pub fn from_slice<'a, T: Deserialize<'a>>(input: &'a [u8]) -> Result<T> {
    let mut deserializer = Deserializer { input, pos: 0 };
    let value = T::deserialize(&mut deserializer)?;
    if deserializer.pos != input.len() { return Err(Error::TrailingBytes); }
    Ok(value)
}

impl<'de> Deserializer<'de> {
    fn read_byte(&mut self) -> Result<u8> {
        if self.pos >= self.input.len() { return Err(Error::Eof); }
        let byte = self.input[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    fn read_bytes(&mut self, count: usize) -> Result<&'de [u8]> {
        if self.input.len().saturating_sub(self.pos) < count { return Err(Error::Eof); }
        let slice = &self.input[self.pos..self.pos + count];
        self.pos += count;
        Ok(slice)
    }

    fn read_varint(&mut self) -> Result<i128> {
        decode_varint(self.input, &mut self.pos).map_err(|e| Error::Message(e.into()))
    }

    fn read_length(&mut self) -> Result<usize> {
        let num = self.read_varint()?;
        if num < 0 { return Err(Error::InvalidLength); }
        usize::try_from(num).map_err(|_| Error::InvalidLength)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let tag = self.read_byte()?;
        match tag {
            0 => visitor.visit_unit(),
            1 => visitor.visit_bool(true),
            2 => visitor.visit_bool(false),
            3 => visitor.visit_i128(self.read_varint()?),
            5 => {
                let len = self.read_length()?;
                let bytes = self.read_bytes(len)?;
                visitor.visit_borrowed_bytes(bytes)
            }
            6 => {
                let len = self.read_length()?;
                visitor.visit_seq(SequenceDeserializer { de: self, remaining: len })
            }
            7 => {
                let len = self.read_length()?;
                visitor.visit_map(MapDeserializer { de: self, remaining: len })
            }
            _ => Err(Error::InvalidTag),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.read_byte()? {
            1 => visitor.visit_bool(true),
            2 => visitor.visit_bool(false),
            _ => Err(Error::InvalidTag),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_i8(self.read_varint()? as i8)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_i16(self.read_varint()? as i16)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_i32(self.read_varint()? as i32)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_i64(self.read_varint()? as i64)
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_i128(self.read_varint()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_u8(self.read_varint()? as u8)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_u16(self.read_varint()? as u16)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_u32(self.read_varint()? as u32)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_u64(self.read_varint()? as u64)
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 3 { return Err(Error::InvalidTag); }
        visitor.visit_u128(self.read_varint()? as u128)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message("floats not supported".into()))
    }

    fn deserialize_f64<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message("floats not supported".into()))
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 5 { return Err(Error::InvalidTag); }
        let len = self.read_length()?;
        let bytes = self.read_bytes(len)?;
        let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
        visitor.visit_borrowed_str(text)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 5 { return Err(Error::InvalidTag); }
        let len = self.read_length()?;
        let bytes = self.read_bytes(len)?;
        visitor.visit_borrowed_bytes(bytes)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.input.get(self.pos) == Some(&0) {
            self.pos += 1;
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 0 { return Err(Error::InvalidTag); }
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 6 { return Err(Error::InvalidTag); }
        let len = self.read_length()?;
        visitor.visit_seq(SequenceDeserializer { de: self, remaining: len })
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.read_byte()? != 7 { return Err(Error::InvalidTag); }
        let len = self.read_length()?;
        visitor.visit_map(MapDeserializer { de: self, remaining: len })
    }

    fn deserialize_struct<V: Visitor<'de>>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(self, _name: &'static str, _variants: &'static [&'static str], visitor: V) -> Result<V::Value> {
        visitor.visit_enum(EnumDeserializer { de: self })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_any(visitor)
    }
}

struct SequenceDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'de, 'a> SeqAccess<'de> for SequenceDeserializer<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        if self.remaining == 0 { return Ok(None); }
        self.remaining -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct MapDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    remaining: usize,
}

impl<'de, 'a> MapAccess<'de> for MapDeserializer<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.remaining == 0 { return Ok(None); }
        self.remaining -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.de)
    }
}

struct EnumDeserializer<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'de, 'a> de::EnumAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        // Check if this is a unit variant (just a string) or struct/tuple variant (proplist wrapper)
        let tag = self.de.read_byte()?;
        if tag == 5 {
            // Unit variant: just a string
            let len = self.de.read_length()?;
            let bytes = self.de.read_bytes(len)?;
            let text = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
            let val = seed.deserialize(de::value::BorrowedStrDeserializer::new(text))?;
            Ok((val, self))
        } else if tag == 7 {
            // Struct/tuple variant: proplist { variant_name: content }
            let outer_len = self.de.read_length()?;
            if outer_len != 1 { return Err(Error::Message("expected single-entry proplist for enum".into())); }
            let val = seed.deserialize(&mut *self.de)?;
            Ok((val, self))
        } else {
            Err(Error::InvalidTag)
        }
    }
}

impl<'de, 'a> de::VariantAccess<'de> for EnumDeserializer<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> { Ok(()) }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value> {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}