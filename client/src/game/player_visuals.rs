//! Player visual effects and spawning.

use bevy::prelude::*;

use zombrise_shared::players::player::{
    DamageFlash, LocalPlayerPosition, LocalPlayerRotation, MyClientId, Player, PlayerOwner,
};
use zombrise_shared::players::player_animation::PlayerAttacking;

/// Marker for spawned player visuals
#[derive(Component)]
pub struct PlayerVisualsSpawned;

/// Marker for player visual mesh
#[derive(Component)]
pub struct PlayerVisualMesh;

/// Stores the smoothed visual transform for other players (not local player)
/// This enables interpolation to hide network update snapping
#[derive(Component)]
pub struct OtherPlayerVisuals {
    pub visual_translation: Vec3,
    pub visual_rotation: Quat,
}

/// Spawn player visuals when a player is added
pub fn spawn_player_visuals(
    mut commands: Commands,
    query: Query<
        (Entity, &Transform, &PlayerOwner),
        (Added<Player>, Without<PlayerVisualsSpawned>),
    >,
    asset_server: Res<AssetServer>,
    my_client_id: Option<Res<MyClientId>>,
) {
    let Some(my_client_id) = my_client_id else {
        return;
    };
    for (entity, transform, owner) in query.iter() {
        let mut entity_commands = commands.entity(entity);

        entity_commands.insert((
            PlayerVisualsSpawned,
            LocalPlayerPosition(transform.translation),
            LocalPlayerRotation(transform.rotation),
            PlayerAttacking::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // For other players (not local), add visual interpolation component
        if owner.0 != my_client_id.0 {
            entity_commands.insert(OtherPlayerVisuals {
                visual_translation: transform.translation,
                visual_rotation: transform.rotation,
            });
        }

        // Offset model
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                SceneRoot(asset_server.load("player.glb#Scene0")),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Transform::from_translation(Vec3::new(0.0, -1.1, 0.0))
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                GlobalTransform::default(),
                PlayerVisualMesh,
            ));
        });
    }
}

/// Update other players' visual transforms using velocity-based interpolation
/// This prevents "jumping" between server updates for non-local players
pub fn update_other_player_visuals(
    mut player_query: Query<
        (
            &mut Transform,
            &mut OtherPlayerVisuals,
            Option<&avian3d::prelude::LinearVelocity>,
        ),
        With<Player>,
    >,
    time: Res<Time>,
) {
    for (mut transform, mut visuals, velocity) in player_query.iter_mut() {
        // Get the server-replicated target position
        let target_translation = transform.translation;
        let target_rotation = transform.rotation;

        // Get the player's velocity for extrapolation
        let vel = velocity
            .map(|v| Vec3::new(v.x, 0.0, v.z))
            .unwrap_or(Vec3::ZERO);

        // First, continue moving in the velocity direction (extrapolation)
        let extrapolated_pos = visuals.visual_translation + vel * time.delta_secs();

        // Then blend between the extrapolated position and the actual target
        let correction_factor = (time.delta_secs() * 12.0).min(1.0);
        visuals.visual_translation = extrapolated_pos.lerp(target_translation, correction_factor);

        // Smoothly interpolate rotation
        let rotation_factor = (time.delta_secs() * 15.0).min(1.0);
        visuals.visual_rotation = visuals
            .visual_rotation
            .slerp(target_rotation, rotation_factor);

        // Snap if too far (teleport)
        if visuals.visual_translation.distance(target_translation) > 3.0 {
            visuals.visual_translation = target_translation;
            visuals.visual_rotation = target_rotation;
        }

        // Apply the smoothed visual transform to the actual transform
        // This overrides the server-replicated position with our smoothed version
        transform.translation = visuals.visual_translation;
        transform.rotation = visuals.visual_rotation;
    }
}

/// Animate player damage flash effect
pub fn animate_player_damage(
    player_query: Query<
        (&DamageFlash, &PlayerOwner, &Children),
        (With<Player>, Changed<DamageFlash>),
    >,
    visual_mesh_query: Query<&MeshMaterial3d<StandardMaterial>, With<PlayerVisualMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    my_client_id: Option<Res<MyClientId>>,
) {
    let Some(my_client_id) = my_client_id else {
        return;
    };
    for (damage_flash, owner, children) in player_query.iter() {
        if owner.0 == my_client_id.0 {
            // Find the visual mesh child
            for child in children.iter() {
                if let Ok(material_handle) = visual_mesh_query.get(child) {
                    if let Some(material) = materials.get_mut(material_handle) {
                        if damage_flash.timer > 0.0 {
                            // Flash red
                            let flash_intensity = (damage_flash.timer / 0.3).clamp(0.0, 1.0);
                            material.base_color = Color::srgb(
                                0.8 + 0.2 * flash_intensity,
                                0.7 - 0.5 * flash_intensity,
                                0.6 - 0.4 * flash_intensity,
                            );
                        } else {
                            material.base_color = Color::srgb(0.8, 0.7, 0.6);
                        }
                    }
                }
            }
        }
    }
}
