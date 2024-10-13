use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct Play;

#[derive(Component)]
pub struct Username(pub String);

#[derive(Component)]
pub struct Uuid(pub uuid::Uuid);

#[derive(Component)]
pub struct ClientBrand(pub String);

#[derive(Component)]
pub struct ProtocolId(pub u32);

#[derive(Component)]
pub struct GameMode(pub pumpkin_core::GameMode);

#[derive(Component)]
pub struct PreviousGameMode(pub pumpkin_core::GameMode);
