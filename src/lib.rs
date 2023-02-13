#![feature(array_try_from_fn)]
#![feature(associated_type_bounds)]
#![feature(iterator_try_collect)]

pub use flate2;
pub use mt_ser_derive::{mt_derive, MtDeserialize, MtSerialize};
pub use paste;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use enumset::{EnumSet, EnumSetTypeWithRepr};
use paste::paste as paste_macro;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt::Debug,
    io::{self, Read, Write},
    num::TryFromIntError,
    ops::Deref,
};
use thiserror::Error;

#[cfg(test)]
mod tests;

#[derive(Error, Debug)]
pub enum SerializeError {
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("collection too big: {0}")]
    TooBig(#[from] TryFromIntError),
}

impl From<Infallible> for SerializeError {
    fn from(_err: Infallible) -> Self {
        unreachable!("infallible")
    }
}

#[derive(Error, Debug)]
pub enum DeserializeError {
    #[error("io error: {0}")]
    IoError(io::Error),
    #[error("unexpected end of file")]
    UnexpectedEof,
    #[error("collection too big: {0}")]
    TooBig(#[from] TryFromIntError),
    #[error("invalid UTF-16: {0}")]
    InvalidUtf16(#[from] std::char::DecodeUtf16Error),
    #[error("invalid {0} enum variant {1}")]
    InvalidEnumVariant(&'static str, u64),
    #[error("invalid constant - wanted: {0:?} - got: {1:?}")]
    InvalidConst(Box<dyn Debug>, Box<dyn Debug>),
}

impl From<Infallible> for DeserializeError {
    fn from(_err: Infallible) -> Self {
        unreachable!("infallible")
    }
}

impl From<io::Error> for DeserializeError {
    fn from(err: io::Error) -> Self {
        if err.kind() == io::ErrorKind::UnexpectedEof {
            DeserializeError::UnexpectedEof
        } else {
            DeserializeError::IoError(err)
        }
    }
}

pub trait OrDefault<T> {
    fn or_default(self) -> Self;
}

pub struct WrapRead<'a, R: Read>(pub &'a mut R);
impl<'a, R: Read> Read for WrapRead<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }

    /*

    fn is_read_vectored(&self) -> bool {
        self.0.is_read_vectored()
    }

    */

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.0.read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.0.read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.0.read_exact(buf)
    }

    /*

    fn read_buf(&mut self, buf: io::BorrowedCursor<'_>) -> io::Result<()> {
        self.0.read_buf(buf)
    }

    fn read_buf_exact(&mut self, cursor: io::BorrowedCursor<'_>) -> io::Result<()> {
        self.0.read_buf_exact(cursor)
    }

    */
}

impl<T: MtDeserialize + Default> OrDefault<T> for Result<T, DeserializeError> {
    fn or_default(self) -> Self {
        match self {
            Err(DeserializeError::UnexpectedEof) => Ok(T::default()),
            x => x,
        }
    }
}

pub trait MtLen {
    fn option(&self) -> Option<usize>;

    type Range: Iterator<Item = usize> + 'static;
    fn range(&self) -> Self::Range;

    type Take<R: Read>: Read;
    fn take<R: Read>(&self, reader: R) -> Self::Take<R>;
}

pub trait MtCfg {
    type Len: MtLen;

    fn utf16() -> bool {
        false
    }

    fn write_len(len: usize, writer: &mut impl Write) -> Result<(), SerializeError>;
    fn read_len(reader: &mut impl Read) -> Result<Self::Len, DeserializeError>;
}

pub trait MtSerialize {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError>;
}

pub trait MtDeserialize: Sized {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError>;
}

impl MtLen for usize {
    fn option(&self) -> Option<usize> {
        Some(*self)
    }

    type Range = std::ops::Range<usize>;
    fn range(&self) -> Self::Range {
        0..*self
    }

    type Take<R: Read> = io::Take<R>;
    fn take<R: Read>(&self, reader: R) -> Self::Take<R> {
        reader.take(*self as u64)
    }
}

trait MtCfgLen:
    Sized
    + MtSerialize
    + MtDeserialize
    + TryFrom<usize, Error: Into<SerializeError>>
    + TryInto<usize, Error: Into<DeserializeError>>
{
}

impl<T: MtCfgLen> MtCfg for T {
    type Len = usize;

    fn write_len(len: usize, writer: &mut impl Write) -> Result<(), SerializeError> {
        Self::try_from(len)
            .map_err(Into::into)?
            .mt_serialize::<DefCfg>(writer)
    }

