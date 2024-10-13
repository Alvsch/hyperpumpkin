use std::time::{Duration, Instant};

use flecs::OnAdd;
use flecs_ecs::prelude::*;
use pumpkin_protocol::{client::play::CKeepAlive, server::play::SKeepAlive, ServerPacket};

use crate::components::{client::{ClientPacketQueue, PacketEncoder}, player::Play};

#[derive(Debug, Component)]
struct KeepAliveState {
    got_keepalive: bool,
    last_keepalive_id: i64,
    last_send: Instant,
}

#[derive(Debug, Component)]
pub struct KeepAliveSettings {
    period: Duration,
}

impl Default for KeepAliveSettings {
    fn default() -> Self {
        Self {
            period: Duration::from_secs(8),
        }
    }
}

impl Default for KeepAliveState {
    fn default() -> Self {
        Self {
            got_keepalive: true,
            last_keepalive_id: 0,
            last_send: Instant::now(),
        }
    }
}

#[derive(Component)]
pub struct KeepAliveModule;

impl Module for KeepAliveModule {
    fn module(world: &World) {
        world.set(KeepAliveSettings::default());

        world.observer_named::<OnAdd, ()>("add_keepalive")
            .with::<Play>()
            .each_entity(|e, _| {
                e.set(KeepAliveState::default());
            });

        world.system_named::<(&mut PacketEncoder, &mut KeepAliveState, &KeepAliveSettings)>("send_keepalive")
            .multi_threaded()
            .term_at(2)
            .singleton()
            .each_entity(|e, (enc, state, settings)| {
                let now = Instant::now();

                if now.duration_since(state.last_send) >= settings.period {
                    if state.got_keepalive {
                        let id = rand::random();

                        state.got_keepalive = false;
                        state.last_keepalive_id = id;
                        state.last_send = now;

                        enc.append_packet(&CKeepAlive {
                            keep_alive_id: id,
                        }).unwrap();
                    } else {
                        let millis = settings.period.as_millis();
                        tracing::warn!("Client {e} timed out: no keepalive response after {millis}ms");
                        e.destruct();
                    }
                }

            });

        world.system_named::<(&ClientPacketQueue, &mut KeepAliveState)>("handle_keepalive")
            .multi_threaded()
            .each_entity(|e, (queue, keepalive)| {
                for mut packet in queue.iter().cloned() {
                    if let Ok(packet) = SKeepAlive::read(&mut packet.bytebuf) {
                        if keepalive.got_keepalive {
                            tracing::warn!("unexpected keepalive from client {e}");
                            // e.destruct();
                        } else if packet.keep_alive_id != keepalive.last_keepalive_id {
                            tracing::warn!(
                                "keepalive IDs don't match for client {e} (expected {}, got {})",
                                keepalive.last_keepalive_id, packet.keep_alive_id,
                            );
                            e.destruct();
                        } else {
                            keepalive.got_keepalive = true;
                            // ping.0 = state.last_send.elapsed().as_millis() as i32;
                        }
                    }
                }
            });

    }
}