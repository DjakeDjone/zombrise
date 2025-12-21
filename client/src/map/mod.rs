use bevy::prelude::*;

/// Snow landscape settings
#[derive(Debug, Clone, Copy)]
pub struct SnowLandscapeConfig {
    /// Radius of snow platform
    pub radius: f32,
    /// Thickness of snow disc
    pub base_height: f32,
    /// Radius of frozen pond
    pub ice_radius: f32,
    /// Ambient brightness
    pub ambient_brightness: f32,
}

impl Default for SnowLandscapeConfig {
    fn default() -> Self {
        Self {
            radius: 28.0,
            base_height: 0.4,
            ice_radius: 9.0,
            ambient_brightness: 380.0,
        }
    }
}

/// Cached map assets
#[derive(Clone)]
pub struct MapAssets {
    pub snow_mesh: Handle<Mesh>,
    pub ice_mesh: Handle<Mesh>,
    pub snow_material: Handle<StandardMaterial>,
    pub ice_material: Handle<StandardMaterial>,
    pub config: SnowLandscapeConfig,
}

/// Spawns snow landscape
pub fn spawn_snow_landscape(commands: &mut Commands, map_assets: &MapAssets, parent: Entity) {
    apply_world_settings(commands, map_assets.config);

    // Plateau
    commands
        .spawn((
            Mesh3d(map_assets.snow_mesh.clone()),
            MeshMaterial3d(map_assets.snow_material.clone()),
            Transform::from_xyz(0.0, -map_assets.config.base_height * 0.5, 0.0),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Name::new("Snow Plateau"),
        ))
        .insert(ChildOf(parent));

    // Frozen Pond
    let thickness = map_assets.config.base_height * 0.45;
    let pond_center_y = -thickness * 0.5;

    commands
        .spawn((
            Mesh3d(map_assets.ice_mesh.clone()),
            MeshMaterial3d(map_assets.ice_material.clone()),
            Transform::from_xyz(
                -map_assets.config.radius * 0.28,
                pond_center_y + 0.01,
                map_assets.config.radius * 0.16,
            ),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Name::new("Frozen Pond"),
        ))
        .insert(ChildOf(parent));
}

fn apply_world_settings(commands: &mut Commands, config: SnowLandscapeConfig) {
    commands.insert_resource(ClearColor(Color::srgb(0.64, 0.74, 0.88)));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.95, 0.97, 1.0),
        brightness: config.ambient_brightness,
        affects_lightmapped_meshes: false,
    });
}
