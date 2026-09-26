//! Reject floats before serde_json can silently convert NaN/Infinity to null.
use serde::ser::Error as _;
use serde::{Serialize, ser::*};

pub(crate) struct IntegerJson {
    depth: usize,
}

impl IntegerJson {
    pub(crate) fn root() -> Self {
        Self { depth: 0 }
    }
    fn child(&self) -> Result<Self, Error> {
        if self.depth >= 256 {
            return Err(Error::custom("canonical JSON depth exceeds 256"));
        }
        Ok(Self {
            depth: self.depth + 1,
        })
    }
}

type Error = serde_json::Error;

macro_rules! accept {
    ($($method:ident($ty:ty)),* $(,)?) => {$(
        fn $method(self, _: $ty) -> Result<(), Error> { Ok(()) }
    )*};
}

impl Serializer for IntegerJson {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    accept!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_char(char),
        serialize_str(&str),
        serialize_bytes(&[u8])
    );
    fn serialize_i64(self, value: i64) -> Result<(), Error> {
        if value.unsigned_abs() > 9_007_199_254_740_991 {
            Err(Error::custom("integer exceeds exact JSON wire range"))
        } else {
            Ok(())
        }
    }
    fn serialize_u64(self, value: u64) -> Result<(), Error> {
        if value > 9_007_199_254_740_991 {
            Err(Error::custom("integer exceeds exact JSON wire range"))
        } else {
            Ok(())
        }
    }
    fn serialize_i128(self, value: i128) -> Result<(), Error> {
        self.serialize_i64(i64::try_from(value).map_err(Error::custom)?)
    }
    fn serialize_u128(self, value: u128) -> Result<(), Error> {
        self.serialize_u64(u64::try_from(value).map_err(Error::custom)?)
    }
    fn serialize_f32(self, _: f32) -> Result<(), Error> {
        Err(Error::custom("float is not a canonical wire number"))
    }
    fn serialize_f64(self, _: f64) -> Result<(), Error> {
        Err(Error::custom("float is not a canonical wire number"))
    }
    fn serialize_none(self) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Error> {
        value.serialize(self.child()?)
    }
    fn serialize_unit(self) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Error> {
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self.child()?)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self.child()?)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self, Error> {
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self, Error> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Error> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Error> {
        Ok(self)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self, Error> {
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Error> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Error> {
        Ok(self)
    }
}

macro_rules! compound {
    ($trait:ident, $method:ident) => {
        impl $trait for IntegerJson {
            type Ok = ();
            type Error = Error;
            fn $method<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
                value.serialize(self.child()?)
            }
            fn end(self) -> Result<(), Error> {
                Ok(())
            }
        }
    };
}
compound!(SerializeSeq, serialize_element);
compound!(SerializeTuple, serialize_element);
compound!(SerializeTupleStruct, serialize_field);
compound!(SerializeTupleVariant, serialize_field);

impl SerializeMap for IntegerJson {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Error> {
        key.serialize(self.child()?)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
        value.serialize(self.child()?)
    }
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}
impl SerializeStruct for IntegerJson {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self.child()?)
    }
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}
impl SerializeStructVariant for IntegerJson {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        value.serialize(self.child()?)
    }
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}
