use bevy::prelude::*;

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
            count: 300,
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

#[derive(Resource)]
struct SnowflakeAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
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

    commands.insert_resource(SnowflakeAssets {
        mesh: mesh.clone(),
        material: material.clone(),
    });

    for _ in 0..config.count {
        spawn_snowflake(&mut commands, &config, &mesh, &material, true);
    }
}

fn spawn_snowflake(
    commands: &mut Commands,
    config: &SnowfallConfig,
    mesh: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    random_height: bool,
) {
    let rng = || fastrand::f32();

    let angle = rng() * std::f32::consts::TAU;
    let distance = rng().sqrt() * config.area_radius;
    let x = angle.cos() * distance;
    let z = angle.sin() * distance;

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

fn animate_snowflakes(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<SnowfallConfig>,
    assets: Option<Res<SnowflakeAssets>>,
    mut query: Query<(Entity, &mut Transform, &Snowflake)>,
) {
    let Some(assets) = assets else { return };
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (entity, mut transform, snowflake) in query.iter_mut() {
        transform.translation.y -= snowflake.speed * dt;

        let drift_x =
            (elapsed * config.drift_speed + snowflake.phase).sin() * snowflake.drift_amplitude * dt;
        let drift_z = (elapsed * config.drift_speed * 0.7 + snowflake.phase * 1.3).cos()
            * snowflake.drift_amplitude
            * 0.6
            * dt;

        transform.translation.x += drift_x;
        transform.translation.z += drift_z;

        if transform.translation.y < config.despawn_height {
            commands.entity(entity).despawn();
            spawn_snowflake(
                &mut commands,
                &config,
                &assets.mesh,
                &assets.material,
                false,
            );
        }
    }
}
