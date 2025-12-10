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

/// Spawns snow landscape
pub fn spawn_snow_landscape(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    apply_world_settings(commands, config);

    let snow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.97, 1.0),
        perceptual_roughness: 0.85,
        metallic: 0.03,
        reflectance: 0.55,
        ..default()
    });

    let ice_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.85, 0.99),
        perceptual_roughness: 0.15,
        metallic: 0.02,
        reflectance: 0.95,
        ..default()
    });

    spawn_plateau(commands, meshes, &snow_material, config, parent);

    spawn_frozen_pond(commands, meshes, &ice_material, config, parent);
}

fn apply_world_settings(commands: &mut Commands, config: SnowLandscapeConfig) {
    commands.insert_resource(ClearColor(Color::srgb(0.64, 0.74, 0.88)));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.95, 0.97, 1.0),
        brightness: config.ambient_brightness,
        affects_lightmapped_meshes: false,
    });
}

fn spawn_plateau(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    snow_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(config.radius, config.base_height))),
            MeshMaterial3d(snow_material.clone()),
            Transform::from_xyz(0.0, -config.base_height * 0.5, 0.0),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Name::new("Snow Plateau"),
        ))
        .insert(ChildOf(parent));
}

fn spawn_frozen_pond(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    ice_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    let thickness = config.base_height * 0.45;
    // Align pond top to plateau
    let pond_center_y = -thickness * 0.5;

    commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(config.ice_radius, thickness))),
            MeshMaterial3d(ice_material.clone()),
            Transform::from_xyz(
                -config.radius * 0.28,
                pond_center_y + 0.01,
                config.radius * 0.16,
            ),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Name::new("Frozen Pond"),
        ))
        .insert(ChildOf(parent));
}