    fn read_len(reader: &mut impl Read) -> Result<Self::Len, DeserializeError> {
        Ok(Self::mt_deserialize::<DefCfg>(reader)?
            .try_into()
            .map_err(Into::into)?)
    }
}

impl MtCfgLen for u8 {}
impl MtCfgLen for u16 {}
impl MtCfgLen for u32 {}
impl MtCfgLen for u64 {}

pub type DefCfg = u16;

impl MtCfg for () {
    type Len = ();

    fn write_len(_len: usize, _writer: &mut impl Write) -> Result<(), SerializeError> {
        Ok(())
    }

    fn read_len(_writer: &mut impl Read) -> Result<Self::Len, DeserializeError> {
        Ok(())
    }
}

impl MtLen for () {
    fn option(&self) -> Option<usize> {
        None
    }

    type Range = std::ops::RangeFrom<usize>;
    fn range(&self) -> Self::Range {
        0..
    }

    type Take<R: Read> = R;
    fn take<R: Read>(&self, reader: R) -> Self::Take<R> {
        reader
    }
}

pub struct Utf16<B: MtCfg = DefCfg>(pub B);

impl<B: MtCfg> MtCfg for Utf16<B> {
    type Len = B::Len;

    fn utf16() -> bool {
        true
    }

    fn write_len(len: usize, writer: &mut impl Write) -> Result<(), SerializeError> {
        B::write_len(len, writer)
    }

    fn read_len(reader: &mut impl Read) -> Result<Self::Len, DeserializeError> {
        B::read_len(reader)
    }
}

impl MtSerialize for u8 {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        writer.write_u8(*self)?;
        Ok(())
    }
}

impl MtDeserialize for u8 {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(reader.read_u8()?)
    }
}

impl MtSerialize for i8 {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        writer.write_i8(*self)?;
        Ok(())
    }
}

impl MtDeserialize for i8 {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(reader.read_i8()?)
    }
}

macro_rules! impl_num {
    ($T:ty) => {
        impl MtSerialize for $T {
            fn mt_serialize<C: MtCfg>(
                &self,
                writer: &mut impl Write,
            ) -> Result<(), SerializeError> {
                paste_macro! {
                    writer.[<write_ $T>]::<BigEndian>(*self)?;
                }
                Ok(())
            }
        }

        impl MtDeserialize for $T {
            fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
                paste_macro! {
                    Ok(reader.[<read_ $T>]::<BigEndian>()?)
                }
            }
        }
    };
}

impl_num!(u16);
impl_num!(i16);

impl_num!(u32);
impl_num!(i32);
impl_num!(f32);

impl_num!(u64);
impl_num!(i64);
impl_num!(f64);

impl MtSerialize for () {
    fn mt_serialize<C: MtCfg>(&self, _writer: &mut impl Write) -> Result<(), SerializeError> {
        Ok(())
    }
}

impl MtDeserialize for () {
    fn mt_deserialize<C: MtCfg>(_reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(())
    }
}

impl MtSerialize for bool {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        (*self as u8).mt_serialize::<DefCfg>(writer)
    }
}

impl MtDeserialize for bool {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(u8::mt_deserialize::<DefCfg>(reader)? != 0)
    }
}

impl<T: MtSerialize> MtSerialize for &T {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        (*self).mt_serialize::<C>(writer)
    }
}

pub fn mt_serialize_seq<C: MtCfg, T: MtSerialize>(
    writer: &mut impl Write,
    iter: impl ExactSizeIterator + IntoIterator<Item = T>,
) -> Result<(), SerializeError> {
    C::write_len(iter.len(), writer)?;

    iter.into_iter()
        .try_for_each(|item| item.mt_serialize::<DefCfg>(writer))
}

pub fn mt_deserialize_seq<'a, C: MtCfg, T: MtDeserialize>(
    reader: &'a mut impl Read,
) -> Result<impl Iterator<Item = Result<T, DeserializeError>> + 'a, DeserializeError> {
    let len = C::read_len(reader)?;
    mt_deserialize_sized_seq(&len, reader)
}

pub fn mt_deserialize_sized_seq<'a, L: MtLen, T: MtDeserialize>(
    len: &L,
    reader: &'a mut impl Read,
) -> Result<impl Iterator<Item = Result<T, DeserializeError>> + 'a, DeserializeError> {
    let variable = len.option().is_none();

    Ok(len
        .range()
        .map_while(move |_| match T::mt_deserialize::<DefCfg>(reader) {
            Err(DeserializeError::UnexpectedEof) if variable => None,
            x => Some(x),
        }))
}

