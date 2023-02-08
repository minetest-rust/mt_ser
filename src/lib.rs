pub use enumset;
pub use flate2;

#[cfg(feature = "random")]
pub use generate_random;

#[cfg(feature = "random")]
pub use rand;

#[cfg(feature = "serde")]
pub use serde;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use enumset::{EnumSet, EnumSetType, EnumSetTypeWithRepr};
use mt_data_derive::mt_derive;
pub use mt_data_derive::{MtDeserialize, MtSerialize};
use paste::paste;
use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    fmt,
    io::{self, Read, Write},
    num::TryFromIntError,
};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "random")]
use generate_random::GenerateRandom;

use crate as mt_data;

#[derive(Error, Debug)]
#[error("variable length")]
pub struct VarLen;

#[derive(Error, Debug)]
pub enum SerializeError {
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("collection too big: {0}")]
    TooBig(#[from] TryFromIntError),
    #[error("unimplemented")]
    Unimplemented,
}

impl From<Infallible> for DeserializeError {
    fn from(_err: Infallible) -> Self {
        unreachable!("infallible")
    }
}

#[derive(Error, Debug)]
pub enum DeserializeError {
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("variable length not supported")]
    NoVarlen(#[from] VarLen),
    #[error("collection too big: {0}")]
    TooBig(#[from] TryFromIntError),
    #[error("unimplemented")]
    Unimplemented,
}

impl From<Infallible> for SerializeError {
    fn from(_err: Infallible) -> Self {
        unreachable!("infallible")
    }
}

pub trait MtCfg:
    Sized
    + MtSerialize
    + MtDeserialize
    + TryFrom<usize, Error = Self::TryFromError>
    + TryInto<usize, Error = Self::TryIntoError>
{
    type TryFromError: Into<SerializeError>;
    type TryIntoError: Into<DeserializeError>;

    #[inline]
    fn utf16() -> bool {
        false
    }

    fn write_len(len: usize, writer: &mut impl Write) -> Result<(), SerializeError> {
        Ok(Self::try_from(len)
            .map_err(|e| e.into())?
            .mt_serialize::<DefaultCfg>(writer)?)
    }
}

pub type DefaultCfg = u16;

pub trait MtSerialize: Sized {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError>;
}

pub trait MtDeserialize: Sized {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError>;
}

impl MtCfg for u8 {
    type TryFromError = TryFromIntError;
    type TryIntoError = Infallible;
}

impl MtCfg for u16 {
    type TryFromError = TryFromIntError;
    type TryIntoError = Infallible;
}

impl MtCfg for u32 {
    type TryFromError = TryFromIntError;
    type TryIntoError = TryFromIntError;
}

impl MtCfg for u64 {
    type TryFromError = TryFromIntError;
    type TryIntoError = TryFromIntError;
}

pub struct NoLen;

impl MtSerialize for NoLen {
    fn mt_serialize<C: MtCfg>(&self, _writer: &mut impl Write) -> Result<(), SerializeError> {
        Ok(())
    }
}

impl MtDeserialize for NoLen {
    fn mt_deserialize<C: MtCfg>(_reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(Self)
    }
}

impl TryFrom<usize> for NoLen {
    type Error = Infallible;

    fn try_from(_x: usize) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl TryInto<usize> for NoLen {
    type Error = VarLen;

    fn try_into(self) -> Result<usize, Self::Error> {
        Err(VarLen)
    }
}

impl MtCfg for NoLen {
    type TryFromError = Infallible;
    type TryIntoError = VarLen;
}

pub struct Utf16<B: MtCfg>(pub B);

impl<B: MtCfg> MtSerialize for Utf16<B> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        self.0.mt_serialize::<DefaultCfg>(writer)
    }
}

impl<B: MtCfg> MtDeserialize for Utf16<B> {
    fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
        Ok(Self(B::mt_deserialize::<DefaultCfg>(reader)?))
    }
}

impl<B: MtCfg> TryFrom<usize> for Utf16<B> {
    type Error = B::TryFromError;

    fn try_from(x: usize) -> Result<Self, Self::Error> {
        Ok(Self(x.try_into()?))
    }
}

impl<B: MtCfg> TryInto<usize> for Utf16<B> {
    type Error = B::TryIntoError;

    fn try_into(self) -> Result<usize, Self::Error> {
        self.0.try_into()
    }
}

impl<B: MtCfg> MtCfg for Utf16<B> {
    type TryFromError = B::TryFromError;
    type TryIntoError = B::TryIntoError;

    #[inline]
    fn utf16() -> bool {
        true
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
                paste! {
                    writer.[<write_ $T>]::<BigEndian>(*self)?;
                }
                Ok(())
            }
        }

        impl MtDeserialize for $T {
            fn mt_deserialize<C: MtCfg>(reader: &mut impl Read) -> Result<Self, DeserializeError> {
                paste! {
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

impl MtSerialize for bool {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        (*self as u8).mt_serialize::<DefaultCfg>(writer)
    }
}

impl<T: MtSerialize, const N: usize> MtSerialize for [T; N] {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        for item in self.iter() {
            item.mt_serialize::<DefaultCfg>(writer)?;
        }

        Ok(())
    }
}

impl<T: MtSerialize, E: EnumSetTypeWithRepr<Repr = T>> MtSerialize for EnumSet<E> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        self.as_repr().mt_serialize::<DefaultCfg>(writer)
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

impl<T: MtSerialize> MtSerialize for Vec<T> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        C::write_len(self.len(), writer)?;
        for item in self.iter() {
            item.mt_serialize::<DefaultCfg>(writer)?;
        }
        Ok(())
    }
}

impl<T: MtSerialize> MtSerialize for HashSet<T> {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        C::write_len(self.len(), writer)?;
        for item in self.iter() {
            item.mt_serialize::<DefaultCfg>(writer)?;
        }
        Ok(())
    }
}

impl<K, V> MtSerialize for HashMap<K, V>
where
    K: MtSerialize + std::cmp::Eq + std::hash::Hash,
    V: MtSerialize,
{
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        C::write_len(self.len(), writer)?;
        for (key, value) in self.iter() {
            key.mt_serialize::<DefaultCfg>(writer)?;
            value.mt_serialize::<DefaultCfg>(writer)?;
        }
        Ok(())
    }
}

impl MtSerialize for String {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        if C::utf16() {
            // TODO
            Err(SerializeError::Unimplemented)
        } else {
            C::write_len(self.len(), writer)?;
            writer.write_all(self.as_bytes())?;

            Ok(())
        }
    }
}

mod to_clt;
mod to_srv;

pub use to_clt::*;
pub use to_srv::*;
