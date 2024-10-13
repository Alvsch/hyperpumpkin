use std::io::Read;

use anyhow::bail;
use flecs_ecs::prelude::*;

use crate::{components::{client::{ClientConnection, ClientPacketQueue, CurrentState, PacketDecoder, PacketEncoder, RemoteAddress, SlabId}, resources::ServerListener}, error::PacketIoError, interrupted, would_block};

pub fn listener_accept(world: &WorldRef, server: &ServerListener) -> anyhow::Result<()> {
    loop {
        let (stream, addr) = match server.listener.accept() {
            Ok(ok) => ok,
            Err(ref err) if would_block(err) => break,
            Err(ref err) if interrupted(err) => continue,
            Err(err) => return Err(err.into()),
        };

        if stream.set_nonblocking(true).is_err() { continue; }
        if stream.set_nodelay(true).is_err() { continue; }
        
        let Some(entry) = server.clients.vacant_entry() else {
            bail!("failed to get vacant entry");
        };
        
        let client = world.entity()
            .set(ClientConnection(stream))
            .set(RemoteAddress(addr.ip()))
            .set(PacketEncoder::default())
            .set(PacketDecoder::default())
            .set(ClientPacketQueue::default())
            .set(SlabId(entry.key()))
            .set(CurrentState::HandShake);

        entry.insert(client.id());
    }
    Ok(())
}

pub fn client_data(
    stream: &mut ClientConnection,
    dec: &mut PacketDecoder,
    queue: &mut ClientPacketQueue
) -> Result<(), PacketIoError> {
    queue.clear();

    let guard = tracing::trace_span!("reading_data").entered();
    loop {
        let mut buf = [0u8; 4096];
        let bytes_read = match stream.read(&mut buf) {
            Ok(0) => return Err(PacketIoError::Disconnect),
            Ok(n) => n,
            Err(ref err) if would_block(err) => break,
            Err(ref err) if interrupted(err) => continue,
            Err(err) => return Err(err.into()),
        };

        dec.queue_slice(&buf[..bytes_read]);
    }
    drop(guard);

    loop {
        let _guard = tracing::trace_span!("decoding_data").entered();
        let packet = match dec.decode() {
            Ok(packet) => match packet {
                Some(packet) => packet,
                None => break,
            },
            Err(err) => return Err(err.into()),
        };

        queue.push(packet);
    }
    Ok(())
}
