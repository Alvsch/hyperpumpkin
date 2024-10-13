use config::config_handler;
use flecs_ecs::core::EntityView;
use handshake::handshake_handler;
use login::login_handler;
use status::status_handler;

use crate::{components::{client::{ClientPacketQueue, CurrentState, PacketDecoder, PacketEncoder}, resources::{KeyPair, ServerConfig, ServerStorage}}, error::PacketIoError};

mod handshake;
mod status;
mod login;
mod config;
pub mod play;

pub fn packet_handler(
    e: EntityView,
    queue: &ClientPacketQueue,
    enc: &mut PacketEncoder,
    dec: &mut PacketDecoder,
    state: &mut CurrentState,
    config: &ServerConfig,
    storage: &ServerStorage,
    key_pair: &KeyPair,
) -> Result<(), PacketIoError> {
    for packet in queue.iter().cloned() {
        let _guard = tracing::trace_span!("handle_packet", id = packet.id.0, state = state.to_string()).entered();
        match state {
            CurrentState::HandShake => handshake_handler(packet, enc, state),
            CurrentState::Status => status_handler(packet, enc, config, storage),
            CurrentState::Login(_) => login_handler(packet, enc, dec, state, key_pair, &config.connection_mode, e).map_err(|err| err.into()),
            CurrentState::Config(_) => config_handler(e, packet, enc, state),
            _ => return Err(PacketIoError::BadPacket("not yet implemented")),
        }?;
    }
    Ok(())
}