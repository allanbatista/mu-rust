use bevy::prelude::MessageWriter;

use crate::world::{WorldId, WorldRequest};

pub fn request_world(world_requests: &mut MessageWriter<WorldRequest>, world: WorldId) {
    world_requests.write(WorldRequest(world));
}
