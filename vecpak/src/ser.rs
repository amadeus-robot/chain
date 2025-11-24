use serde::{ser, Serialize};
use crate::{error::{Error, Result}, encode_varint};

pub struct Serializer {
    output: Vec<u8>,
}

pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut serializer = Serializer { output: Vec::new() };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = MapSerializer<'a>;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output.push(if v { 1 } else { 2 });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_i16(self, v: i16) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_i32(self, v: i32) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_i64(self, v: i64) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_i128(self, v: i128) -> Result<()> {
        self.output.push(3);
        encode_varint(&mut self.output, v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_u16(self, v: u16) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_u32(self, v: u32) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_u64(self, v: u64) -> Result<()> { self.serialize_i128(v as i128) }
    fn serialize_u128(self, v: u128) -> Result<()> {
        if v > i128::MAX as u128 { return Err(Error::Message("u128 too large".into())); }
        self.serialize_i128(v as i128)
    }

    fn serialize_f32(self, _v: f32) -> Result<()> { Err(Error::Message("floats not supported".into())) }
    fn serialize_f64(self, _v: f64) -> Result<()> { Err(Error::Message("floats not supported".into())) }

    fn serialize_char(self, v: char) -> Result<()> { self.serialize_str(&v.to_string()) }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output.push(5);
        encode_varint(&mut self.output, v.len() as i128);
        self.output.extend_from_slice(v.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.output.push(5);
        encode_varint(&mut self.output, v.len() as i128);
        self.output.extend_from_slice(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<()> { self.output.push(0); Ok(()) }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> { value.serialize(self) }
    fn serialize_unit(self) -> Result<()> { self.output.push(0); Ok(()) }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> { self.serialize_unit() }

    fn serialize_unit_variant(self, _name: &'static str, _idx: u32, variant: &'static str) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, _name: &'static str, value: &T) -> Result<()> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(self, _name: &'static str, _idx: u32, variant: &'static str, value: &T) -> Result<()> {
        self.output.push(7);
        encode_varint(&mut self.output, 1);
        self.serialize_str(variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.output.push(6);
        encode_varint(&mut self.output, len.unwrap_or(0) as i128);
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> { self.serialize_seq(Some(len)) }
    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(self, _name: &'static str, _idx: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        self.output.push(7);
        encode_varint(&mut self.output, 1);
        self.serialize_str(variant)?;
        self.serialize_seq(Some(len))?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer::new(self))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(MapSerializer::new(self))
    }

    fn serialize_struct_variant(self, _name: &'static str, _idx: u32, variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant> {
        self.output.push(7);
        encode_varint(&mut self.output, 1);
        self.serialize_str(variant)?;
        self.output.push(7);
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> { value.serialize(&mut **self) }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> { value.serialize(&mut **self) }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> { value.serialize(&mut **self) }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> { value.serialize(&mut **self) }
    fn end(self) -> Result<()> { Ok(()) }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        key.serialize(&mut **self)?;
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> { Ok(()) }
}

pub struct MapSerializer<'a> {
    ser: &'a mut Serializer,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl<'a> MapSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        MapSerializer { ser, entries: Vec::new() }
    }

    fn end_map(self) -> Result<()> {
        let mut entries = self.entries;
        entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        self.ser.output.push(7);
        encode_varint(&mut self.ser.output, entries.len() as i128);
        for (key_bytes, value_bytes) in entries {
            self.ser.output.extend_from_slice(&key_bytes);
            self.ser.output.extend_from_slice(&value_bytes);
        }
        Ok(())
    }
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        let mut key_serializer = Serializer { output: Vec::new() };
        key.serialize(&mut key_serializer)?;
        self.entries.push((key_serializer.output, Vec::new()));
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        let mut value_serializer = Serializer { output: Vec::new() };
        value.serialize(&mut value_serializer)?;
        self.entries.last_mut().unwrap().1 = value_serializer.output;
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.end_map()
    }
}

impl<'a> ser::SerializeStruct for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        ser::SerializeMap::serialize_key(self, key)?;
        ser::SerializeMap::serialize_value(self, value)
    }

    fn end(self) -> Result<()> {
        self.end_map()
    }
}