impl<T: MtSerialize, const N: usize> MtSerialize for [T; N] {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        mt_serialize_seq::<(), _>(writer, self.iter())
    }
}

impl<T: MtDeserialize, const N: usize> MtDeserialize for [T; N] {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        std::array::try_from_fn(|_| T::mt_deserialize::<DefCfg>(reader))
    }
}

impl<T: MtSerialize, E: EnumSetTypeWithRepr<Repr = T>> MtSerialize for EnumSet<E> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        self.as_repr().mt_serialize::<DefCfg>(writer)
    }
}

impl<T: MtDeserialize, E: EnumSetTypeWithRepr<Repr = T>> MtDeserialize for EnumSet<E> {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(Self::from_repr_truncated(T::mt_deserialize::<DefCfg>(
            reader,
        )?))
    }
}

impl<T: MtSerialize> MtSerialize for Option<T> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        match self {
            Some(item) => item.mt_serialize::<C>(writer),
            None => Ok(()),
        }
    }
}

impl<T: MtDeserialize> MtDeserialize for Option<T> {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        T::mt_deserialize::<C>(reader).map(Some).or_default()
    }
}

impl<T: MtSerialize> MtSerialize for Vec<T> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        mt_serialize_seq::<C, _>(writer, self.iter())
    }
}

impl<T: MtDeserialize> MtDeserialize for Vec<T> {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        mt_deserialize_seq::<C, _>(reader)?.try_collect()
    }
}

impl<T: MtSerialize + std::cmp::Eq + std::hash::Hash> MtSerialize for HashSet<T> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        mt_serialize_seq::<C, _>(writer, self.iter())
    }
}

impl<T: MtDeserialize + std::cmp::Eq + std::hash::Hash> MtDeserialize for HashSet<T> {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        mt_deserialize_seq::<C, _>(reader)?.try_collect()
    }
}

// TODO: support more tuples
impl<A: MtSerialize, B: MtSerialize> MtSerialize for (A, B) {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        self.0.mt_serialize::<DefCfg>(writer)?;
        self.1.mt_serialize::<DefCfg>(writer)?;

        Ok(())
    }
}

impl<A: MtDeserialize, B: MtDeserialize> MtDeserialize for (A, B) {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        let a = A::mt_deserialize::<DefCfg>(reader)?;
        let b = B::mt_deserialize::<DefCfg>(reader)?;

        Ok((a, b))
    }
}

impl<K, V> MtSerialize for HashMap<K, V>
where
    K: MtSerialize + std::cmp::Eq + std::hash::Hash,
    V: MtSerialize,
{
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        mt_serialize_seq::<C, _>(writer, self.iter())
    }
}

impl<K, V> MtDeserialize for HashMap<K, V>
where
    K: MtDeserialize + std::cmp::Eq + std::hash::Hash,
    V: MtDeserialize,
{
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        mt_deserialize_seq::<C, _>(reader)?.try_collect()
    }
}

impl MtSerialize for String {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        if C::utf16() {
            self.encode_utf16()
                .collect::<Vec<_>>() // FIXME: is this allocation necessary?
                .mt_serialize::<C>(writer)
        } else {
            mt_serialize_seq::<C, _>(writer, self.as_bytes().iter())
        }
    }
}

impl MtDeserialize for String {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        if C::utf16() {
            let mut err = None;

            let res =
                char::decode_utf16(mt_deserialize_seq::<C, _>(reader)?.map_while(|x| match x {
                    Ok(v) => Some(v),
                    Err(e) => {
                        err = Some(e);
                        None
                    }
                }))
                .try_collect();

            match err {
                None => Ok(res?),
                Some(e) => Err(e),
            }
        } else {
            let len = C::read_len(reader)?;

            // use capacity if available
            let mut st = match len.option() {
                Some(x) => String::with_capacity(x),
                None => String::new(),
            };

            len.take(WrapRead(reader)).read_to_string(&mut st)?;

            Ok(st)
        }
    }
}

impl<T: MtSerialize> MtSerialize for Box<T> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        self.deref().mt_serialize::<C>(writer)
    }
}

impl<T: MtDeserialize> MtDeserialize for Box<T> {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(Self::new(T::mt_deserialize::<C>(reader)?))
    }
}
