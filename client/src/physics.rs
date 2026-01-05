//! Client-side physics setup for replicated entities.
//!
//! Since Collider and LockedAxes cannot be replicated (they don't impl PartialEq),
//! we add them on the client side when entities with physics components appear.

use avian3d::prelude::*;
use bevy::prelude::*;
use zombrise_shared::players::player::Player;
use zombrise_shared::shared::{MapMarker, TreeMarker};
use zombrise_shared::zombie::zombie::Zombie;

pub struct ClientPhysicsPlugin;

impl Plugin for ClientPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                add_player_physics,
                add_zombie_physics,
                add_map_physics,
                add_tree_physics,
            ),
        );
    }
}

/// Add physics components to player entities that have RigidBody but no Collider
fn add_player_physics(
    mut commands: Commands,
    query: Query<Entity, (With<Player>, With<RigidBody>, Without<Collider>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert((
            Collider::capsule(0.5, 1.0),
            LockedAxes::new().lock_rotation_x().lock_rotation_z(),
        ));
    }
}

/// Add physics components to zombie entities that have RigidBody but no Collider
fn add_zombie_physics(
    mut commands: Commands,
    query: Query<Entity, (With<Zombie>, With<RigidBody>, Without<Collider>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert((
            Collider::capsule(0.3, 1.0),
            LockedAxes::new().lock_rotation_x().lock_rotation_z(),
        ));
    }
}

/// Add physics components to map entities
fn add_map_physics(
    mut commands: Commands,
    query: Query<Entity, (With<MapMarker>, Without<Collider>)>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert((RigidBody::Static, Collider::cuboid(56.0, 0.1, 56.0)));
    }
}

/// Add physics components to tree entities
fn add_tree_physics(
    mut commands: Commands,
    query: Query<Entity, (With<TreeMarker>, Without<Collider>)>,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert((RigidBody::Static, Collider::cylinder(0.3, 2.0)));
    }
}
