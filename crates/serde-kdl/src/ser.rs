use {
    crate::*,
    paste::paste,
    serde::ser::*,
    std::{fmt, io},
};

mod default;
mod human;

const HASHES_LITERAL: &str = unsafe { std::str::from_utf8_unchecked(&[b'#'; u8::MAX as _]) };
const INDENT_LITERAL: &str = unsafe {
    const INDENT_LITERAL_BYTES: [u8; 256] = {
        let mut arr = [b' '; u8::MAX as usize + 1];
        arr[0] = b'\n';
        arr
    };
    std::str::from_utf8_unchecked(&INDENT_LITERAL_BYTES)
};

fn count_needed_hashes(s: &str) -> Option<usize> {
    if !s.contains('"') {
        return None;
    }

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
    Some(outside_hash_count)
}

/// ```text
/// identifier := string | bare-identifier
/// bare-identifier := ( (identifier-char - digit - sign) identifier-char*
///                    | sign ((identifier-char - digit) identifier-char*)?
///                    ) - keyword
/// identifier-char := unicode - linespace - [\/(){}<>;[]=,"]
/// keyword := boolean | 'null'
///
/// digit := [0-9]
/// sign := '+' | '-'
///
/// boolean := 'true' | 'false'
/// linespace := newline | ws | single-line-comment
/// newline := See Table (All line-break white_space)
/// ws := bom | unicode-space | multi-line-comment
/// bom := '\u{FEFF}'
/// unicode-space := See Table (All White_Space unicode characters which are not `newline`)
///
/// single-line-comment := '//' ^newline+ (newline | eof)
/// multi-line-comment := '/*' commented-block
/// commented-block := '*/' | (multi-line-comment | '*' | '/' | [^*/]+) commented-block
/// ```
fn is_valid_kdl_identifier(s: &str) -> bool {
    // NB: contains_forbidden_char excludes comments as well, since it excludes `/`
    let contains_forbidden_char = s.chars().any(char::is_whitespace)
        || s.contains(
            &[
                '\\', '/', '(', ')', '{', '}', '<', '>', ';', '[', ']', '=', ',', '"', '\u{FEFF}',
            ][..],
        );
    let digit = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'][..];
    let confusable_number =
        s.starts_with(digit) || (s.starts_with(&['+', '-'][..]) && s[1..].starts_with(digit));
    let confusable_string = s.starts_with("r#"); // kdl-org/kdl#224
    let confusable_keyword = matches!(s, "null" | "true" | "false");

    !(contains_forbidden_char || confusable_number || confusable_string || confusable_keyword)
}

#[rustfmt::skip] // alignment helps a lot here
pub trait FormatSik {
    type Sink   : ?Sized;
    type Tuple  : FormatSikTuple    <Format = Self>;
    type Struct : FormatSikStruct   <Format = Self>;
    type Map    : FormatSikMap      <Format = Self>;

