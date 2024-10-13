use std::{fmt::Display, net::{IpAddr, TcpStream}};

use bytes::BytesMut;
use derive_more::derive::{Deref, DerefMut};
use flecs_ecs::prelude::*;
use pumpkin_config::compression::CompressionInfo;
use pumpkin_protocol::{ClientPacket, ConnectionState, PacketError, RawPacket};

#[derive(Default, Component)]
pub struct PacketEncoder(pumpkin_protocol::packet_encoder::PacketEncoder);

impl PacketEncoder {
    pub fn append_packet<P: ClientPacket>(&mut self, packet: &P) -> Result<(), PacketError> {
        tracing::trace!("Appending packet [ID: {}] ", P::PACKET_ID);
        return self.0.append_packet(packet);
    }

    pub fn set_encryption(&mut self, key: Option<&[u8; 16]>) {
        return self.0.set_encryption(key);
    }

    pub fn set_compression(&mut self, compression: Option<CompressionInfo>) {
        self.0.set_compression(compression);
    }

    pub fn take(&mut self) -> BytesMut {
        return self.0.take()
    }

}

#[derive(Default, Component, Deref, DerefMut)]
pub struct PacketDecoder(pumpkin_protocol::packet_decoder::PacketDecoder);

#[derive(Component, Default, Deref, DerefMut, Clone)]
pub struct ClientPacketQueue(Vec<RawPacket>);

#[derive(Debug, Component, Deref, DerefMut)]
pub struct SlabId(pub usize);

#[derive(Debug, Component, Deref, DerefMut)]
pub struct ClientConnection(pub TcpStream);

#[derive(Debug, Component)]
pub struct RemoteAddress(pub IpAddr);

#[derive(Debug, Clone)]
pub enum LoginState {
    LoginStart,
    EncryptionResponse {
        verify_token: [u8; 4],
        uuid: uuid::Uuid,
        username: String,
    },
    LoginAck,
    VelocityResponse {
        message_id: i32,
        username: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigState {
    KnownPacks,
    AckFinish,
}

#[derive(Debug, Component)]
pub enum CurrentState {
    HandShake,
    Status,
    Login(LoginState),
    Transfer,
    Config(ConfigState),
    Play,
}

impl Display for CurrentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            CurrentState::HandShake => "HandShake",
            CurrentState::Status => "Status",
            CurrentState::Login(_) => "Login",
            CurrentState::Transfer => "Transfer",
            CurrentState::Config(_) => "Config",
            CurrentState::Play => "Play",
        };
        write!(f, "{}", value)?;
        Ok(())
    }
}

impl From<ConnectionState> for CurrentState {
    fn from(value: ConnectionState) -> Self {
        match value {
            ConnectionState::HandShake => CurrentState::HandShake,
            ConnectionState::Status => CurrentState::Status,
            ConnectionState::Login => CurrentState::Login(LoginState::LoginStart),
            ConnectionState::Transfer => CurrentState::Transfer,
            ConnectionState::Config => CurrentState::Config(ConfigState::AckFinish),
            ConnectionState::Play => CurrentState::Play,
        }
    }
}
