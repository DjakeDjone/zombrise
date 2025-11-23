use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt, RepliconRenetPlugins,
    renet::{
        ConnectionConfig, RenetClient,
        transport::{ClientAuthentication, NetcodeClientTransport},
    },
};
use dragon_queen::players::player::{
    CameraRotation, MainCamera, Player, PlayerOwner, handle_input,
};
use dragon_queen::shared::{MapMarker, SharedPlugin, TreeMarker};
use dragon_queen::zombie::zombie::Zombie;
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

mod map;
use map::{SnowLandscapeConfig, spawn_snow_landscape};

#[derive(Resource)]
struct MyClientId(u64);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)
        .add_plugins(SharedPlugin)
        .insert_resource(CameraRotation {
            yaw: 0.0,
            pitch: -0.3,
        })
        .add_systems(Startup, (setup, setup_client, lock_cursor))
        .add_systems(
            Update,
            (
                handle_input,
                handle_camera_rotation,
                camera_follow,
                spawn_player_visuals,
                spawn_map_visuals,
                spawn_zombie_visuals,
                spawn_tree_visuals,
            ),
        )
        .run();
}

fn setup_client(mut commands: Commands, network_channels: Res<RepliconChannels>) {
    let server_channels_config = network_channels.get_server_configs();
    let client_channels_config = network_channels.get_client_configs();

    let client = RenetClient::new(ConnectionConfig {
        server_channels_config,
        client_channels_config,
        ..Default::default()
    });

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;
    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: 0,
        server_addr,
        user_data: None,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

    commands.insert_resource(client);
    commands.insert_resource(transport);
    commands.insert_resource(MyClientId(client_id));
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Additional directional light for better illumination
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.5, -0.5, 0.0)),
        ..default()
    });
}

fn spawn_map_visuals(
    mut commands: Commands,
    query: Query<Entity, Added<MapMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(SpatialBundle::default());
        // Spawn landscape without trees (trees come from server)
        spawn_snow_landscape(
            &mut commands,
            &mut meshes,
            &mut materials,
            SnowLandscapeConfig::default(),
            entity,
        );
    }
}

fn spawn_tree_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), Added<TreeMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

    let trunk_mesh = meshes.add(Cylinder::new(0.12, 1.9));
    let canopy_mesh = meshes.add(Sphere::new(0.9));

    for (entity, transform) in query.iter() {
        let trunk_transform =
            Transform::from_translation(transform.translation + Vec3::new(0.0, 0.95, 0.0));

        commands
            .entity(entity)
            .insert(PbrBundle {
                mesh: trunk_mesh.clone(),
                material: bark_material.clone(),
                transform: trunk_transform,
                ..default()
            })
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

fn spawn_player_visuals(
    mut commands: Commands,
    query: Query<Entity, Added<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
            ..default()
        });
    }
}

fn spawn_zombie_visuals(
    mut commands: Commands,
    query: Query<Entity, Added<Zombie>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::srgb(0.2, 0.8, 0.2)),
            ..default()
        });
    }
}

fn camera_follow(
    player_query: Query<(&Transform, &PlayerOwner), (With<Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    my_client_id: Res<MyClientId>,
    camera_rotation: Res<CameraRotation>,
) {
    for (player_transform, owner) in player_query.iter() {
        if owner.0.get() == my_client_id.0 {
            if let Ok(mut camera_transform) = camera_query.get_single_mut() {
                // Calculate camera offset using yaw and pitch
                let distance = 10.0;
                let yaw = camera_rotation.yaw;
                let pitch = camera_rotation.pitch;

                // Calculate the offset vector from yaw and pitch
                let offset = Vec3::new(
                    distance * pitch.cos() * yaw.sin(),
                    // distance * pitch.sin(),
                    2.0,
                    distance * pitch.cos() * yaw.cos(),
                );

                camera_transform.translation = player_transform.translation + offset;
                camera_transform.look_at(player_transform.translation, Vec3::Y);
            }
        }
    }
}

fn handle_camera_rotation(
    mut mouse_motion: EventReader<MouseMotion>,
    mut camera_rotation: ResMut<CameraRotation>,
) {
    const SENSITIVITY: f32 = 0.003;
    const PITCH_LIMIT: f32 = 1.5; // Limit pitch to avoid flipping

    for motion in mouse_motion.read() {
        camera_rotation.yaw -= motion.delta.x * SENSITIVITY;
        camera_rotation.pitch =
            (camera_rotation.pitch - motion.delta.y * SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
}

fn lock_cursor(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}
