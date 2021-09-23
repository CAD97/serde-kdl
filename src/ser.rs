use {
    crate::*,
    paste::paste,
    serde::ser::*,
    std::{
        io::{self, prelude::*},
        marker::PhantomData,
    },
};

const INDENT_LITERAL: &str = unsafe { std::str::from_utf8_unchecked(&[b' '; u8::MAX as _]) };
const HASHES_LITERAL: &str = unsafe { std::str::from_utf8_unchecked(&[b'#'; u8::MAX as _]) };

fn count_needed_hashes(s: &str) -> usize {
    let mut outside_hash_count = 0;
    let mut cursor = 0;
    while let Some(quote_index) = s[cursor..].find('"').map(|i| cursor + i) {
        let not_hash_index = s[quote_index + 1..]
            .find(|c| c != '#')
            .map(|i| quote_index + 1 + i)
            .unwrap_or_else(|| s.len());
        let inside_hash_count = not_hash_index - quote_index - 1;
        outside_hash_count = outside_hash_count.max(inside_hash_count + 1);
        cursor = not_hash_index;
    }
    outside_hash_count
}

fn is_valid_kdl_identifier(s: &str) -> bool {
    !s.starts_with(&['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'][..])
        && !s.contains("//")
        && !s.contains(|c: char| {
            c as u32 <= 0x20
                || c as u32 > 0x10FFFF
                || r#"\/(){}<>;[]=,""#.contains(c)
                || c.is_whitespace()
        })
}

pub trait Format {
    type Sink: ?Sized;

