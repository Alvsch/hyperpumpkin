use std::{io::Write, net::{Shutdown, SocketAddr, TcpListener}};

use flecs::OnRemove;
use flecs_ecs::prelude::*;
use sharded_slab::Slab;

use crate::{components::{client::{ClientConnection, ClientPacketQueue, PacketDecoder, PacketEncoder, SlabId}, resources::ServerListener}, error::PacketIoError, net::{client_data, listener_accept}};

#[derive(Component, Clone)]
pub struct NetworkSettings {
    pub address: SocketAddr
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            address: ([127, 0, 0, 1], 25565).into()
        }
    }
}

#[derive(Component)]
pub struct NetworkModule;

impl Module for NetworkModule {
    fn module(world: &World) {
        let network_receive = world
            .entity()
            .add::<flecs::pipeline::Phase>()
            .depends_on::<flecs::pipeline::PreUpdate>();

        let settings = world.get::<Option<&NetworkSettings>>(|settings| {
            settings
            .map_or(
                NetworkSettings::default(),
                |f| f.clone()
            )
        });
        
        let listener = TcpListener::bind(settings.address).expect("bind tcp listener");

        listener
            .set_nonblocking(true)
            .expect("failed to set nonblocking");
    
        world.set(ServerListener {
            listener,
            clients: Slab::new(),
        });

        // remove disconnected people
        world.observer::<OnRemove, (&mut ClientConnection, &SlabId, &ServerListener)>()
            .with::<ClientConnection>()
            .term_at(2).singleton()
            .each(| (stream, slab_id, storage)| {
                tracing::info!("Client [ID: {}] disconnected. Cleaning up resources", slab_id.0);
                let _ = stream.shutdown(Shutdown::Both);
                storage.clients.remove(slab_id.0);
            });

        // listener system
        world.system_named::<&ServerListener>("listener_accept")
            .term_at(0).singleton()
            .kind_id(network_receive)
            .each_iter(|it, _, server| {
                let world = it.world();

                let _guard = tracing::trace_span!("listener_accept").entered();
                match listener_accept(&world, server) {
                    Ok(_) => {},
                    Err(err) => {
                        tracing::error!("{}", err);
                    },
                };
            });
    
        // client incoming
        world.system_named::<(&mut ClientConnection, &mut PacketDecoder, &mut ClientPacketQueue)>("client_data")
            .multi_threaded()
            .kind_id(network_receive)
            .each_entity(|e, (stream, dec, queue)| {
                let _guard = tracing::trace_span!("client_data").entered(); 
                match client_data(stream, dec, queue) {
                    Ok(_) => {},
                    Err(err) => {
                        if !matches!(err, PacketIoError::Disconnect) {
                            tracing::warn!("Client data handling failed: {}.", err);
                        }
                        e.destruct();
                    },
                };
            });
    
        // Flush
        world.system_named::<(&mut ClientConnection, &mut PacketEncoder)>("flush")
            .multi_threaded()
            .kind::<flecs::pipeline::PostUpdate>()
            .each_entity(|e, (stream, enc)| {
                let _guard = tracing::trace_span!("flush").entered();
                let data = enc.take();
                match stream.write_all(&data) {
                    Ok(_) => {},
                    Err(err) => {
                        tracing::warn!("Failed to write data to client stream: {}.", err);
                        e.destruct();
                    },
                };
            });

    }
}