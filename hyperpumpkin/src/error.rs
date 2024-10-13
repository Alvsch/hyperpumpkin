use std::io;

use pumpkin_protocol::{bytebuf::DeserializerError, PacketError};
use thiserror::Error;


#[derive(Debug, Error)]
pub enum PacketIoError {
    #[error("packet: {0}")]
    Packet(#[from] PacketError),
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("deserializer: {0}")]
    Deserializer(#[from] DeserializerError),
    #[error("rsa: {0}")]
    Rsa(#[from] rsa::Error),
    #[error("{0}")]
    BadPacket(&'static str),
    #[error("disconnect")]
    Disconnect,
    #[error("anyhow: {0}")]
    Anyhow(#[from] anyhow::Error)
}