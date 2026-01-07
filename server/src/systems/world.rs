//! World setup and management systems.

use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::{NetworkTarget, Replicate};

use zombrise_shared::players::player::Player;
use zombrise_shared::shared::{MapMarker, TreeMarker};
use zombrise_shared::zombie::zombie::Zombie;

use super::zombie_ai::ZOMBIE_BOUNDARY;

/// Map radius constant
pub const MAP_RADIUS: f32 = 28.0;

/// Fall threshold for removing entities
pub const FALL_THRESHOLD: f32 = -10.0;

/// Setup server world (map and trees)
pub fn setup_server(mut commands: Commands) {
    let radius = MAP_RADIUS;

    // Spawn map with collision
    commands.spawn((
        MapMarker,
        Replicate::to_clients(NetworkTarget::All),
        Transform::from_xyz(0.0, -0.05, 0.0),
        GlobalTransform::default(),
        RigidBody::Static,
        Collider::cuboid(56.0, 0.1, 56.0),
    ));

    // Spawn trees
    let tree_positions = [
        Vec3::new(radius * 0.34, 0.0, radius * 0.4),
        Vec3::new(-radius * 0.36, 0.0, -radius * 0.38),
        Vec3::new(-radius * 0.12, 0.0, -radius * 0.55),
        Vec3::new(radius * 0.55, 0.0, 0.22),
        Vec3::new(radius * 0.7, 0.0, radius * 0.65),
        Vec3::new(-radius * 0.72, 0.0, radius * 0.58),
        Vec3::new(radius * 0.15, 0.0, -radius * 0.78),
        Vec3::new(-radius * 0.8, 0.0, -radius * 0.15),
        Vec3::new(radius * 0.82, 0.0, -radius * 0.45),
        Vec3::new(-radius * 0.25, 0.0, radius * 0.72),
        Vec3::new(radius * 0.48, 0.0, -radius * 0.68),
        Vec3::new(-radius * 0.62, 0.0, radius * 0.32),
        Vec3::new(radius * 0.22, 0.0, radius * 0.85),
        Vec3::new(-radius * 0.45, 0.0, -radius * 0.75),
        Vec3::new(radius * 0.75, 0.0, radius * 0.18),
        Vec3::new(radius * 0.38, 0.0, -radius * 0.22),
        Vec3::new(-radius * 0.85, 0.0, radius * 0.08),
        Vec3::new(radius * 0.05, 0.0, radius * 0.62),
    ];

    for position in tree_positions {
        commands.spawn((
            TreeMarker,
            Replicate::to_clients(NetworkTarget::All),
            Transform::from_translation(position),
            GlobalTransform::default(),
            RigidBody::Static,
            Collider::cylinder(0.3, 2.0),
        ));
    }

    // Giant tree
    commands.spawn((
        TreeMarker,
        Replicate::to_clients(NetworkTarget::All),
        Transform::from_translation(Vec3::new(radius * 0.9, 0.0, radius * 0.9)),
        GlobalTransform::default(),
        RigidBody::Static,
        Collider::cylinder(0.6, 4.0),
    ));
}

/// Update map size based on player count
pub fn update_map_size(
    player_query: Query<&Player>,
    mut map_query: Query<&mut Transform, With<MapMarker>>,
) {
    let player_count = player_query.iter().count();
    let scale = 1.0 + (player_count as f32 * 0.1).min(2.0);

    for mut transform in &mut map_query {
        transform.scale = Vec3::splat(scale);
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
pub fn cleanup_wandering_zombies(
    mut commands: Commands,
    zombie_query: Query<(Entity, &Transform), With<Zombie>>,
) {
    for (entity, transform) in &zombie_query {
        let distance_from_origin =
            Vec2::new(transform.translation.x, transform.translation.z).length();
        if distance_from_origin > ZOMBIE_BOUNDARY {
            commands.entity(entity).despawn();
        }
    }
}
