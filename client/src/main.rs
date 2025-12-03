use bevy::animation::{AnimationPlayer, AnimationTarget};
use bevy::camera::primitives::Aabb;
use bevy::ecs::hierarchy::ChildOf;
use bevy::gltf::{
    GltfExtras, GltfMaterialExtras, GltfMaterialName, GltfMeshExtras, GltfMeshName, GltfSceneExtras,
};
use bevy::input::mouse::MouseMotion;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::pbr::prelude::*;
use bevy::prelude::*;
use bevy::scene::SceneRoot;
use bevy::window::{PresentMode, PrimaryWindow, WindowPlugin};
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{ClientAuthentication, NetcodeClientTransport},
    renet2::{ConnectionConfig, RenetClient},
    RenetChannelsExt, RepliconRenetPlugins,
};
use bevy_simple_text_input::TextInputPlugin;
use renet2_netcode::NativeSocket;
use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::SystemTime,
};
use zombrise_shared::players::player::{
    handle_input, CameraRotation, DamageFlash, Health, MainCamera, Player, PlayerOwner,
};
use zombrise_shared::shared::{MapMarker, SharedPlugin, TreeMarker};
use zombrise_shared::zombie::zombie::{setup_zombie_animation, Zombie};

mod map;
use map::{spawn_snow_landscape, SnowLandscapeConfig};

mod startup_screen;
use startup_screen::{
    cleanup_startup_screen, handle_copy_paste, handle_startup_ui, show_startup_screen, AppState,
    ServerConfig,
};

mod death_screen;
use death_screen::{detect_player_death, handle_death_screen_input, show_death_screen, PlayerDied};

fn client_event_system(client: Res<RenetClient>, mut player_died: ResMut<PlayerDied>) {
    if client.is_disconnected() {
        if !player_died.0 {
            println!("Client disconnected");
            player_died.0 = true;
        }
    } else if player_died.0 {
        player_died.0 = false;
    }
}

#[derive(Resource)]
pub struct MyClientId(pub u64);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Fifo,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)
        .add_plugins(SharedPlugin)
        .add_plugins(TextInputPlugin)
        .init_state::<AppState>()
        .init_resource::<ServerConfig>()
        .insert_resource(CameraRotation {
            yaw: 0.0,
            pitch: -0.3,
        })
        .init_resource::<PlayerDied>()
        .add_systems(Startup, setup_camera)
        .register_type::<Transform>()
        .register_type::<GlobalTransform>()
        .register_type::<Visibility>()
        .register_type::<InheritedVisibility>()
        .register_type::<ViewVisibility>()
        .register_type::<bevy::transform::components::TransformTreeChanged>()
        .register_type::<Children>()
        .register_type::<ChildOf>()
        .register_type::<Name>()
        .register_type::<AnimationTarget>()
        .register_type::<AnimationPlayer>()
        .register_type::<SkinnedMesh>()
        .register_type::<MeshMaterial3d<StandardMaterial>>()
        .register_type::<Mesh3d>()
        .register_type::<Aabb>()
        .register_type::<GltfMeshName>()
        .register_type::<GltfMaterialName>()
        .register_type::<GltfExtras>()
        .register_type::<GltfSceneExtras>()
        .register_type::<GltfMeshExtras>()
        .register_type::<GltfMaterialExtras>()
        .add_systems(OnEnter(AppState::StartupScreen), show_startup_screen)
        .add_systems(OnExit(AppState::StartupScreen), cleanup_startup_screen)
        .add_systems(
            Update,
            (handle_startup_ui, handle_copy_paste).run_if(in_state(AppState::StartupScreen)),
        )
        .add_systems(
            OnEnter(AppState::Playing),
            (setup, setup_client, lock_cursor, activate_game_cameras),
        )
        .add_systems(OnExit(AppState::Playing), cleanup_playing_state)
        .add_systems(
            Update,
            (
                client_event_system,
                handle_input,
                handle_camera_rotation,
                camera_follow,
                spawn_player_visuals,
                spawn_map_visuals,
                spawn_zombie_visuals,
                setup_zombie_animation,
                spawn_tree_visuals,
                animate_player_damage,
                display_health_bar,
                detect_player_death,
                show_death_screen,
                handle_death_screen_input,
                handle_escape_key,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .run();
}

fn setup_client(
    mut commands: Commands,
    network_channels: Res<RepliconChannels>,
    server_config: Res<ServerConfig>,
) {
    let server_channels_config = network_channels.server_configs();
    let client_channels_config = network_channels.client_configs();

    let client = RenetClient::new(
        ConnectionConfig {
            server_channels_config,
            client_channels_config,
            available_bytes_per_tick: 16 * 1024,
        },
        false,
    );

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;

    let server_addr: SocketAddr = server_config
        .url
        .to_socket_addrs()
        .expect("Failed to resolve server address")
        .find(|addr| addr.is_ipv4()) // Prefer IPv4
        .or_else(|| {
            // Fallback to any address if no IPv4 found
            server_config.url.to_socket_addrs().ok()?.next()
        })
        .expect("No address found for server");

    println!("Connecting to server at: {}", server_addr);

    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: 0,
        server_addr,
        socket_id: 0,
        user_data: None,
    };

    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let native_socket = NativeSocket::new(socket).unwrap();
    let transport =
        NetcodeClientTransport::new(current_time, authentication, native_socket).unwrap();

    commands.insert_resource(client);
    commands.insert_resource(transport);
    commands.insert_resource(MyClientId(client_id));
}

