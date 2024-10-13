use lazy_static::lazy_static;
use pumpkin_protocol::{client::login::CLoginDisconnect, server::handshake::SHandShake, ConnectionState, RawPacket, ServerPacket};
use valence_text::Text;

use crate::{components::client::CurrentState, error::PacketIoError, PacketEncoder};

lazy_static! {
    pub static ref REASON: String = {
        let text = Text::text(format!("Outdated client! Please use {}", pumpkin_protocol::CURRENT_MC_VERSION));
        serde_json::to_string(&text).unwrap()
    };
}

pub fn handshake_handler(
    mut packet: RawPacket,
    enc: &mut PacketEncoder,
    state: &mut CurrentState
) -> Result<(), PacketIoError> {
    let handshake = SHandShake::read(&mut packet.bytebuf)?;
    if handshake.next_state == ConnectionState::Login && handshake.protocol_version != pumpkin_protocol::CURRENT_MC_PROTOCOL.into() {
        enc.append_packet(&CLoginDisconnect::new(&REASON))?;

        return Err(PacketIoError::Disconnect)
    }
    *state = handshake.next_state.into();
    Ok(())
}