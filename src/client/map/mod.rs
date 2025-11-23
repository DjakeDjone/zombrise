use bevy::prelude::*;

/// Readable settings bundle to tweak the generated snow landscape.
#[derive(Debug, Clone, Copy)]
pub struct SnowLandscapeConfig {
    /// Radius of the circular snow platform (in world units).
    pub radius: f32,
    /// Thickness of the packed snow disc.
    pub base_height: f32,
    /// Radius of the frozen pond feature.
    pub ice_radius: f32,
    /// Ambient brightness applied to the scene.
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

/// Spawns a stylized snow landscape: a circular plateau, gentle snow mounds,
/// a frozen pond, scattered boulders, evergreen trees, and ice shards.
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

    let packed_snow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.92, 0.96),
        perceptual_roughness: 0.65,
        metallic: 0.0,
        reflectance: 0.2,
        ..default()
    });

    let ice_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.85, 0.99),
        perceptual_roughness: 0.15,
        metallic: 0.02,
        reflectance: 0.95,
        ..default()
    });

    let bark_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.38, 0.28, 0.22),
        perceptual_roughness: 0.9,
        ..default()
    });

    let foliage_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.44, 0.64, 0.54),
        perceptual_roughness: 0.6,
        metallic: 0.02,
        reflectance: 0.3,
        ..default()
    });

    let stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.65, 0.7),
        perceptual_roughness: 0.95,
        metallic: 0.08,
        ..default()
    });

    spawn_plateau(commands, meshes, &snow_material, config, parent);
    spawn_snow_drifts(commands, meshes, &snow_material, config, parent);
    spawn_frozen_pond(commands, meshes, &ice_material, config, parent);
    spawn_trail(commands, meshes, &packed_snow_material, config, parent);
    spawn_boulders(commands, meshes, &stone_material, config, parent);
    spawn_trees(
        commands,
        meshes,
        &bark_material,
        &foliage_material,
        config,
        parent,
    );
    spawn_ice_shards(commands, meshes, &ice_material, config, parent);
}

fn apply_world_settings(commands: &mut Commands, config: SnowLandscapeConfig) {
    commands.insert_resource(ClearColor(Color::srgb(0.64, 0.74, 0.88)));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.95, 0.97, 1.0),
        brightness: config.ambient_brightness,
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
            PbrBundle {
                mesh: meshes.add(Cylinder::new(config.radius, config.base_height)),
                material: snow_material.clone(),
                transform: Transform::from_xyz(0.0, -config.base_height * 0.5, 0.0),
                ..default()
            },
            Name::new("Snow Plateau"),
        ))
        .set_parent(parent);
}

fn spawn_snow_drifts(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    snow_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    let drift_mesh = meshes.add(Sphere::new(1.0));
    let drifts = [
        (
            Vec3::new(config.radius * 0.45, 0.0, config.radius * 0.12),
            2.6,
            0.75,
        ),
        (
            Vec3::new(-config.radius * 0.38, 0.0, -config.radius * 0.24),
            2.2,
            0.6,
        ),
        (
            Vec3::new(-config.radius * 0.08, 0.0, config.radius * 0.38),
            1.8,
            0.4,
        ),
        (
            Vec3::new(config.radius * 0.18, 0.0, -config.radius * 0.42),
            2.3,
            0.55,
        ),
    ];

    for (position, scale, elevation) in drifts {
        let mut transform = Transform::from_translation(position + Vec3::Y * elevation);
        transform.scale = Vec3::splat(scale);
        commands
            .spawn((
                PbrBundle {
                    mesh: drift_mesh.clone(),
                    material: snow_material.clone(),
                    transform,
                    ..default()
                },
                Name::new("Snow Drift"),
            ))
            .set_parent(parent);
    }
}

fn spawn_frozen_pond(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    ice_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    let thickness = config.base_height * 0.45;
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Cylinder::new(config.ice_radius, thickness)),
                material: ice_material.clone(),
                transform: Transform::from_xyz(
                    -config.radius * 0.28,
                    -config.base_height * 0.6,
                    config.radius * 0.16,
                ),
                ..default()
            },
            Name::new("Frozen Pond"),
        ))
        .set_parent(parent);
}

fn spawn_trail(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    trail_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    let mut transform =
        Transform::from_xyz(config.radius * 0.05, -config.base_height * 0.5 + 0.025, 0.0);
    transform.rotation = Quat::from_rotation_y(0.3);

    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(
                    config.radius * 0.2,
                    config.base_height * 0.14,
                    config.radius * 1.05,
                )),
                material: trail_material.clone(),
                transform,
                ..default()
            },
            Name::new("Compacted Trail"),
        ))
        .set_parent(parent);
}

