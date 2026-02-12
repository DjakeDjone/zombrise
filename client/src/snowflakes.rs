use bevy::prelude::*;

use zombrise_shared::players::player::MainCamera;

#[derive(Resource, Clone)]
pub struct SnowfallConfig {
    pub count: u32,
    pub area_radius: f32,
    pub spawn_height: f32,
    pub despawn_height: f32,
    pub fall_speed: f32,
    pub drift_speed: f32,
    pub size_range: (f32, f32),
}

impl Default for SnowfallConfig {
    fn default() -> Self {
        Self {
            count: 100,
            area_radius: 35.0,
            spawn_height: 15.0,
            despawn_height: -1.0,
            fall_speed: 1.5,
            drift_speed: 0.8,
            size_range: (0.02, 0.06),
        }
    }
}

#[derive(Component)]
pub struct Snowflake {
    pub speed: f32,
    pub phase: f32,
    pub drift_amplitude: f32,
}

pub struct SnowfallPlugin;

impl Plugin for SnowfallPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnowfallConfig>()
            .add_systems(Startup, setup_snowfall)
            .add_systems(Update, animate_snowflakes);
    }
}


fn setup_snowfall(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<SnowfallConfig>,
) {
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));

    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.85),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    for _ in 0..config.count {
        spawn_snowflake(&mut commands, &config, &mesh, &material, Vec3::ZERO, true);
    }
}

fn spawn_snowflake(
    commands: &mut Commands,
    config: &SnowfallConfig,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    center: Vec3,
    random_height: bool,
) {
    let rng = || fastrand::f32();

    let angle = rng() * std::f32::consts::TAU;
    let distance = rng().sqrt() * config.area_radius;
    let x = center.x + angle.cos() * distance;
    let z = center.z + angle.sin() * distance;

    let y = if random_height {
        config.despawn_height + rng() * (config.spawn_height - config.despawn_height)
    } else {
        config.spawn_height + rng() * 2.0
    };

    let size = config.size_range.0 + rng() * (config.size_range.1 - config.size_range.0);

    let speed_variation = 0.7 + rng() * 0.6;
    let phase = rng() * std::f32::consts::TAU;
    let drift_amplitude = 0.5 + rng() * 1.0;

    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_xyz(x, y, z).with_scale(Vec3::splat(size)),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        Snowflake {
            speed: config.fall_speed * speed_variation,
            phase,
            drift_amplitude,
        },
        Name::new("Snowflake"),
    ));
}

/// Recycle a snowflake by teleporting it to a new random position near `center`.
fn recycle_snowflake(
    transform: &mut Transform,
    snowflake: &Snowflake,
    config: &SnowfallConfig,
    center: Vec3,
) {
    let rng = || fastrand::f32();
    let angle = rng() * std::f32::consts::TAU;
    let distance = rng().sqrt() * config.area_radius;
    transform.translation.x = center.x + angle.cos() * distance;
    transform.translation.z = center.z + angle.sin() * distance;
    transform.translation.y = config.spawn_height + rng() * 2.0;
    let _ = snowflake; // speed/phase/drift stay the same — visual variety is preserved
}

fn animate_snowflakes(
    time: Res<Time>,
    config: Res<SnowfallConfig>,
    camera_query: Query<&Transform, (With<MainCamera>, Without<Snowflake>)>,
    mut query: Query<(&mut Transform, &Snowflake)>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    // Use camera position as center; fallback to origin if no camera yet
    let center = camera_query
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (mut transform, snowflake) in query.iter_mut() {
        transform.translation.y -= snowflake.speed * dt;

        let drift_x =
            (elapsed * config.drift_speed + snowflake.phase).sin() * snowflake.drift_amplitude * dt;
        let drift_z = (elapsed * config.drift_speed * 0.7 + snowflake.phase * 1.3).cos()
            * snowflake.drift_amplitude
            * 0.6
            * dt;

        transform.translation.x += drift_x;
        transform.translation.z += drift_z;

        // Recycle snowflakes that have fallen below despawn height or drifted
        // too far from the camera, instead of despawning and respawning
        let too_low = transform.translation.y < config.despawn_height;
        let too_far = Vec2::new(
            transform.translation.x - center.x,
            transform.translation.z - center.z,
        )
        .length()
            > config.area_radius * 1.5;

        if too_low || too_far {
            recycle_snowflake(&mut transform, snowflake, &config, center);
        }
    }
}
