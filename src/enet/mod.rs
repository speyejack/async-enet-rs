pub mod channel;
pub mod deserializer;
pub mod host;
pub mod peer;
pub mod protocol;
pub mod serializer;
pub mod sizer;

use serde::{de::Error as DeError, ser::Error as SerError};
use std::num::TryFromIntError;
use std::str::Utf8Error;

use thiserror::*;

use self::{
    channel::ChannelID,
    peer::{PeerID, PeerSendEvent},
};

pub type Result<T> = std::result::Result<T, ENetError>;

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("Channel send error")]
    PeerSendError(#[from] tokio::sync::mpsc::error::SendError<(PeerSendEvent, PeerID, ChannelID)>),

    #[error("Channel close")]
    PeerClosed,
}

#[derive(Error, Debug)]
pub enum ENetError {
    #[error("Socket error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Encoding error: {0}")]
    Encoding(#[from] EncodingError),

    #[error("Bad config: {0}")]
    BadConfig(String),

    #[error("Bad conversion: {0}")]
    IntConversion(#[from] TryFromIntError),

    #[error("Unexpected packet type")]
    UnexpectedPacketType,

    #[error("Invalid peer id: {0}")]
    InvalidPeerId(PeerID),

    #[error("Invalid channel id: {0}")]
    InvalidChannelId(ChannelID),

    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Error, Debug)]
pub enum EncodingError {
    #[error("Not enough data, {0} < {0}")]
    NotEnoughData(usize, usize),
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
