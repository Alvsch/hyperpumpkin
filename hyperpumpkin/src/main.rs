use base64::{engine::general_purpose, Engine};
use components::{client::{ClientConnection, ClientPacketQueue, CurrentState, PacketDecoder, PacketEncoder, RemoteAddress, SlabId}, player::{ClientBrand, GameMode, Play, PreviousGameMode, ProtocolId, Username, Uuid}, resources::{ConnectionMode, ExitSignal, KeyPair, ServerConfig, ServerStorage}};
use flecs_ecs::prelude::*;
use handlers::{packet_handler, play::play_handler};
use modules::{KeepAliveModule, NetworkModule};
use rsa::{pkcs8::EncodePublicKey, rand_core::OsRng, RsaPrivateKey};
use tracing::Level;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};
use std::{
    io,
    sync::atomic::Ordering,
};

mod net;
pub mod components;
mod error;
mod handlers;
pub mod modules;
pub mod world;

fn main() {
    let (chrome, _flush_guard) = tracing_chrome::ChromeLayerBuilder::new()
        .file("./trace-latest.json")
        .include_args(true)
        .build();

    tracing_subscriber::registry()
        .with(chrome)
        .with(tracing_subscriber::fmt::layer()
            .map_writer(|w| w.with_max_level(Level::DEBUG)))
        .init();    
    
    let world = World::new();
    let world = Box::new(world);
    let world = Box::leak(world);
    
    // spawn new players into the world

    world.import::<NetworkModule>();
    world.import::<KeepAliveModule>();

    world.component::<PacketEncoder>();
    world.component::<PacketDecoder>();
    world.component::<ClientPacketQueue>();
    world.component::<SlabId>();
    world.component::<ClientConnection>();
    world.component::<RemoteAddress>();
    world.component::<CurrentState>();

    world.component::<Play>();
    world.component::<Uuid>();
    world.component::<Username>();
    world.component::<ClientBrand>();
    world.component::<ProtocolId>();
    world.component::<GameMode>();
    world.component::<PreviousGameMode>();

    let signal = ExitSignal::default();

    world.set(signal.clone());


    ctrlc::set_handler(move || {
        signal.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    world.system_named::<&ExitSignal>("ctrlc_exit")
        .term_at(0).singleton()
        .each(|signal| {
            if signal.load(Ordering::SeqCst) {
                tracing::info!("Ctrl-C detected, quitting.");
                world.quit();
            }
        });

    tracing::debug!("generating RSA key");
    let private = RsaPrivateKey::new(&mut OsRng, 2048)
        .expect("generate private key");

    tracing::debug!("finished generating RSA key");

    let public_bytes = private.to_public_key().to_public_key_der().expect("public key to bytes");

    world.set(KeyPair {
        private,
        public_bytes
    });

    let bytes = include_bytes!("../../icon.png");
    let base64 = general_purpose::STANDARD.encode(bytes);

    world.set(ServerConfig {
        max_players: 10,
        description: "Hello, World!".to_string(),
        favicon: format!("data:image/png;base64,{}", base64),
        // connection_mode: ConnectionMode::Velocity { secret: std::sync::atomic::Arc::from(include_str!("../../forwarding.secret")) }
        connection_mode: ConnectionMode::Offline,
    });

    world.set(ServerStorage {
        connections: 0,
        online_players: 0,
    });

    world.system_named::<(
        &ClientPacketQueue,
        &mut PacketEncoder,
        &mut PacketDecoder,
        &mut CurrentState,
        &ServerConfig,
        &ServerStorage,
        &KeyPair
    )>("handle_packet")
        .multi_threaded()
        .term_at(4).singleton()
        .term_at(5).singleton()
        .term_at(6).singleton()
        .each_entity(|e, (
            queue,
            enc,
            dec,
            state,
            config,
            storage,
            key_pair
        )| {
            match packet_handler(e, queue, enc, dec, state, config, storage, key_pair) {
                Ok(_) => {},
                Err(err) => {
                    tracing::warn!("bad packet: {}", err);
                    e.destruct();
                },
            }
        });
    
    world.system_named::<&ClientPacketQueue>("play")
        .multi_threaded()
        .with::<Play>()
        .each_entity(|e, queue| {
            for packet in queue.iter().cloned() {
                match play_handler(packet) {
                    Ok(_) => {},
                    Err(err) => {
                        tracing::warn!("play error: {}", err);
                        e.destruct();
                    },
                }
            }
        });

    let mut last_frame_time_total = 0.0;
    world.system_named::<()>("global_stats")
        .each_iter(move |it, _, _| {
            let world = it.world();
            let info = world.info();

            let current_frame_time_total = info.frame_time_total;
            let ms_per_tick = (current_frame_time_total - last_frame_time_total) * 1000.0;
            last_frame_time_total = current_frame_time_total;

            let title = format!("{ms_per_tick:05.2} ms/tick");
            println!("{}", title);
        }).disable_self();

    let mut app = world.app();

    app.set_threads(4)
        .set_target_fps(20.0)
        .enable_stats(true);
        
    
    tracing::info!("Running app");
    app.run();
    tracing::info!("Exiting program");
}

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

pub fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}

/*
tracing::debug!("spawn player");
enc.append_packet(&CLogin::new(
    0,
    false,
    &["minecraft:overworld"],
    10.into(),
    32.into(),
    16.into(),
    false,
    false,
    false,
    0.into(),
    "minecraft:overworld",
    0,
    GameMode::Creative as u8,
    -1,
    false,
    false,
    None,
    0.into(),
    false
))?;
enc.append_packet(&CPluginMessage::new("minecraft:brand", b"valence"))?;
enc.append_packet(&CPlayerAbilities::new(0x01 | 0x04 | 0x02, 0.05, 0.1))?;
enc.append_packet(&CSyncPlayerPosition::new(0.0, 100.0, 0.0, 0.0, 0.0, 0x0, 0.into()))?;
*/
