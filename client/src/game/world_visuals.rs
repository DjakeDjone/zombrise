//! World visual spawning (map and trees).

use avian3d::prelude::*;
use bevy::prelude::*;

use crate::map::{apply_world_settings, create_chunk_mesh, create_map_assets, MapAssets, SkyDome};
use zombrise_shared::shared::{Chunk, TreeMarker};

/// Marker for spawned map visuals
#[derive(Component)]
pub struct MapVisualsSpawned;

/// Marker for spawned tree visuals
#[derive(Component)]
pub struct TreeVisualsSpawned;

/// Initialize map assets and sky dome
pub fn init_world_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let map_assets = create_map_assets(&mut meshes, &mut materials, &asset_server);
    
    // Apply global settings (light, clear color)
    apply_world_settings(&mut commands, &map_assets);

    // Spawn Sky Dome
    commands.spawn((
        Mesh3d(map_assets.sky_mesh.clone()),
        MeshMaterial3d(map_assets.sky_material.clone()),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        SkyDome,
        Name::new("Sky Dome"),
    ));

    commands.insert_resource(map_assets);
}

/// Spawn visual meshes for new chunks
pub fn spawn_chunk_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Chunk), (Added<Chunk>, Without<MapVisualsSpawned>, Without<TreeMarker>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    map_assets: Res<MapAssets>,
) {
    for (entity, chunk) in query.iter() {
        // Create mesh for this specific chunk
        let mesh = create_chunk_mesh(map_assets.config.chunk_size, chunk.x, chunk.z);
        let mesh_handle = meshes.add(mesh);

        commands.entity(entity).insert((
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapVisualsSpawned,
            RigidBody::Static,
            Collider::cuboid(map_assets.config.chunk_size, 0.1, map_assets.config.chunk_size),
        )).with_children(|parent| {
            parent.spawn((
                Mesh3d(mesh_handle),
                MeshMaterial3d(map_assets.snow_material.clone()),
                // Transform is identity because vertex positions are already in world space relative 
                // to the chunk entity (which is positioned at center).
                // Wait, create_chunk_mesh generates positions relative to GLOBAL origin if we used world_x/z 
                // but Plane3d is local.
                // Let's re-check create_chunk_mesh implementation.
                // In create_chunk_mesh:
                // world_x = (chunk_x * size + size/2) + local_x
                // pos[1] = noise(world_x, world_z)
                // The X/Z positions in the mesh are still local (-size/2 to size/2). 
                // Only the Y (height) is based on world coordinates.
                // So adding it as a child to the Chunk entity (which is at world position) is correct.
                Transform::from_xyz(0.0, 0.0, 0.0), 
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Name::new(format!("Chunk Visual {},{}", chunk.x, chunk.z)),
            ));
        });
    }
}

/// Spawn tree visuals when tree marker is added
pub fn spawn_tree_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (Added<TreeMarker>, Without<TreeVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in query.iter() {
        // Simple scale variation based on position
        let seed = (transform.translation.x.abs() + transform.translation.z.abs()) * 1000.0;
        let scale_factor = 0.6 + (seed.sin().abs() * 0.8); // Range: 0.6 to 1.4

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
