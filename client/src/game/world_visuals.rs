//! World visual spawning (map and trees).

use bevy::prelude::*;

use crate::map::{create_map_assets, spawn_snow_landscape, MapAssets};
use zombrise_shared::shared::{MapMarker, TreeMarker};

/// Marker for spawned map visuals
#[derive(Component)]
pub struct MapVisualsSpawned;

/// Marker for spawned tree visuals
#[derive(Component)]
pub struct TreeVisualsSpawned;

/// Spawn map visuals when map marker is added
pub fn spawn_map_visuals(
    mut commands: Commands,
    query: Query<Entity, (Added<MapMarker>, Without<MapVisualsSpawned>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut map_assets_cache: Local<Option<MapAssets>>,
) {
    if map_assets_cache.is_none() {
        *map_assets_cache = Some(create_map_assets(
            &mut meshes,
            &mut materials,
            &asset_server,
        ));
    }

    let Some(map_assets) = map_assets_cache.as_ref() else {
        return;
    };

    for entity in query.iter() {
        commands.entity(entity).insert((
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapVisualsSpawned,
        ));

        spawn_snow_landscape(&mut commands, map_assets, entity);
    }
}

/// Spawn tree visuals when tree marker is added
pub fn spawn_tree_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (Added<TreeMarker>, Without<TreeVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in query.iter() {
        // Check if this is the giant corner tree (at radius * 0.9 ≈ 25.2)
        let is_giant_tree = transform.translation.x > 24.0 && transform.translation.z > 24.0;

        let scale_factor = if is_giant_tree {
            5.0 // Giant tree scale
        } else {
            // Use position as seed for deterministic random scale
            let seed = (transform.translation.x.abs() + transform.translation.z.abs()) * 1000.0;
            0.6 + (seed.sin().abs() * 0.8) // Range: 0.6 to 1.4
        };

        commands
            .entity(entity)
            .insert((
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                TreeVisualsSpawned,
            ))
            .with_children(|parent| {
                parent.spawn((
                    SceneRoot(asset_server.load("Pine Tree with Snow.glb#Scene0")),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    Transform::from_scale(Vec3::splat(scale_factor)),
                    GlobalTransform::default(),
                    Name::new("Pine Tree with Snow"),
                ));
            });
    }
}