    // Type annotation
    fn provide_type_annotation(&mut self, s: &mut Self::Sink, ty: &'static str) -> io::Result<()>;
    fn require_type_annotation(&mut self, s: &mut Self::Sink, ty: &'static str) -> io::Result<()>;

    // Primitives
    fn write_bool(&mut self, s: &mut Self::Sink, v: bool) -> io::Result<()>;
    fn write_u8(&mut self, s: &mut Self::Sink, v: u8) -> io::Result<()>;
    fn write_u16(&mut self, s: &mut Self::Sink, v: u16) -> io::Result<()>;
    fn write_u32(&mut self, s: &mut Self::Sink, v: u32) -> io::Result<()>;
    fn write_u64(&mut self, s: &mut Self::Sink, v: u64) -> io::Result<()>;
    fn write_u128(&mut self, s: &mut Self::Sink, v: u128) -> io::Result<()>;
    fn write_i8(&mut self, s: &mut Self::Sink, v: i8) -> io::Result<()>;
    fn write_i16(&mut self, s: &mut Self::Sink, v: i16) -> io::Result<()>;
    fn write_i32(&mut self, s: &mut Self::Sink, v: i32) -> io::Result<()>;
    fn write_i64(&mut self, s: &mut Self::Sink, v: i64) -> io::Result<()>;
    fn write_i128(&mut self, s: &mut Self::Sink, v: i128) -> io::Result<()>;
    fn write_f32(&mut self, s: &mut Self::Sink, v: f32) -> io::Result<()>;
    fn write_f64(&mut self, s: &mut Self::Sink, v: f64) -> io::Result<()>;
    fn write_unit(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn write_string(&mut self, s: &mut Self::Sink, v: &str) -> io::Result<()>;
    fn write_bytes(&mut self, s: &mut Self::Sink, v: &[u8]) -> io::Result<()>;

    // Struct, Tuple, Seq
    fn begin_group(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn end_group(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn begin_field(&mut self, s: &mut Self::Sink, name: Option<&'static str>) -> io::Result<()>;
    fn end_field(&mut self, s: &mut Self::Sink) -> io::Result<()>;

    // Maps
    fn begin_map(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn begin_map_key(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn end_map_key(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn begin_map_value(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn end_map_value(&mut self, s: &mut Self::Sink) -> io::Result<()>;
    fn end_map(&mut self, s: &mut Self::Sink) -> io::Result<()>;
}

/// A formatter for SiK that prioritizes single-pass serialization.
///
/// This allows it to serialize to an arbitrary `io::Write`, though
/// the emitted SiK may not be ideal (containing unnecessary blocks).
///
/// Not really intended for human-facing SiK; as such generates
/// single-line, unformatted SiK for simplicity.
#[derive(Debug)]
pub struct SimpleFormatter<W: ?Sized> {
    ty: Option<&'static str>,
    field: Option<&'static str>,
    _sink: PhantomData<W>,
}

impl<W: ?Sized> Default for SimpleFormatter<W> {
    fn default() -> Self {
        Self {
            ty: None,
            field: Some("-"),
            _sink: PhantomData,
        }
    }
}

impl<W: ?Sized> SimpleFormatter<W> {
    pub fn new() -> Self {
        Self::default()
    }

    fn write_pre_value(&mut self, w: &mut W) -> io::Result<()>
    where
        W: io::Write,
    {
        if let Some(ty) = self.ty.take() {
            if is_valid_kdl_identifier(ty) {
                write!(w, "({})", ty)?;
            } else {
                let hash_count = count_needed_hashes(ty);
                write!(
                    w,
                    r#"(r{hashes}"{}"{hashes})"#,
                    ty,
                    hashes = &HASHES_LITERAL[..hash_count]
                )?;
            }
        }
        if let Some(field) = self.field.take() {
            if is_valid_kdl_identifier(field) {
                write!(w, "{} ", field)?;
            } else {
                let hash_count = count_needed_hashes(field);
                write!(
                    w,
                    r#"r{hashes}"{}"{hashes} "#,
                    field,
                    hashes = &HASHES_LITERAL[..hash_count]
                )?;
            }
        }
        Ok(())
    }
}

macro_rules! forward_write_to_display {
    ($($T:ident),* $(,)?) => {$(
        paste! {
            fn [<write_ $T:snake>](&mut self, w: &mut Self::Sink, v: $T) -> io::Result<()>
            {
                self.write_pre_value(w)?;
                write!(w, "{}", v)
            }
        }
    )*};
}

impl<W: ?Sized> Format for SimpleFormatter<W>
where
    W: io::Write,
{
    type Sink = W;

    fn provide_type_annotation(
        &mut self,
        _s: &mut Self::Sink,
        _ty: &'static str,
    ) -> io::Result<()> {
        // nop
        Ok(())
    }

    fn require_type_annotation(&mut self, _s: &mut Self::Sink, ty: &'static str) -> io::Result<()> {
        if self.ty.is_some() {
            panic!("Provided two mandatory type annotations (this is a bug in serde-kdl)");
        }
        self.ty = Some(ty);
        Ok(())
    }

    forward_write_to_display! {
        bool, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64,
    }

    fn write_unit(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.write_pre_value(s)?;
        write!(s, "null")
    }

    fn write_string(&mut self, s: &mut Self::Sink, v: &str) -> io::Result<()> {
        self.write_pre_value(s)?;
        let hash_count = count_needed_hashes(v);

        write!(
            s,
            r#"r{hashes}"{}"{hashes}"#,
            v,
            hashes = &HASHES_LITERAL[..hash_count]
        )
    }

    fn write_bytes(&mut self, mut s: &mut Self::Sink, v: &[u8]) -> io::Result<()> {
        self.write_pre_value(s)?;
        write!(s, r#"""#)?;
        {
            let mut w = base64::write::EncoderWriter::new(
                &mut s,
                base64::Config::new(base64::CharacterSet::Standard, true),
            );
            w.write_all(v)?;
            w.finish()?;
        }
        write!(s, r#"""#)
    }

    fn begin_group(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.write_pre_value(s)?;
        write!(s, "{{ ")
    }

    fn end_group(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        write!(s, "}}")
    }

    fn begin_field(&mut self, _s: &mut Self::Sink, name: Option<&'static str>) -> io::Result<()> {
        self.field = Some(name.unwrap_or("-"));
        Ok(())
    }

    fn end_field(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        write!(s, "; ")
    }

    fn begin_map(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.begin_group(s)
    }

    fn begin_map_key(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.begin_field(s, None)?;
        self.begin_group(s)?;
        self.begin_field(s, Some("key"))
    }

    fn end_map_key(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.end_field(s)
    }

    fn begin_map_value(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.begin_field(s, Some("value"))
    }

    fn end_map_value(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.end_field(s)?;
        self.end_group(s)?;
        self.end_field(s)
    }

    fn end_map(&mut self, s: &mut Self::Sink) -> io::Result<()> {
        self.end_group(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapFormat {
    Infer,
    Tuple,
    Struct,
}

#[derive(Debug)]
pub struct Options {
    pub option_as_enum: bool,
    pub newtype_as_tuple: bool,
    pub map_format: MapFormat,
    #[doc(hidden)]
    pub _non_exhaustive_but_pub: (),
}

impl Default for Options {
    fn default() -> Self {
        Self {
            option_as_enum: false,
            newtype_as_tuple: false,
            map_format: MapFormat::Infer,
            _non_exhaustive_but_pub: (),
        }
    }
}

/// Serde [`Serializer`](serde::Serializer) for KDL documents.
///
/// Note that this serializer may only be used once
#[derive(Debug)]
pub struct Serializer<'a, F: Format> {
    opt: Options,
    sink: &'a mut F::Sink,
    fmt: F,
}

impl<'a, F: Format> Serializer<'a, F> {
    pub fn new(sink: &'a mut F::Sink, fmt: F) -> Self {
        Self::new_with_options(sink, fmt, Options::default())
    }

    pub fn new_with_options(sink: &'a mut F::Sink, fmt: F, opt: Options) -> Self {
        Serializer { opt, sink, fmt }
    }
}

macro_rules! forward_ser_to_write {
    ($($T:ident),* $(,)?) => {$(
        paste! {
            fn [<serialize_ $T:snake>](self, v: $T) -> Result {
                Ok(self.fmt.[<write_ $T:snake>](self.sink, v)?)
            }
        }
    )*};
}

impl<'a, F: Format> serde::Serializer for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    forward_ser_to_write! {
        bool, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64,
    }

    fn serialize_char(self, v: char) -> Result {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, v: &str) -> Result {
        Ok(self.fmt.write_string(self.sink, v)?)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result {
        Ok(self.fmt.write_bytes(self.sink, v)?)
    }

    fn serialize_none(self) -> Result {
        if self.opt.option_as_enum {
            self.serialize_unit_variant("Option", 0, "None")
        } else {
            Ok(self.fmt.write_unit(self.sink)?)
        }
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result
    where
        T: Serialize,
    {
        if self.opt.option_as_enum {
            self.serialize_newtype_variant("Option", 1, "Some", value)
        } else {
            value.serialize(self)
        }
    }

    fn serialize_unit(self) -> Result {
        Ok(self.fmt.write_unit(self.sink)?)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result {
        self.fmt.provide_type_annotation(self.sink, name)?;
        Ok(self.fmt.write_unit(self.sink)?)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result {
        self.fmt.require_type_annotation(self.sink, variant)?;
        self.serialize_unit()
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result
    where
        T: Serialize,
    {
        if self.opt.newtype_as_tuple {
            let mut tuple = self.serialize_tuple_struct(name, 1)?;
            SerializeTupleStruct::serialize_field(&mut tuple, value)?;
            SerializeTupleStruct::end(tuple)
        } else {
            self.fmt.provide_type_annotation(self.sink, name)?;
            value.serialize(self)
        }
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result
    where
        T: Serialize,
    {
        if self.opt.newtype_as_tuple {
            let mut tuple = self.serialize_tuple_variant(name, variant_index, variant, 1)?;
            SerializeTupleVariant::serialize_field(&mut tuple, value)?;
            SerializeTupleVariant::end(tuple)
        } else {
            self.fmt.require_type_annotation(self.sink, variant)?;
            value.serialize(self)
        }
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.fmt.begin_group(self.sink)?;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.fmt.provide_type_annotation(self.sink, name)?;
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.fmt.require_type_annotation(self.sink, variant)?;
        self.serialize_tuple(len)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.fmt.begin_map(self.sink)?;
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.fmt.begin_group(self.sink)?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.fmt.require_type_annotation(self.sink, variant)?;
        self.serialize_struct(variant, len)
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

impl<'a, F: Format> SerializeSeq for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_field(&mut self.sink, None)?;
        value.serialize(&mut **self)?;
        self.fmt.end_field(&mut self.sink)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_group(self.sink)?;
        Ok(())
    }
}

impl<'a, F: Format> SerializeTuple for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_field(&mut self.sink, None)?;
        value.serialize(&mut **self)?;
        self.fmt.end_field(&mut self.sink)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_group(self.sink)?;
        Ok(())
    }
}

impl<'a, F: Format> SerializeTupleStruct for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_field(&mut self.sink, None)?;
        value.serialize(&mut **self)?;
        self.fmt.end_field(&mut self.sink)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_group(self.sink)?;
        Ok(())
    }
}

impl<'a, F: Format> SerializeTupleVariant for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_field(&mut self.sink, None)?;
        value.serialize(&mut **self)?;
        self.fmt.end_field(&mut self.sink)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_group(self.sink)?;
        Ok(())
    }
}

impl<'a, F: Format> SerializeMap for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_map_key(self.sink)?;
        key.serialize(&mut **self)?;
        self.fmt.end_map_key(self.sink)?;
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_map_value(self.sink)?;
        value.serialize(&mut **self)?;
        self.fmt.end_map_value(self.sink)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_map(self.sink)?;
        Ok(())
    }
}

impl<'a, F: Format> SerializeStruct for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_field(self.sink, Some(key))?;
        value.serialize(&mut **self)?;
        self.fmt.end_field(self.sink)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_group(self.sink)?;
        Ok(())
    }
}

impl<'a, F: Format> SerializeStructVariant for &'a mut Serializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.begin_field(self.sink, Some(key))?;
        value.serialize(&mut **self)?;
        self.fmt.end_field(self.sink)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_group(self.sink)?;
        Ok(())
    }
}

pub fn to_writer_ugly<W, T>(writer: &mut W, value: &T) -> Result
where
    W: ?Sized + io::Write,
    T: ?Sized + Serialize,
{
    let mut ser = Serializer::new(writer, SimpleFormatter::default());
    value.serialize(&mut ser)
}

pub fn to_vec_ugly<T>(value: &T) -> Result<Vec<u8>>
where
    T: ?Sized + Serialize,
{
    let mut writer = Vec::with_capacity(128);
    to_writer_ugly(&mut writer, value)?;
    Ok(writer)
}

pub fn to_string_ugly<T>(value: &T) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let bytes = to_vec_ugly(value)?;
    let string = unsafe { String::from_utf8_unchecked(bytes) };
    Ok(string)
}
