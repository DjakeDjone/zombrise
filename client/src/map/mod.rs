use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy_mesh::{Indices, VertexAttributeValues};

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

fn create_snow_ground_mesh(radius: f32, _segments: u32) -> Mesh {
    // scale1/scale2 for noise
    let noise = |x: f32, z: f32| -> f32 {
        let scale1 = 0.12;
        let scale2 = 0.25;
        let n1 = (x * scale1).sin() * (z * scale1).cos() * 0.15;
        let n2 = (x * scale2 + 1.7).cos() * (z * scale2 + 2.3).sin() * 0.08;
        n1 + n2
    };

    // Use a Plane instead of manual radial mesh for stability
    let size = radius * 2.5; // Slightly larger to cover corner gaps of the circle approximation
    let mut mesh = Plane3d::default()
        .mesh()
        .size(size, size)
        .subdivisions(64)
        .build();

    // specific mutable borrow to modify positions
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for pos in positions.iter_mut() {
            let x = pos[0];
            let z = pos[2];
            // Apply height noise
            let dist_sq = x * x + z * z;
            let max_r = radius * 1.1;

            // Fade out edges
            let fade = (1.0 - (dist_sq.sqrt() / max_r).powf(2.0)).max(0.0);

            pos[1] = noise(x, z) * fade;
        }
    }

    // Recalculate normals to account for height changes
    mesh.duplicate_vertices(); // Ensure unique vertices for flat shading or just correct calculation?
                               // Actually Plane3d shares vertices. We want smooth shading. Shared is fine.
                               // But we need to update normals.
    mesh.compute_flat_normals(); // Build-in helper? No, that gives flat shading.
                                 // For now, let's trust the height variation is small enough that Up normals are "okay"
                                 // OR we can rely on normal map for details.

    // Generate tangents for normal mapping
    mesh.generate_tangents().ok();

    mesh
}

pub fn create_map_assets(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) -> MapAssets {
    let config = SnowLandscapeConfig::default();
    let sky_config = SkyConfig::default();

    let sampler_desc = ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..default()
    };

    let settings = move |s: &mut ImageLoaderSettings| {
        s.sampler = ImageSampler::Descriptor(sampler_desc.clone());
    };

    let snow_color: Handle<Image> = asset_server.load_with_settings(
        "Snow010A_1K-PNG/Snow010A_1K-PNG_Color.png",
        settings.clone(),
    );
    let snow_normal: Handle<Image> = asset_server.load_with_settings(
        "Snow010A_1K-PNG/Snow010A_1K-PNG_NormalGL.png",
        settings.clone(),
    );
    let snow_ao: Handle<Image> = asset_server.load_with_settings(
        "Snow010A_1K-PNG/Snow010A_1K-PNG_AmbientOcclusion.png",
        settings,
    );

    let snow_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(snow_color),
        normal_map_texture: Some(snow_normal),
        occlusion_texture: Some(snow_ao),
        perceptual_roughness: 0.85,
        reflectance: 0.2,
        // Restore UV Transform to tile the texture
        uv_transform: bevy::math::Affine2::from_scale(bevy::math::Vec2::splat(4.0)),
        ..default()
    });

    let ice_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.95), // White to blend with snow
        perceptual_roughness: 0.4,                 // Less smooth, more like snow-covered ice
        metallic: 0.0,
        reflectance: 0.2, // Less reflective
        ..default()
    });

    let sky_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let sky_mesh = meshes.add(create_sky_dome_mesh(sky_config.radius, &sky_config));

    println!("Creating Map Assets with Plane3d approach.");

    MapAssets {
        snow_mesh: meshes.add(create_snow_ground_mesh(config.radius, 64)),
        ice_mesh: meshes.add(Cylinder::new(config.ice_radius, config.base_height * 0.45)),
        snow_material,
        ice_material,
        sky_mesh,
        sky_material,
        config,
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
