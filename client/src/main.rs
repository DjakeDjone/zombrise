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
use bevy_simple_text_input::TextInputPlugin;
use dragon_queen_shared::players::player::{
    CameraRotation, DamageFlash, Health, MainCamera, Player, PlayerOwner, handle_input,
};
use dragon_queen_shared::shared::{MapMarker, SharedPlugin, TreeMarker};
use dragon_queen_shared::zombie::zombie::Zombie;
use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::SystemTime,
};

mod map;
use map::{SnowLandscapeConfig, spawn_snow_landscape};

mod startup_screen;
use startup_screen::{
    AppState, ServerConfig, cleanup_startup_screen, handle_startup_ui, show_startup_screen,
};

mod death_screen;
use death_screen::{PlayerDied, detect_player_death, handle_death_screen_input, show_death_screen};

#[derive(Resource)]
pub struct MyClientId(pub u64);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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
        .add_systems(OnEnter(AppState::StartupScreen), show_startup_screen)
        .add_systems(OnExit(AppState::StartupScreen), cleanup_startup_screen)
        .add_systems(
            Update,
            handle_startup_ui.run_if(in_state(AppState::StartupScreen)),
        )
        .add_systems(
            OnEnter(AppState::Playing),
            (setup, setup_client, lock_cursor),
        )
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
                animate_player_damage,
                display_health_bar,
                detect_player_death,
                show_death_screen,
                handle_death_screen_input,
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

    // Parse server address from config - supports both domain names and IP addresses
    // Expected format: "domain.com:port" or "IP:port"
    // Prefer IPv4 addresses to avoid IPv6 connectivity issues
    let server_addr: SocketAddr = server_config
        .url
        .to_socket_addrs()
        .expect("Failed to resolve server address")
        .find(|addr| addr.is_ipv4())  // Prefer IPv4
        .or_else(|| {
            // Fallback to any address if no IPv4 found
            server_config.url.to_socket_addrs().ok()?.next()
        })
        .expect("No address found for server");
    
    println!("Connecting to server at: {}", server_addr);
    
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
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

fn setup_camera(mut commands: Commands) {
    // Camera - spawn at startup for UI rendering
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));
}

fn setup(mut commands: Commands) {
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
                // Lower canopy (snow-covered)
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

                // Upper canopy (snow-covered)
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

fn animate_player_damage(
    mut player_query: Query<
        (&DamageFlash, &Handle<StandardMaterial>, &PlayerOwner),
        (With<Player>, Changed<DamageFlash>),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    my_client_id: Res<MyClientId>,
) {
    for (damage_flash, material_handle, owner) in player_query.iter_mut() {
        // Only animate our own player
        if owner.0.get() == my_client_id.0 {
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
    mut health_fill_query: Query<(&mut Style, &mut BackgroundColor), (With<HealthBarFill>, Without<HealthText>)>,
    mut health_text_query: Query<&mut Text, With<HealthText>>,
) {
    // Find our player's health
    let mut our_health: Option<&Health> = None;
    for (health, owner) in player_query.iter() {
        if owner.0.get() == my_client_id.0 {
            our_health = Some(health);
            break;
        }
    }

    // If we have health data and no UI exists, create it
    if our_health.is_some() && health_ui_query.is_empty() {
        // Create health bar UI
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(20.0),
                        top: Val::Px(20.0),
                        width: Val::Px(300.0),
                        height: Val::Px(50.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                },
                HealthBarUI,
            ))
            .with_children(|parent| {
                // Health text
                parent.spawn((
                    TextBundle::from_section(
                        "Health: 100/100 (100%)",
                        TextStyle {
                            font_size: 20.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    }),
                    HealthText,
                ));

                // Health bar background
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(300.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.2, 0.2).into(),
                        border_color: Color::srgb(0.8, 0.8, 0.8).into(),
                        ..default()
                    })
                    .with_children(|parent| {
                        // Health bar fill
                        parent.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                background_color: Color::srgb(0.2, 0.8, 0.2).into(),
                                ..default()
                            },
                            HealthBarFill,
                        ));
                    });
            });
    }

    // Update health bar if it exists
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
        if let Ok((mut style, mut bg_color)) = health_fill_query.get_single_mut() {
            style.width = Val::Percent(health_percent);
            *bg_color = bar_color.into();
        }

        // Update health text
        if let Ok(mut text) = health_text_query.get_single_mut() {
            text.sections[0].value = format!(
                "Health: {:.0}/{:.0} ({:.0}%)",
                health.current,
                health.max,
                health_percent
            );
            
            // Change text color based on health percentage
            text.sections[0].style.color = if health_percent > 60.0 {
                Color::srgb(0.2, 1.0, 0.2) // Green
            } else if health_percent > 30.0 {
                Color::srgb(1.0, 0.8, 0.0) // Yellow
            } else {
                Color::srgb(1.0, 0.2, 0.2) // Red
            };
        }
    }
}
