use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct WorldModule;

impl Module for WorldModule {
    fn module(_world: &World) {
        
    }
}