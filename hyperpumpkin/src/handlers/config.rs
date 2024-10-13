use flecs_ecs::core::EntityView;
use pumpkin_protocol::{bytebuf::packet_id::Packet, client::config::{CFinishConfig, CRegistryData}, server::config::{SAcknowledgeFinishConfig, SClientInformationConfig, SKnownPacks, SPluginMessage}, RawPacket, ServerPacket};
use pumpkin_registry::Registry;

use crate::{components::{client::{ConfigState, CurrentState, PacketEncoder}, player::{ClientBrand, Play}}, error::PacketIoError, handlers::play::on_play};

pub fn config_handler(
    e: EntityView,
    mut packet: RawPacket,
    enc: &mut PacketEncoder,
    state: &mut CurrentState,
) -> Result<(), PacketIoError> {
    let CurrentState::Config(config) = state else { unreachable!(); };

    match packet.id.0 {
        SPluginMessage::PACKET_ID => {
            if *config == ConfigState::KnownPacks {
                // tracing::info!("Optional Brand packet received");
                let packet = SPluginMessage::read(&mut packet.bytebuf)?;
                if packet.channel == "minecraft:brand" {
                    let brand = String::from_utf8(packet.data)
                        .map_err(|_| PacketIoError::BadPacket("invalid utf-8"))?;

                    e.set(ClientBrand(brand));
                }
            } else {
                tracing::warn!("Out of order packet: Expected {:?}, received KnownPacks", config);
            }
        }
        SClientInformationConfig::PACKET_ID => {
            if *config == ConfigState::KnownPacks {
                // tracing::info!("Optional ClientInfo packet received");
                let _packet = SClientInformationConfig::read(&mut packet.bytebuf)?;
            } else {
                tracing::warn!("Out of order packet: Expected {:?}, received KnownPacks", config);
            }
        }
        SKnownPacks::PACKET_ID => {
            if *config == ConfigState::KnownPacks {
                let _packet = SKnownPacks::read(&mut packet.bytebuf)?;

                send_registry_data(enc)?;
                enc.append_packet(&CFinishConfig {})?;

                *config = ConfigState::AckFinish;
            } else {
                tracing::warn!("Out of order packet: Expected {:?}, received KnownPacks", config);
            }
        }
        SAcknowledgeFinishConfig::PACKET_ID => {
            // AckFinish is mandatory and must be received after KnownPacks
            if *config == ConfigState::AckFinish {
                tracing::info!("Config done");
                e.remove::<CurrentState>();
                e.add::<Play>();

                on_play(enc)?;
            } else {
                tracing::warn!("Out of order packet: Expected {:?}, received AckFinish", config);
            }
        }
        _ => {
            tracing::warn!("Unknown config packet: {:?}", packet.id);
        }
    }

    Ok(())
}

fn send_registry_data(enc: &mut PacketEncoder) -> Result<(), PacketIoError> {
    let registry = Registry::get_static();
    for entry in registry {
        enc.append_packet(&CRegistryData::new(&entry.registry_id, &entry.registry_entries))?;
    }

    Ok(())
}