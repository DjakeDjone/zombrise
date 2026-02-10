//! World setup and management systems.

use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::{NetworkTarget, Replicate};
use rand::{Rng, SeedableRng}; // Add random number generator
use std::collections::HashSet;

use zombrise_shared::players::player::Player;
use zombrise_shared::shared::{Chunk, TreeMarker};
use zombrise_shared::zombie::zombie::Zombie;

use super::zombie_ai::ZOMBIE_BOUNDARY;

/// Chunk size in world units
pub const CHUNK_SIZE: f32 = 32.0;
/// Number of chunks to load around the player (radius)
pub const VIEW_DISTANCE: i32 = 3;

/// Fall threshold for removing entities
pub const FALL_THRESHOLD: f32 = -10.0;

/// Resource to track loaded chunks
#[derive(Resource, Default)]
pub struct ChunkManager {
    pub loaded_chunks: HashSet<(i32, i32)>,
}

/// Setup server world (resources)
pub fn setup_server(mut commands: Commands) {
    commands.init_resource::<ChunkManager>();
}

/// Update chunks based on player positions
pub fn update_chunks(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut chunk_manager: ResMut<ChunkManager>,
    chunk_query: Query<(Entity, &Chunk), Without<TreeMarker>>,
    tree_query: Query<(Entity, &Chunk), With<TreeMarker>>,
) {
    let mut required_chunks = HashSet::new();

    // Identify required chunks based on all players
    for transform in &player_query {
        let center_chunk_x = (transform.translation.x / CHUNK_SIZE).floor() as i32;
        let center_chunk_z = (transform.translation.z / CHUNK_SIZE).floor() as i32;

        for x in -VIEW_DISTANCE..=VIEW_DISTANCE {
            for z in -VIEW_DISTANCE..=VIEW_DISTANCE {
                required_chunks.insert((center_chunk_x + x, center_chunk_z + z));
            }
        }
    }

    // Default to spawning around 0,0 if no players exist yet (so the world isn't empty on start)
    if player_query.iter().count() == 0 {
        for x in -1..=1 {
            for z in -1..=1 {
                required_chunks.insert((x, z));
            }
        }
    }

    // Despawn chunks that are no longer needed
    for (entity, chunk) in &chunk_query {
        if !required_chunks.contains(&(chunk.x, chunk.z)) {
            commands.entity(entity).despawn();
            chunk_manager.loaded_chunks.remove(&(chunk.x, chunk.z));
        }
    }

    // Despawn tree entities whose chunk is no longer needed
    for (entity, chunk) in &tree_query {
        if !required_chunks.contains(&(chunk.x, chunk.z)) {
            commands.entity(entity).despawn();
        }
    }

    // Spawn missing chunks
    for (chunk_x, chunk_z) in required_chunks {
        if !chunk_manager.loaded_chunks.contains(&(chunk_x, chunk_z)) {
            spawn_chunk(&mut commands, chunk_x, chunk_z);
            chunk_manager.loaded_chunks.insert((chunk_x, chunk_z));
        }
    }
}

fn spawn_chunk(commands: &mut Commands, chunk_x: i32, chunk_z: i32) {
    let x_pos = chunk_x as f32 * CHUNK_SIZE;
    let z_pos = chunk_z as f32 * CHUNK_SIZE;

    // Spawn chunk ground collider
    // We position it at the center of the chunk
    let center_x = x_pos + CHUNK_SIZE / 2.0;
    let center_z = z_pos + CHUNK_SIZE / 2.0;

    commands.spawn((
        Chunk {
            x: chunk_x,
            z: chunk_z,
        },
        Replicate::to_clients(NetworkTarget::All),
        Transform::from_xyz(center_x, -0.05, center_z),
        GlobalTransform::default(),
        RigidBody::Static,
        Collider::cuboid(CHUNK_SIZE, 0.1, CHUNK_SIZE),
    ));

    // Spawn trees as independent root-level entities with world-space transforms.
    // This avoids hierarchy replication issues with Lightyear (which doesn't replicate
    // parent-child relationships), preventing trees from glitching/jumping positions.
    let seed = (chunk_x as u64).wrapping_mul(73856093) ^ (chunk_z as u64).wrapping_mul(19349663);
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Try to spawn a few trees per chunk
    if rng.random_bool(0.6) { // 60% chance to have trees
        let num_trees = rng.random_range(1..=3);
        for _ in 0..num_trees {
            // Random position within chunk (offset from center)
            let rx = rng.random_range(-CHUNK_SIZE/2.2 .. CHUNK_SIZE/2.2);
            let rz = rng.random_range(-CHUNK_SIZE/2.2 .. CHUNK_SIZE/2.2);

            commands.spawn((
                TreeMarker,
                // Tag with chunk coordinates so we can clean up when this chunk unloads
                Chunk { x: chunk_x, z: chunk_z },
                Replicate::to_clients(NetworkTarget::All),
                Transform::from_xyz(center_x + rx, 0.05, center_z + rz), // World-space
                GlobalTransform::default(),
                RigidBody::Static,
                Collider::cylinder(0.3, 2.0),
            ));
        }
    }
}

/// Remove entities that have fallen below the world
pub fn remove_fallen_entities(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
    zombie_query: Query<(Entity, &Transform), With<Zombie>>,
) {
    for (entity, transform) in &player_query {
        if transform.translation.y < FALL_THRESHOLD {
            commands.entity(entity).despawn();
        }
    }

    for (entity, transform) in &zombie_query {
        if transform.translation.y < FALL_THRESHOLD {
            commands.entity(entity).despawn();
        }
    }

}

/// Clean up zombies that have wandered too far from the map (prevents unbounded growth)
/// Updated for infinite world: clean up if far from ALL players
pub fn cleanup_wandering_zombies(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    zombie_query: Query<(Entity, &Transform), With<Zombie>>,
) {
    let players: Vec<Vec3> = player_query.iter().map(|t| t.translation).collect();
    
    // If no players, don't despawn yet (or despawn all? let's keep them briefly)
    if players.is_empty() {
        return;
    }

    for (entity, transform) in &zombie_query {
        let mut min_dist = f32::MAX;
        for player_pos in &players {
            let dist = transform.translation.distance(*player_pos);
            if dist < min_dist {
                min_dist = dist;
            }
        }

        // Despawn if too far from any player
        if min_dist > ZOMBIE_BOUNDARY {
            commands.entity(entity).despawn();
        }
    }
}
