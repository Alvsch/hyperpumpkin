use pumpkin_core::GameMode;
use pumpkin_protocol::{bytebuf::packet_id::Packet, client::play::{CCenterChunk, CGameEvent, CLogin, CPlayerAbilities, CPlayerInfoUpdate, CSyncPlayerPosition, GameEvent}, server::play::{SPlayerPosition, SPlayerPositionRotation}, RawPacket};

use crate::{components::client::PacketEncoder, error::PacketIoError};

pub fn on_play(enc: &mut PacketEncoder) -> anyhow::Result<()> {
    enc.append_packet(&CLogin::new(
        0.into(),
        false,
        &["minecraft:overworld"],
        10.into(),
        16.into(),
        16.into(),
        false,
        false,
        false,
        0.into(),
        "minecraft:overworld",
        0.into(),
        GameMode::Creative as u8,
        GameMode::Creative as i8,
        false,
        false,
        None,
        0.into(),
        false
    ))?;

    enc.append_packet(&CPlayerAbilities::new(
        0x04,
        0.05,
        0.1
    ))?;

    enc.append_packet(&CSyncPlayerPosition::new(
        0.into(),
        64.into(),
        0.into(),
        0.0,
        0.0,
        0,
        0.into(),
    ))?;

    enc.append_packet(&CPlayerInfoUpdate::new(
        0x01 | 0x08,
        &[],
    ))?;

    enc.append_packet(&CGameEvent::new(GameEvent::StartWaitingChunks, 0.0))?;

    player_chunks(enc)?;

    Ok(())
}

fn player_chunks(enc: &mut PacketEncoder) -> anyhow::Result<()> {
    enc.append_packet(&CCenterChunk {
        chunk_x: 0.into(),
        chunk_z: 0.into(),
    })?;

    Ok(())
}

pub fn play_handler(packet: RawPacket) -> Result<(), PacketIoError> {
    match packet.id.0 {
        SPlayerPosition::PACKET_ID => {
            tracing::info!("got player position");
        },
        SPlayerPositionRotation::PACKET_ID => {
            tracing::info!("got player position and rotation");
        },
        _ => tracing::warn!("ignore unknown packet: {}", packet.id.0),
    }

    Ok(())
}