fn setup_camera(mut commands: Commands) {
    println!("=== SETUP_CAMERA CALLED ===");

    let camera_3d_entity = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 0,                                                             // Render first
                is_active: false, // Inactive during startup screen
                clear_color: ClearColorConfig::Custom(Color::srgb(0.64, 0.74, 0.88)), // Sky color
                ..default()
            },
            Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            MainCamera,
        ))
        .id();
    println!("3D camera spawned (inactive): {:?}", camera_3d_entity);
    
    let camera_2d_entity = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1, // Render after 3D camera
                clear_color: ClearColorConfig::Custom(Color::srgb(0.15, 0.15, 0.2)),
                ..default()
            },
            IsDefaultUiCamera,
        ))
        .id();
    println!(
        "UI camera spawned (active with clear color): {:?}",
        camera_2d_entity
    );

    println!("=== SETUP_CAMERA COMPLETE ===");
}

fn activate_game_cameras(
    mut camera_3d_query: Query<&mut Camera, With<MainCamera>>,
    mut camera_2d_query: Query<&mut Camera, (With<Camera2d>, Without<MainCamera>)>,
) {
    // Activate the 3D camera
    if let Ok(mut camera) = camera_3d_query.single_mut() {
        camera.is_active = true;
    }

    // Change UI camera to transparent mode so it doesn't clear the 3D world
    if let Ok(mut camera) = camera_2d_query.single_mut() {
        camera.clear_color = ClearColorConfig::None;
    }
}

fn setup(mut commands: Commands) {
    // Directional light (Sun) with higher illuminance
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.5, -0.5, 0.0)),
        Visibility::default(),
    ));
}

fn cleanup_playing_state(
    mut commands: Commands,
    health_ui_query: Query<Entity, With<HealthBarUI>>,
) {
    for entity in health_ui_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn spawn_map_visuals(
    mut commands: Commands,
    query: Query<Entity, (Added<MapMarker>, Without<MapVisualsSpawned>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in query.iter() {
        // Don't insert Transform - it's already replicated from server
        commands.entity(entity).insert((
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapVisualsSpawned,
        ));
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
    query: Query<(Entity, &Transform), (Added<TreeMarker>, Without<TreeVisualsSpawned>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Trunk: darker grey (visible under snow)
    let bark_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.25),
        perceptual_roughness: 0.9,
        ..default()
    });

    // Foliage: snowy white (matte)
    let foliage_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.98, 0.98, 0.98),
        perceptual_roughness: 0.85,
        metallic: 0.0,
        reflectance: 0.1,
        ..default()
    });

    let trunk_mesh = meshes.add(Cylinder::new(0.12, 1.9));
    let canopy_mesh = meshes.add(Sphere::new(0.9));

    for (entity, _transform) in query.iter() {
        commands
            .entity(entity)
            .insert((
                Mesh3d(trunk_mesh.clone()),
                MeshMaterial3d(bark_material.clone()),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                TreeVisualsSpawned,
            ))
            .with_children(|parent| {
                // Lower canopy (snow-covered)
                let mut lower_canopy = Transform::from_translation(Vec3::new(0.0, 1.05, 0.0));
                lower_canopy.scale = Vec3::new(1.6, 1.15, 1.6);

                parent.spawn((
                    (
                        Mesh3d(canopy_mesh.clone()),
                        MeshMaterial3d(foliage_material.clone()),
                        lower_canopy,
                        Visibility::default(),
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                    ),
                    Name::new("Evergreen Foliage (Lower)"),
                ));

                // Upper canopy (snow-covered)
                let mut upper_canopy = Transform::from_translation(Vec3::new(0.0, 1.7, 0.0));
                upper_canopy.scale = Vec3::new(1.0, 1.1, 1.0);

                parent.spawn((
                    (
                        Mesh3d(canopy_mesh.clone()),
                        MeshMaterial3d(foliage_material.clone()),
                        upper_canopy,
                        Visibility::default(),
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                    ),
                    Name::new("Evergreen Foliage (Upper)"),
                ));
            });
    }
}

fn spawn_player_visuals(
    mut commands: Commands,
    query: Query<Entity, (Added<Player>, Without<PlayerVisualsSpawned>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            PlayerVisualsSpawned,
        ));
    }
}

