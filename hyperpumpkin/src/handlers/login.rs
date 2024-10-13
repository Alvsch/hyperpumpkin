use std::net::IpAddr;

use anyhow::{bail, ensure, Context};
use bytes::BytesMut;
use flecs_ecs::core::EntityView;
use hmac::{Hmac, Mac};
use pumpkin_config::compression::CompressionInfo;
use pumpkin_protocol::{bytebuf::ByteBuffer, client::{config::CKnownPacks, login::{CEncryptionRequest, CLoginPluginRequest, CLoginSuccess, CSetCompression}}, packet_decoder::PacketDecoder, server::login::{SEncryptionResponse, SLoginAcknowledged, SLoginPluginResponse, SLoginStart}, KnownPack, Property, RawPacket, ServerPacket, VarInt};
use rand::Rng;
use rsa::{pkcs8::Document, Pkcs1v15Encrypt};
use sha2::Sha256;

use crate::{components::{client::{ConfigState, CurrentState, LoginState, PacketEncoder, RemoteAddress}, player::Uuid, resources::{ConnectionMode, KeyPair}}, error::PacketIoError};

const VELOCITY_MIN_SUPPORTED_VERSION: u8 = 1;
const VELOCITY_MODERN_FORWARDING_WITH_KEY_V2: i32 = 3;

pub fn login_handler(
    mut packet: RawPacket,
    enc: &mut PacketEncoder,
    dec: &mut PacketDecoder,
    state: &mut CurrentState,
    key_pair: &KeyPair,
    mode: &ConnectionMode,
    e: EntityView,
) -> anyhow::Result<()> {
    let CurrentState::Login(login) = state else { unreachable!(); };

    match login {
        LoginState::LoginStart => {
            let packet = SLoginStart::read(&mut packet.bytebuf)?;
            
            match mode {
                ConnectionMode::Velocity { .. } => {
                    login_velocity(enc, packet.name, login)?;
                },
                ConnectionMode::Offline => {
                    login_offline(enc, packet.name, &key_pair.public_bytes, login)?;
                },
            }
        },
        LoginState::VelocityResponse { message_id, username } => {
            let packet = SLoginPluginResponse::read(&mut packet.bytebuf)?;
            
            ensure!(
                packet.message_id.0 == *message_id,
                "mismatched plugin response ID (got {}, expected {message_id})",
                packet.message_id.0,
            );
            
            let data = packet
                .data
                .context("missing plugin response data")?;
            
            ensure!(data.len() >= 32, "invalid plugin response data length");
            let (signature, data_without_signature) = data.split_at(32);
            
            let ConnectionMode::Velocity { secret } = mode else { bail!("invalid state"); };

            // Verify signature
            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
            Mac::update(&mut mac, data_without_signature);
            mac.verify_slice(&signature)?;

            let mut data_without_signature = ByteBuffer::new(BytesMut::from(data_without_signature));
            
            // Check Velocity version
            let version = data_without_signature.get_var_int()
                .context("failed to decode velocity version")?
                .0;
            
            // Get client address
            let remote_addr: IpAddr = data_without_signature.get_string()?.parse()?;
            
            // Get UUID
            let uuid = data_without_signature.get_uuid()?;
            
            // Get username and validate
            let name = data_without_signature.get_string()?;
            ensure!(
                username == &name,
                "mismatched usernames"
            );
        
            // Read game profile properties
            let _properties = data_without_signature.get_list(|data| {
                let name = data.get_string()?;
                let value = data.get_string()?;
                let signature = data.get_option(|data| {
                    data.get_string()
                })?;

                Ok(Property {
                    name,
                    value,
                    signature,
                })
            }).context("decoding velocity game profile properties")?;
        
            if version >= VELOCITY_MODERN_FORWARDING_WITH_KEY_V2 {
                // TODO
            }

            e.set(Uuid(uuid));
            e.set(RemoteAddress(remote_addr));

            setup_compression(256, enc, dec)?;

            enc.append_packet(&CLoginSuccess::new(
                &uuid,
                &username,
                &[],
                true,
            ))?;
            
            *login = LoginState::LoginAck;
        },
        LoginState::EncryptionResponse { verify_token, uuid, username  } => {
            let packet = SEncryptionResponse::read(&mut packet.bytebuf)?;

            let shared_secret = key_pair.private.decrypt(Pkcs1v15Encrypt, &packet.shared_secret)?;
            let client_verify_token = key_pair.private.decrypt(Pkcs1v15Encrypt, &packet.verify_token)?;

            if client_verify_token.as_slice() != verify_token {
                bail!(PacketIoError::BadPacket("invalid verify token"));
            }

            let Some(shared_secret) = to_bytes::<16>(&shared_secret) else {
                bail!(PacketIoError::BadPacket("invalid shared secret"));
            };

            enc.set_encryption(Some(shared_secret));
            dec.set_encryption(Some(shared_secret));

            setup_compression(256, enc, dec)?;

            enc.append_packet(&CLoginSuccess::new(
                &uuid,
                &username,
                &[],
                true,
            ))?;
            
            *login = LoginState::LoginAck;
        },
        LoginState::LoginAck => {
            let _ = SLoginAcknowledged::read(&mut packet.bytebuf)?;
            
            const PACK: KnownPack<'_> = KnownPack {
                namespace: "minecraft:core",
                id: "core",
                version: "1.21",
            };
            enc.append_packet(&CKnownPacks::new(&[PACK]))?;
            *state = CurrentState::Config(ConfigState::KnownPacks);
        },
    }
    Ok(())
}

fn setup_compression(threshold: i32, enc: &mut PacketEncoder, dec: &mut PacketDecoder) -> anyhow::Result<()> {
    enc.append_packet(&CSetCompression::new(VarInt(threshold)))?;

    if threshold >= 0 {
        enc.set_compression(Some(CompressionInfo {
            threshold: threshold as u32,
            level: 6,
        }));
        dec.set_compression(true);
    }

    Ok(())
}

fn login_offline(enc: &mut PacketEncoder, username: String, public_bytes: &Document, login: &mut LoginState) -> anyhow::Result<()> {
    let mut verify_token = [0u8; 4];
    rand::thread_rng().fill(&mut verify_token);
    
    enc.append_packet(&CEncryptionRequest::new(
        "",
        public_bytes.as_bytes(),
        &verify_token,
        false,
    ))?;

    *login = LoginState::EncryptionResponse {
        verify_token,
        uuid: offline_uuid(&username),
        username
    };
    Ok(())
}

fn login_velocity(enc: &mut PacketEncoder, username: String, login: &mut LoginState) -> anyhow::Result<()> {
    let message_id: i32 = 0; // TODO: make this random?
    enc.append_packet(&CLoginPluginRequest::new(
        VarInt(message_id),
        "velocity:player_info",
        &[VELOCITY_MIN_SUPPORTED_VERSION]
    ))?;

    *login = LoginState::VelocityResponse { message_id, username };
    Ok(())
}

fn offline_uuid(username: &str) -> uuid::Uuid {
    uuid::Uuid::from_slice(&md5::compute(username)[..16]).expect("failed to create offline uuid")
}

fn to_bytes<const N: usize>(slice: &[u8]) -> Option<&[u8; N]> {
    slice.try_into().ok()
}
