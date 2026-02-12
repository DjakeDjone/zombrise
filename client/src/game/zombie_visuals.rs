//! Zombie visual effects and spawning.

use bevy::camera::primitives::Aabb;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;

use zombrise_shared::zombie::zombie::{Zombie, ZombieAnimationState, ZombieLink};

/// Marker for zombie visual entities
#[derive(Component)]
pub struct ZombieVisual;

/// Marker for spawned zombie visuals
#[derive(Component)]
pub struct ZombieVisualsSpawned;

/// Marker for entities that have frustum culling already fixed
#[derive(Component)]
pub struct FrustumCullingFixed;


/// Spawn zombie visuals when a zombie is added
pub fn spawn_zombie_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (Added<Zombie>, Without<ZombieVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in query.iter() {
        // Mark the logic entity as having visuals to prevent duplicate processing
        commands.entity(entity).insert((
            ZombieVisualsSpawned,
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // Spawn a separate entity for the visual mesh to allow smooth interpolation
        // unrelated to the network snap updates on the main zombie entity.
        commands.spawn((
            SceneRoot(asset_server.load("zombie.glb#Scene0")),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // Start at the zombie's current position
            Transform::from_translation(transform.translation + Vec3::new(0.0, -0.75, 0.0))
                .with_rotation(transform.rotation * Quat::from_rotation_y(std::f32::consts::PI)),
            GlobalTransform::default(),
            ZombieVisual,
            ZombieLink(entity),
        ));
    }
}

/// Update zombie visual transforms (velocity-aware smooth interpolation)
///
/// This uses a combination of:
/// 1. Velocity-based extrapolation: Continue moving in the zombie's velocity direction
///    (only when animation state indicates the zombie should be moving)
/// 2. Position correction: Smoothly correct toward the actual server position
///
/// This creates much smoother movement than just chasing the position while
/// ensuring the visual doesn't slide when the animation shows standing still.
pub fn update_zombie_visuals_transform(
    mut visual_query: Query<(&mut Transform, &ZombieLink), With<ZombieVisual>>,
    zombie_query: Query<
        (
            &Transform,
            Option<&avian3d::prelude::LinearVelocity>,
            Option<&ZombieAnimationState>,
        ),
        (With<Zombie>, Without<ZombieVisual>),
    >,
    time: Res<Time>,
) {
    for (mut visual_transform, link) in visual_query.iter_mut() {
        if let Ok((target_transform, velocity, anim_state)) = zombie_query.get(link.0) {
            let target_translation = target_transform.translation + Vec3::new(0.0, -0.75, 0.0);
            let target_rotation =
                target_transform.rotation * Quat::from_rotation_y(std::f32::consts::PI);

            // Only use velocity extrapolation when the zombie's animation state
            // indicates it should be moving (Walking or Running).
            // This prevents the visual from sliding while the animation shows standing.
            let should_use_velocity = anim_state
                .map(|state| {
                    matches!(
                        state,
                        ZombieAnimationState::Walking | ZombieAnimationState::Running
                    )
                })
                .unwrap_or(false);

            let vel = if should_use_velocity {
                velocity
                    .map(|v| Vec3::new(v.x, 0.0, v.z))
                    .unwrap_or(Vec3::ZERO)
            } else {
                Vec3::ZERO
            };

            // First, continue moving in the velocity direction (extrapolation)
            // This prevents the visual from "lagging behind" during movement
            let extrapolated_pos = visual_transform.translation + vel * time.delta_secs();

            // Then blend between the extrapolated position and the target
            // Use a relatively high blend factor to correct errors quickly
            let correction_factor = (time.delta_secs() * 12.0).min(1.0);
            visual_transform.translation =
                extrapolated_pos.lerp(target_translation, correction_factor);

            // Smoothly interpolate rotation
            let rotation_factor = (time.delta_secs() * 15.0).min(1.0);
            visual_transform.rotation = visual_transform
                .rotation
                .slerp(target_rotation, rotation_factor);

            // Snap if too far (teleport) - this handles spawning and large corrections
            if visual_transform.translation.distance(target_translation) > 3.0 {
                visual_transform.translation = target_translation;
                visual_transform.rotation = target_rotation;
            }
        }
    }
}

/// Clean up orphaned zombie visuals when zombie entity is despawned
pub fn cleanup_orphaned_zombie_visuals(
    mut commands: Commands,
    visual_query: Query<(Entity, &ZombieLink), With<ZombieVisual>>,
    zombie_query: Query<Entity, With<Zombie>>,
    children_query: Query<&Children>,
) {
    for (entity, link) in visual_query.iter() {
        if !zombie_query.contains(link.0) {
            despawn_with_children_recursive(&mut commands, entity, &children_query);
        }
    }
}

fn despawn_with_children_recursive(
    commands: &mut Commands,
    entity: Entity,
    children_query: &Query<&Children>,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            despawn_with_children_recursive(commands, child, children_query);
        }
    }
    commands.entity(entity).despawn();
}

/// Fix zombie frustum culling issues
/// Uses Added filter and FrustumCullingFixed marker to avoid repeated processing
pub fn fix_zombie_frustum_culling(
    mut commands: Commands,
    skinned_mesh_query: Query<
        Entity,
        (
            Added<bevy_mesh::skinning::SkinnedMesh>,
            Without<FrustumCullingFixed>,
        ),
    >,
    parent_query: Query<&ChildOf>,
    zombie_query: Query<Entity, With<ZombieVisual>>,
) {
    for entity in skinned_mesh_query.iter() {
        // Check if this mesh belongs to a zombie
        let mut current = entity;
        let mut is_zombie = false;

        // Traverse up to find ZombieVisual component
        while let Ok(child_of) = parent_query.get(current) {
            current = child_of.get();
            if zombie_query.contains(current) {
                is_zombie = true;
                break;
            }
        }

        if is_zombie {
            // Expand AABB and mark as fixed to prevent re-processing
            commands.entity(entity).insert((
                Aabb {
                    center: Vec3::new(0.0, 1.0, 0.0).into(),
                    half_extents: Vec3::splat(5.0).into(),
                },
                FrustumCullingFixed,
            ));
        }
    }
}
