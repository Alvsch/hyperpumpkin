use std::{net::TcpListener, sync::{atomic::AtomicBool, Arc}};

use derive_more::derive::Deref;
use flecs_ecs::prelude::*;
use rsa::{pkcs8::Document, RsaPrivateKey};
use sharded_slab::Slab;


#[derive(Component)]
pub struct ServerListener {
    pub listener: TcpListener,
    pub clients: Slab<Entity>,
}

#[derive(Component)]
pub struct KeyPair {
    pub private: RsaPrivateKey,
    pub public_bytes: Document,
}

#[derive(Component)]
pub struct ServerStorage {
    pub connections: usize,
    pub online_players: usize,
}

#[derive(Debug, Default, Clone)]
pub enum ConnectionMode {
    #[default]
    Offline,
    Velocity {
        secret: Arc<str>,
    },
}

#[derive(Component)]
pub struct ServerConfig {
    pub max_players: usize,
    pub description: String,
    pub favicon: String,
    pub connection_mode: ConnectionMode,
}

#[derive(Component, Clone, Deref)]
pub struct ExitSignal(Arc<AtomicBool>);

impl Default for ExitSignal {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}