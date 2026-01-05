//! Player visual effects and spawning.

use bevy::prelude::*;

use zombrise_shared::entity2::Health;
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

/// Spawn player visuals when a player is added
pub fn spawn_player_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (Added<Player>, Without<PlayerVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in query.iter() {
        commands.entity(entity).insert((
            PlayerVisualsSpawned,
            LocalPlayerPosition(transform.translation),
            LocalPlayerRotation(transform.rotation),
            PlayerAttacking::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

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

/// Animate player damage flash effect
pub fn animate_player_damage(
    player_query: Query<
        (&DamageFlash, &PlayerOwner, &Children),
        (With<Player>, Changed<DamageFlash>),
    >,
    visual_mesh_query: Query<&MeshMaterial3d<StandardMaterial>, With<PlayerVisualMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    my_client_id: Res<MyClientId>,
) {
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
