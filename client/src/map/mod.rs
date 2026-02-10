use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy_mesh::{Indices, VertexAttributeValues};

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct SnowLandscapeConfig {
    pub chunk_size: f32,
    pub base_height: f32,
    pub ambient_brightness: f32,
}

impl Default for SnowLandscapeConfig {
    fn default() -> Self {
        Self {
            chunk_size: 32.0,
            base_height: 0.4,
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

#[derive(Clone, Resource)]
pub struct MapAssets {
    pub snow_material: Handle<StandardMaterial>,
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

pub fn create_chunk_mesh(chunk_size: f32, chunk_x: i32, chunk_z: i32) -> Mesh {
    // scale1/scale2 for noise
    let noise = |x: f32, z: f32| -> f32 {
        let scale1 = 0.12;
        let scale2 = 0.25;
        let n1 = (x * scale1).sin() * (z * scale1).cos() * 0.15;
        let n2 = (x * scale2 + 1.7).cos() * (z * scale2 + 2.3).sin() * 0.08;
        n1 + n2
    };

    // Use a Plane
    let mut mesh = Plane3d::default()
        .mesh()
        .size(chunk_size, chunk_size)
        .subdivisions(16) // Lower subdivisions for performance, or keep high for quality
        .build();

    // specific mutable borrow to modify positions
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for pos in positions.iter_mut() {
            let local_x = pos[0];
            let local_z = pos[2];
            
            // Calculate global world space coordinates
            // Chunk center is at (chunk_x * size + size/2, chunk_z * size + size/2)
            // But Plane3d is centered at 0,0 locally.
            // When we spawn the chunk visual, we'll attach it to the parent entity which is at the chunk center.
            // So global_pos = transform.translation + local_pos
            // world_x = (chunk_x * size + size/2) + local_x
            
            let world_x = (chunk_x as f32 * chunk_size) + (chunk_size / 2.0) + local_x;
            let world_z = (chunk_z as f32 * chunk_size) + (chunk_size / 2.0) + local_z;
            
            // Apply height noise using world coordinates
            pos[1] = noise(world_x, world_z);
        }
    }

    // Recalculate normals to account for height changes
    mesh.duplicate_vertices(); 
    mesh.compute_flat_normals(); 
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

    let sky_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let sky_mesh = meshes.add(create_sky_dome_mesh(sky_config.radius, &sky_config));

    MapAssets {
        snow_material,
        sky_mesh,
        sky_material,
        config,
    }
}


pub fn apply_world_settings(commands: &mut Commands, map_assets: &MapAssets) {
    commands.insert_resource(ClearColor(Color::srgb(0.70, 0.68, 0.75)));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.95, 0.97, 1.0),
        brightness: map_assets.config.ambient_brightness,
        affects_lightmapped_meshes: false,
    });
}

