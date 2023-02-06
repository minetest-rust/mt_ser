pub use enumset;

#[cfg(feature = "random")]
pub use generate_random;

#[cfg(feature = "random")]
pub use rand;

#[cfg(feature = "serde")]
pub use serde;

use enumset::{EnumSet, EnumSetType};
use mt_data_derive::mt_derive;
pub use mt_data_derive::{MtDeserialize, MtSerialize};
use std::{collections::HashMap, fmt, io};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "random")]
use generate_random::GenerateRandom;

#[derive(Error, Debug)]
pub enum SerializeError {
    #[error("{0}")]
    IoError(#[from] io::Error),
    #[error("serialization is not implemented")]
    Unimplemented,
}

#[derive(Error, Debug)]
pub enum DeserializeError {
    #[error("{0}")]
    IoError(#[from] io::Error),
    #[error("deserialization is not implemented")]
    Unimplemented,
}

pub trait MtSerialize: Sized {
    fn mt_serialize<W: io::Write>(&self, writer: &mut W) -> Result<(), SerializeError>;
}

pub trait MtDeserialize: Sized {
    fn mt_deserialize<R: io::Read>(reader: &mut R) -> Result<Self, DeserializeError>;
}

mod to_clt;
mod to_srv;

pub use to_clt::*;
pub use to_srv::*;
