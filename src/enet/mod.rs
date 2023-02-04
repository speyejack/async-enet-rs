pub mod deserializer;
pub mod host;
pub mod peer;
pub mod protocol;
pub mod serializer;
pub mod sizer;

use serde::{de::Error as DeError, ser::Error as SerError};
use std::net::IpAddr;
use std::num::TryFromIntError;
use std::str::Utf8Error;

use thiserror::*;

pub type Result<T> = std::result::Result<T, ENetError>;
pub type ENetAddress = (IpAddr, u16);

#[derive(Error, Debug)]
pub enum ENetError {
    #[error("Socket error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encoding error: {0}")]
    Encoding(#[from] EncodingError),
}

#[derive(Error, Debug)]
pub enum EncodingError {
    #[error("Not enough data")]
    NotEnoughData,
    #[error("Invalid string data")]
    BadUtf8(#[from] Utf8Error),
    #[error("Invalid integer conversion")]
    IntConversion(#[from] TryFromIntError),
    #[error("Connection reset by peer")]
    ConnectionReset,
    #[error("Connection closed by peer")]
    ConnectionClose,
    #[error("Serde error")]
    CustomError,
}

impl SerError for EncodingError {
    fn custom<T>(_msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        EncodingError::CustomError
    }
}

impl DeError for EncodingError {
    fn custom<T>(_msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        EncodingError::CustomError
    }
}