fn spawn_boulders(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    stone_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    let boulder_mesh = meshes.add(Sphere::new(0.9));
    let boulders = [
        (
            Vec3::new(config.radius * 0.5, 0.0, -config.radius * 0.2),
            Vec3::new(1.6, 0.8, 1.3),
        ),
        (
            Vec3::new(-config.radius * 0.47, 0.0, config.radius * 0.24),
            Vec3::new(1.3, 0.6, 1.0),
        ),
        (
            Vec3::new(config.radius * 0.18, 0.0, config.radius * 0.48),
            Vec3::new(1.1, 0.55, 0.9),
        ),
    ];

    for (position, scale) in boulders {
        let mut transform = Transform::from_translation(position + Vec3::Y * 0.18);
        transform.scale = scale;
        commands
            .spawn((
                PbrBundle {
                    mesh: boulder_mesh.clone(),
                    material: stone_material.clone(),
                    transform,
                    ..default()
                },
                Name::new("Frosted Boulder"),
            ))
            .set_parent(parent);
    }
}

fn spawn_trees(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    bark_material: &Handle<StandardMaterial>,
    foliage_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    let trunk_mesh = meshes.add(Cylinder::new(0.12, 1.9));
    let canopy_mesh = meshes.add(Sphere::new(0.9));

    let positions = [
        Vec3::new(config.radius * 0.34, 0.0, config.radius * 0.4),
        Vec3::new(-config.radius * 0.36, 0.0, -config.radius * 0.38),
        Vec3::new(-config.radius * 0.12, 0.0, -config.radius * 0.55),
        Vec3::new(config.radius * 0.55, 0.0, 0.22),
        Vec3::new(-config.radius * 0.5, 0.0, 0.15),
    ];

    for position in positions {
        let trunk_transform = Transform::from_translation(position + Vec3::new(0.0, 0.95, 0.0));

        commands
            .spawn((
                PbrBundle {
                    mesh: trunk_mesh.clone(),
                    material: bark_material.clone(),
                    transform: trunk_transform,
                    ..default()
                },
                Name::new("Evergreen Trunk"),
            ))
            .set_parent(parent)
            .with_children(|parent| {
                let mut lower_canopy = Transform::from_translation(Vec3::new(0.0, 1.05, 0.0));
                lower_canopy.scale = Vec3::new(1.6, 1.15, 1.6);

                parent.spawn((
                    PbrBundle {
                        mesh: canopy_mesh.clone(),
                        material: foliage_material.clone(),
                        transform: lower_canopy,
                        ..default()
                    },
                    Name::new("Evergreen Foliage (Lower)"),
                ));

                let mut upper_canopy = Transform::from_translation(Vec3::new(0.0, 1.7, 0.0));
                upper_canopy.scale = Vec3::new(1.0, 1.1, 1.0);

                parent.spawn((
                    PbrBundle {
                        mesh: canopy_mesh.clone(),
                        material: foliage_material.clone(),
                        transform: upper_canopy,
                        ..default()
                    },
                    Name::new("Evergreen Foliage (Upper)"),
                ));
            });
    }
}

fn spawn_ice_shards(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    ice_material: &Handle<StandardMaterial>,
    config: SnowLandscapeConfig,
    parent: Entity,
) {
    let shard_mesh = meshes.add(Cuboid::new(0.35, 1.8, 0.35));
    let shards = [
        (
            Vec3::new(-config.radius * 0.18, 0.0, config.radius * 0.14),
            0.35,
        ),
        (
            Vec3::new(-config.radius * 0.1, 0.0, config.radius * 0.18),
            -0.25,
        ),
        (
            Vec3::new(-config.radius * 0.05, 0.0, config.radius * 0.22),
            0.6,
        ),
    ];

    for (position, yaw) in shards {
        let mut transform = Transform::from_translation(position + Vec3::new(0.0, 0.9, 0.0));
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, -0.2, 0.0);
        transform.scale = Vec3::new(0.45, 1.0, 0.45);

        commands
            .spawn((
                PbrBundle {
                    mesh: shard_mesh.clone(),
                    material: ice_material.clone(),
                    transform,
                    ..default()
                },
                Name::new("Ice Shard"),
            ))
            .set_parent(parent);
    }
}
