use pumpkin_protocol::{bytebuf::packet_id::Packet, client::status::{CPingResponse, CStatusResponse}, server::status::{SStatusPingRequest, SStatusRequest}, PacketError, RawPacket, ServerPacket};
use serde_json::json;

use crate::{components::{client::PacketEncoder, resources::{ServerConfig, ServerStorage}}, error::PacketIoError};

pub fn status_handler(
    mut packet: RawPacket,
    enc: &mut PacketEncoder,
    config: &ServerConfig,
    storage: &ServerStorage
) -> Result<(), PacketIoError> {
    match packet.id.0 {
        SStatusRequest::PACKET_ID => {
            let value = json!({
                "version": {
                    "name": pumpkin_protocol::CURRENT_MC_VERSION,
                    "protocol": pumpkin_protocol::CURRENT_MC_PROTOCOL,
                },
                "players": {
                    "max": config.max_players,
                    "online": storage.online_players,
                    "sample": [],
                },
                "description": {
                    "text": config.description
                },
                "favicon": config.favicon,
                "enforcesSecureChat": false,
            }).to_string();

            let response = CStatusResponse::new(&value);
            enc.append_packet(&response)?;
        }
        SStatusPingRequest::PACKET_ID => {
            let ping = SStatusPingRequest::read(&mut packet.bytebuf)?;
            let response = CPingResponse::new(ping.payload);
            enc.append_packet(&response)?;
        }
        _ => return Err(PacketError::DecodeID.into()),
    };
    Ok(())
}