fn spawn_zombie_visuals(
    mut commands: Commands,
    query: Query<Entity, (Added<Zombie>, Without<ZombieVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for entity in query.iter() {
        // First add visibility components to the parent entity
        commands.entity(entity).insert((
            ZombieVisualsSpawned,
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // Then spawn the scene as a child
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                SceneRoot(asset_server.load("zombie.glb#Scene0")),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Transform::default(),
                GlobalTransform::default(),
            ));
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
        if owner.0 == my_client_id.0 {
            if let Ok(mut camera_transform) = camera_query.single_mut() {
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
    mut mouse_motion: bevy::prelude::MessageReader<MouseMotion>,
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
    // if let Ok(mut window) = window_query.single_mut() {
    //     window.cursor.grab_mode = CursorGrabMode::Locked;
    //     window.cursor.visible = false;
    // }
}

fn handle_escape_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        // if let Ok(mut window) = window_query.single_mut() {
        //     window.cursor.grab_mode = CursorGrabMode::None;
        //     window.cursor.visible = true;
        // }
    }
}

fn animate_player_damage(
    mut player_query: Query<
        (
            &DamageFlash,
            &MeshMaterial3d<StandardMaterial>,
            &PlayerOwner,
        ),
        (With<Player>, Changed<DamageFlash>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    my_client_id: Res<MyClientId>,
) {
    for (damage_flash, material_handle, owner) in player_query.iter_mut() {
        // Only animate our own player
        if owner.0 == my_client_id.0 {
            if let Some(material) = materials.get_mut(material_handle) {
                if damage_flash.timer > 0.0 {
                    // Flash red when damaged
                    let flash_intensity = (damage_flash.timer / 0.3).clamp(0.0, 1.0);
                    material.base_color = Color::srgb(
                        0.8 + 0.2 * flash_intensity,
                        0.7 - 0.5 * flash_intensity,
                        0.6 - 0.4 * flash_intensity,
                    );
                } else {
                    // Reset to normal color
                    material.base_color = Color::srgb(0.8, 0.7, 0.6);
                }
            }
        }
    }
}

// Marker components to track visual spawning
#[derive(Component)]
struct PlayerVisualsSpawned;

#[derive(Component)]
struct ZombieVisualsSpawned;

#[derive(Component)]
struct MapVisualsSpawned;

#[derive(Component)]
struct TreeVisualsSpawned;

// Component to mark the health UI elements
#[derive(Component)]
struct HealthBarUI;

#[derive(Component)]
struct HealthBarFill;

#[derive(Component)]
struct HealthText;

fn display_health_bar(
    mut commands: Commands,
    player_query: Query<(&Health, &PlayerOwner), With<Player>>,
    my_client_id: Res<MyClientId>,
    health_ui_query: Query<Entity, With<HealthBarUI>>,
    mut health_fill_query: Query<
        (&mut Node, &mut BackgroundColor),
        (With<HealthBarFill>, Without<HealthText>),
    >,
    mut health_text_query: Query<(&mut Text, &mut TextColor), With<HealthText>>,
) {
    // Find our player's health
    let mut our_health: Option<&Health> = None;
    for (health, owner) in player_query.iter() {
        if owner.0 == my_client_id.0 {
            our_health = Some(health);
            break;
        }
    }

    // Clean up health UI if player doesn't exist
    if our_health.is_none() && !health_ui_query.is_empty() {
        for entity in health_ui_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    // If we have health data and no UI exists, create it
    if our_health.is_some() && health_ui_query.is_empty() {
        // Create health bar UI
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(20.0),
                    top: Val::Px(20.0),
                    width: Val::Px(300.0),
                    height: Val::Px(50.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                HealthBarUI,
            ))
            .with_children(|parent| {
                // Health text
                parent.spawn((
                    Text::new("Health: 100/100 (100%)"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    },
                    HealthText,
                ));

                // Health bar background
                parent
                    .spawn((
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2).into()),
                        // BorderColor::all(Color::srgb(0.8, 0.8, 0.8).into()),
                    ))
                    .with_children(|parent| {
                        // Health bar fill
                        parent.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.8, 0.2).into()),
                            HealthBarFill,
                        ));
                    });
            });
    }

    if let Some(health) = our_health {
        let health_percent = (health.current / health.max * 100.0).max(0.0);

        // Determine color based on health percentage
        let bar_color = if health_percent > 60.0 {
            Color::srgb(0.2, 0.8, 0.2) // Green
        } else if health_percent > 30.0 {
            Color::srgb(1.0, 0.8, 0.0) // Yellow
        } else {
            Color::srgb(1.0, 0.2, 0.2) // Red
        };

        // Update health bar fill width and color
        if let Ok((mut node, mut bg_color)) = health_fill_query.single_mut() {
            node.width = Val::Percent(health_percent);
            *bg_color = bar_color.into();
        }

        // Update health text
        if let Ok((mut text, mut text_color)) = health_text_query.single_mut() {
            text.0 = format!(
                "Health: {:.0}/{:.0} ({:.0}%)",
                health.current, health.max, health_percent
            );

            // Change text color based on health percentage
            text_color.0 = if health_percent > 60.0 {
                Color::srgb(0.2, 1.0, 0.2) // Green
            } else if health_percent > 30.0 {
                Color::srgb(1.0, 0.8, 0.0) // Yellow
            } else {
                Color::srgb(1.0, 0.2, 0.2) // Red
            };
        }
    }
}