    // Label
    fn provide_type_annotation  (&mut self, s: &mut Self::Sink, ty: &'static str    ) -> io::Result<()>;
    fn require_type_annotation  (&mut self, s: &mut Self::Sink, ty: &'static str    ) -> io::Result<()>;

    // Primitive
    fn write_bool               (     self, s: &mut Self::Sink, v: bool             ) -> io::Result<()>;
    fn write_u8                 (     self, s: &mut Self::Sink, v: u8               ) -> io::Result<()>;
    fn write_u16                (     self, s: &mut Self::Sink, v: u16              ) -> io::Result<()>;
    fn write_u32                (     self, s: &mut Self::Sink, v: u32              ) -> io::Result<()>;
    fn write_u64                (     self, s: &mut Self::Sink, v: u64              ) -> io::Result<()>;
    fn write_u128               (     self, s: &mut Self::Sink, v: u128             ) -> io::Result<()>;
    fn write_i8                 (     self, s: &mut Self::Sink, v: i8               ) -> io::Result<()>;
    fn write_i16                (     self, s: &mut Self::Sink, v: i16              ) -> io::Result<()>;
    fn write_i32                (     self, s: &mut Self::Sink, v: i32              ) -> io::Result<()>;
    fn write_i64                (     self, s: &mut Self::Sink, v: i64              ) -> io::Result<()>;
    fn write_i128               (     self, s: &mut Self::Sink, v: i128             ) -> io::Result<()>;
    fn write_f32                (     self, s: &mut Self::Sink, v: f32              ) -> io::Result<()>;
    fn write_f64                (     self, s: &mut Self::Sink, v: f64              ) -> io::Result<()>;
    fn write_null               (     self, s: &mut Self::Sink                      ) -> io::Result<()>;
    fn write_string             (     self, s: &mut Self::Sink, v: &str             ) -> io::Result<()>;
    fn write_bytes              (     self, s: &mut Self::Sink, v: &[u8]            ) -> io::Result<()>;

    // Compound
    fn begin_tuple              (     self, s: &mut Self::Sink                      ) -> io::Result<Self::Tuple>;
    fn begin_struct             (     self, s: &mut Self::Sink                      ) -> io::Result<Self::Struct>;
    fn begin_map                (     self, s: &mut Self::Sink                      ) -> io::Result<Self::Map>;
}

pub trait FormatSikTuple {
    type Format: FormatSik;

    fn begin_element(
        &mut self,
        s: &mut <Self::Format as FormatSik>::Sink,
    ) -> io::Result<Self::Format>;
    fn end_element(&mut self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<()>;
    fn end_tuple(self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<()>;
}

pub trait FormatSikStruct {
    type Format: FormatSik;

    fn begin_field(
        &mut self,
        s: &mut <Self::Format as FormatSik>::Sink,
        name: &'static str,
    ) -> io::Result<Self::Format>;
    fn end_field(&mut self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<()>;
    fn end_struct(self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<()>;
}

pub trait FormatSikMap {
    type Format: FormatSik;

    fn begin_key(&mut self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<Self::Format>;
    fn end_key(&mut self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<()>;
    fn begin_value(
        &mut self,
        s: &mut <Self::Format as FormatSik>::Sink,
    ) -> io::Result<Self::Format>;
    fn end_value(&mut self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<()>;
    fn end_map(self, s: &mut <Self::Format as FormatSik>::Sink) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub map_is_struct: bool,
    pub unit_is_tuple: bool,
    pub option_is_enum: bool,
    pub force_root_node: bool,
    pub newtype_is_tuple: bool,
    #[doc(hidden)]
    pub _non_exhaustive_but_pub: (),
}

/// Serde [`Serializer`](serde::Serializer) for KDL documents.
#[derive(Debug)]
pub struct Serializer<'a, F: FormatSik> {
    opt: Options,
    snk: &'a mut F::Sink,
    fmt: F,
}

impl<'a, F: FormatSik> Serializer<'a, F> {
    pub fn new(snk: &'a mut F::Sink, fmt: F) -> Self {
        Self::new_with_options(snk, fmt, Options::default())
    }

    pub fn new_with_options(snk: &'a mut F::Sink, fmt: F, opt: Options) -> Self {
        Serializer { opt, snk, fmt }
    }
}

#[derive(Debug)]
pub struct TupleSerializer<'a, F: FormatSik> {
    opt: Options,
    snk: &'a mut F::Sink,
    fmt: F::Tuple,
}

#[derive(Debug)]
pub struct StructSerializer<'a, F: FormatSik> {
    opt: Options,
    snk: &'a mut F::Sink,
    fmt: F::Struct,
}

#[derive(Debug)]
enum MapMode<F: FormatSik> {
    Seq(F::Tuple, Option<F::Struct>),
    Map(F::Map),
}

pub struct MapSerializer<'a, F: FormatSik> {
    opt: Options,
    snk: &'a mut F::Sink,
    fmt: MapMode<F>,
}

impl<F: fmt::Debug + FormatSik> fmt::Debug for MapSerializer<'_, F>
where
    F::Sink: fmt::Debug,
    F::Tuple: fmt::Debug,
    F::Struct: fmt::Debug,
    F::Map: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapSerializer")
            .field("opt", &self.opt)
            .field("snk", &self.snk)
            .field("fmt", &self.fmt)
            .finish()
    }
}

macro_rules! forward_ser_to_write {
    ($($T:ident),* $(,)?) => {$(
        paste! {
            fn [<serialize_ $T:snake>](self, v: $T) -> Result {
                Ok(self.fmt.[<write_ $T:snake>](self.snk, v)?)
            }
        }
    )*};
}

impl<'a, F: FormatSik> serde::Serializer for Serializer<'a, F> {
    type Ok = ();
    type Error = crate::Error;
    type SerializeSeq = TupleSerializer<'a, F>;
    type SerializeTuple = TupleSerializer<'a, F>;
    type SerializeTupleStruct = TupleSerializer<'a, F>;
    type SerializeTupleVariant = TupleSerializer<'a, F>;
    type SerializeMap = MapSerializer<'a, F>;
    type SerializeStruct = StructSerializer<'a, F>;
    type SerializeStructVariant = StructSerializer<'a, F>;

    forward_ser_to_write! {
        bool, u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64,
    }

    fn serialize_char(self, v: char) -> Result {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_str(self, v: &str) -> Result {
        Ok(self.fmt.write_string(self.snk, v)?)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result {
        Ok(self.fmt.write_bytes(self.snk, v)?)
    }

    fn serialize_none(self) -> Result {
        if self.opt.option_is_enum {
            self.serialize_unit_variant("Option", 0, "None")
        } else {
            Ok(self.fmt.write_null(self.snk)?)
        }
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result
    where
        T: Serialize,
    {
        if self.opt.option_is_enum {
            self.serialize_newtype_variant("Option", 1, "Some", value)
        } else {
            value.serialize(self)
        }
    }

    fn serialize_unit(self) -> Result {
        if self.opt.unit_is_tuple {
            let tuple = self.serialize_tuple(0)?;
            SerializeTuple::end(tuple)
        } else {
            Ok(self.fmt.write_null(self.snk)?)
        }
    }

    fn serialize_unit_struct(mut self, name: &'static str) -> Result {
        self.fmt.provide_type_annotation(self.snk, name)?;
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result {
        self.fmt.require_type_annotation(self.snk, variant)?;
        self.serialize_unit()
    }

    fn serialize_newtype_struct<T: ?Sized>(mut self, name: &'static str, value: &T) -> Result
    where
        T: Serialize,
    {
        self.fmt.provide_type_annotation(self.snk, name)?;
        if self.opt.newtype_is_tuple {
            let mut tuple = self.serialize_tuple(1)?;
            SerializeTuple::serialize_element(&mut tuple, value)?;
            SerializeTuple::end(tuple)
        } else {
            value.serialize(self)
        }
    }

    fn serialize_newtype_variant<T: ?Sized>(
        mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result
    where
        T: Serialize,
    {
        self.fmt.require_type_annotation(self.snk, variant)?;
        self.serialize_newtype_struct(variant, value)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        let fmt = self.fmt.begin_tuple(self.snk)?;
        Ok(TupleSerializer {
            opt: self.opt,
            snk: self.snk,
            fmt,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        mut self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.fmt.provide_type_annotation(self.snk, name)?;
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.fmt.require_type_annotation(self.snk, variant)?;
        self.serialize_tuple(len)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        if self.opt.map_is_struct {
            let fmt = self.fmt.begin_tuple(self.snk)?;
            Ok(MapSerializer {
                opt: self.opt,
                snk: self.snk,
                fmt: MapMode::Seq(fmt, None),
            })
        } else {
            let fmt = self.fmt.begin_map(self.snk)?;
            Ok(MapSerializer {
                opt: self.opt,
                snk: self.snk,
                fmt: MapMode::Map(fmt),
            })
        }
    }

    fn serialize_struct(
        mut self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.fmt.provide_type_annotation(self.snk, name)?;
        let fmt = self.fmt.begin_struct(&mut self.snk)?;
        Ok(StructSerializer {
            opt: self.opt,
            snk: self.snk,
            fmt,
        })
    }

    fn serialize_struct_variant(
        mut self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.fmt.require_type_annotation(self.snk, variant)?;
        self.serialize_struct(variant, len)
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

impl<F: FormatSik> SerializeSeq for TupleSerializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        let fmt = self.fmt.begin_element(self.snk)?;
        value.serialize(Serializer {
            opt: self.opt,
            snk: self.snk,
            fmt,
        })?;
        self.fmt.end_element(&mut self.snk)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_tuple(self.snk)?;
        Ok(())
    }
}

impl<F: FormatSik> SerializeTuple for TupleSerializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result {
        SerializeSeq::end(self)
    }
}

impl<F: FormatSik> SerializeTupleStruct for TupleSerializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result {
        SerializeSeq::end(self)
    }
}

impl<F: FormatSik> SerializeTupleVariant for TupleSerializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result {
        SerializeSeq::end(self)
    }
}

impl<F: FormatSik> SerializeMap for MapSerializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result
    where
        T: Serialize,
    {
        match &mut self.fmt {
            MapMode::Seq(_, Some(_)) => {
                panic!(
                    "Didn't call `{}`",
                    stringify!(SerializeMap::serialize_value),
                )
            }
            MapMode::Seq(tuple, store @ None) => {
                let fmt = tuple.begin_element(self.snk)?;
                let mut map = fmt.begin_struct(self.snk)?;
                let fmt = map.begin_field(self.snk, "key")?;
                key.serialize(Serializer {
                    opt: self.opt,
                    snk: self.snk,
                    fmt,
                })?;
                map.end_field(self.snk)?;
                *store = Some(map);
            }
            MapMode::Map(map) => {
                let fmt = map.begin_key(self.snk)?;
                key.serialize(Serializer {
                    opt: self.opt,
                    snk: self.snk,
                    fmt,
                })?;
                map.end_key(self.snk)?;
            }
        }
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result
    where
        T: Serialize,
    {
        match &mut self.fmt {
            MapMode::Seq(_, None) => panic!(
                "Called `{}` without `{}`",
                stringify!(SerializeMap::serialize_value),
                stringify!(SerializeMap::serialize_key),
            ),
            MapMode::Seq(tuple, store @ Some(_)) => {
                let mut map = store.take().unwrap();
                let fmt = map.begin_field(self.snk, "value")?;
                value.serialize(Serializer {
                    opt: self.opt,
                    snk: self.snk,
                    fmt,
                })?;
                map.end_field(self.snk)?;
                map.end_struct(self.snk)?;
                tuple.end_element(self.snk)?;
            }
            MapMode::Map(map) => {
                let fmt = map.begin_value(self.snk)?;
                value.serialize(Serializer {
                    opt: self.opt,
                    snk: self.snk,
                    fmt,
                })?;
                map.end_value(self.snk)?;
            }
        }
        Ok(())
    }

    fn end(self) -> Result {
        match self.fmt {
            MapMode::Seq(_, Some(_)) => {
                panic!(
                    "Didn't call `{}`",
                    stringify!(SerializeMap::serialize_value),
                )
            }
            MapMode::Seq(tuple, None) => {
                tuple.end_tuple(self.snk)?;
            }
            MapMode::Map(map) => {
                map.end_map(self.snk)?;
            }
        }
        Ok(())
    }
}

impl<F: FormatSik> SerializeStruct for StructSerializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result
    where
        T: Serialize,
    {
        let fmt = self.fmt.begin_field(self.snk, key)?;
        value.serialize(Serializer {
            opt: self.opt,
            snk: self.snk,
            fmt,
        })?;
        self.fmt.end_field(self.snk)?;
        Ok(())
    }

    fn end(self) -> Result {
        self.fmt.end_struct(self.snk)?;
        Ok(())
    }
}

impl<'a, F: FormatSik> SerializeStructVariant for StructSerializer<'_, F> {
    type Ok = ();
    type Error = crate::Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result
    where
        T: Serialize,
    {
        SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result {
        SerializeStruct::end(self)
    }
}
