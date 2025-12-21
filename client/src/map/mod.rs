use bevy::prelude::*;
use bevy_mesh::{Indices, PrimitiveTopology, VertexAttributeValues};

#[derive(Debug, Clone, Copy)]
pub struct SnowLandscapeConfig {
    pub radius: f32,
    pub base_height: f32,
    pub ice_radius: f32,
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

#[derive(Debug, Clone, Copy)]
pub struct SkyConfig {
    pub radius: f32,
    pub zenith_color: Color,
    pub horizon_color: Color,
    pub nadir_color: Color,
}

impl Default for SkyConfig {
    fn default() -> Self {
        Self {
            radius: 500.0,
            zenith_color: Color::srgb(0.45, 0.58, 0.82),
            horizon_color: Color::srgb(0.85, 0.75, 0.78),
            nadir_color: Color::srgb(0.70, 0.68, 0.75),
        }
    }
}

#[derive(Clone)]
pub struct MapAssets {
    pub snow_mesh: Handle<Mesh>,
    pub ice_mesh: Handle<Mesh>,
    pub snow_material: Handle<StandardMaterial>,
    pub ice_material: Handle<StandardMaterial>,
    pub sky_mesh: Handle<Mesh>,
    pub sky_material: Handle<StandardMaterial>,
    pub config: SnowLandscapeConfig,
    pub sky_config: SkyConfig,
}

#[derive(Component)]
pub struct SkyDome;

fn create_sky_dome_mesh(radius: f32, sky_config: &SkyConfig) -> Mesh {
    let mut mesh = Sphere::new(radius).mesh().uv(64, 32);

    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
    let positions: Vec<[f32; 3]> = match positions {
        VertexAttributeValues::Float32x3(v) => v.clone(),
        _ => panic!("Expected Float32x3 for positions"),
    };

    let mut colors: Vec<[f32; 4]> = Vec::with_capacity(positions.len());

    for pos in &positions {
        let y = pos[1];

        let normalized_y = (y / radius + 1.0) / 2.0;

        let color = if normalized_y < 0.5 {
            let t = normalized_y * 2.0;
            lerp_color(&sky_config.nadir_color, &sky_config.horizon_color, t)
        } else {
            let t = (normalized_y - 0.5) * 2.0;
            lerp_color(&sky_config.horizon_color, &sky_config.zenith_color, t)
        };

        let linear = color.to_linear();
        colors.push([linear.red, linear.green, linear.blue, 1.0]);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

    if let Some(VertexAttributeValues::Float32x3(ref mut normals)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for normal in normals.iter_mut() {
            normal[0] = -normal[0];
            normal[1] = -normal[1];
            normal[2] = -normal[2];
        }
    }

    if let Some(indices) = mesh.indices_mut() {
        let indices_vec: Vec<u32> = indices.iter().map(|i| i as u32).collect();
        let reversed: Vec<u32> = indices_vec
            .chunks(3)
            .flat_map(|tri| [tri[0], tri[2], tri[1]])
            .collect();
        mesh.insert_indices(Indices::U32(reversed));
    }

    mesh
}

fn lerp_color(a: &Color, b: &Color, t: f32) -> Color {
    let a_linear = a.to_linear();
    let b_linear = b.to_linear();
    let t = t.clamp(0.0, 1.0);

    Color::linear_rgb(
        a_linear.red + (b_linear.red - a_linear.red) * t,
        a_linear.green + (b_linear.green - a_linear.green) * t,
        a_linear.blue + (b_linear.blue - a_linear.blue) * t,
    )
}

fn create_snow_ground_mesh(radius: f32, segments: u32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        positions.push([x, 0.0, z]);
        normals.push([0.0, 1.0, 0.0]);

        uvs.push([x / radius * 0.5 + 0.5, z / radius * 0.5 + 0.5]);
    }

    for i in 0..segments {
        indices.push(0);
        indices.push(i + 1);
        indices.push(i + 2);
    }

    Mesh::new(PrimitiveTopology::TriangleList, Default::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

pub fn create_map_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> MapAssets {
    let config = SnowLandscapeConfig::default();
    let sky_config = SkyConfig::default();

    let snow_color: Handle<Image> = asset_server.load("Snow010A_1K-JPG/Snow010A_1K-JPG_Color.jpg");
    let snow_normal: Handle<Image> =
        asset_server.load("Snow010A_1K-JPG/Snow010A_1K-JPG_NormalGL.jpg");
    let snow_roughness: Handle<Image> =
        asset_server.load("Snow010A_1K-JPG/Snow010A_1K-JPG_Roughness.jpg");

    let snow_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(snow_color),
        normal_map_texture: Some(snow_normal),
        metallic_roughness_texture: Some(snow_roughness),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        reflectance: 0.5,

        uv_transform: bevy::math::Affine2::from_scale(bevy::math::Vec2::splat(8.0)),
        ..default()
    });

    let ice_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.68, 0.85, 0.99),
        perceptual_roughness: 0.15,
        metallic: 0.02,
        reflectance: 0.95,
        ..default()
    });

    let sky_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let sky_mesh = meshes.add(create_sky_dome_mesh(sky_config.radius, &sky_config));

    MapAssets {
        snow_mesh: meshes.add(create_snow_ground_mesh(config.radius, 64)),
        ice_mesh: meshes.add(Cylinder::new(config.ice_radius, config.base_height * 0.45)),
        snow_material,
        ice_material,
        sky_mesh,
        sky_material,
        config,
        sky_config,
    }
}

pub fn spawn_snow_landscape(commands: &mut Commands, map_assets: &MapAssets, parent: Entity) {
    apply_world_settings(commands, map_assets);

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

    commands
        .spawn((
            Mesh3d(map_assets.snow_mesh.clone()),
            MeshMaterial3d(map_assets.snow_material.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Name::new("Snow Plateau"),
        ))
        .insert(ChildOf(parent));

    let thickness = map_assets.config.base_height * 0.45;
    let pond_surface_y = -0.05;

    commands
        .spawn((
            Mesh3d(map_assets.ice_mesh.clone()),
            MeshMaterial3d(map_assets.ice_material.clone()),
            Transform::from_xyz(
                -map_assets.config.radius * 0.28,
                pond_surface_y - thickness * 0.5,
                map_assets.config.radius * 0.16,
            ),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Name::new("Frozen Pond"),
        ))
        .insert(ChildOf(parent));
}

fn apply_world_settings(commands: &mut Commands, map_assets: &MapAssets) {
    commands.insert_resource(ClearColor(Color::srgb(0.70, 0.68, 0.75)));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.95, 0.97, 1.0),
        brightness: map_assets.config.ambient_brightness,
        affects_lightmapped_meshes: false,
    });
}
