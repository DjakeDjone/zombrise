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
use bevy::window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowPlugin};
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
use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{
    handle_input, CameraRotation, DamageFlash, LocalPlayerPosition, LocalPlayerRotation,
    MainCamera, MyClientId, Player, PlayerOwner,
};
use zombrise_shared::players::player_animation::{
    control_player_animation, setup_player_animation, trigger_player_attack_animation,
    update_player_animation_state, update_player_attack_timer, update_player_idle_variations,
    update_player_prev_positions, PlayerAttacking,
};
use zombrise_shared::shared::{MapMarker, SharedPlugin, TreeMarker};
use zombrise_shared::suduxu::SuduxuPlugin;
use zombrise_shared::zombie::zombie::{
    add_zombie_animation_events, control_zombie_animation, handle_zombie_animation_events,
    setup_zombie_animation, update_zombie_animation_state, Zombie, ZombieAnimationEvent,
    ZombieAnimationEventsState, ZombieLink,
};

mod map;
use map::{create_map_assets, spawn_snow_landscape, MapAssets};

mod audio;
use audio::GameAudioPlugin;

mod snowflakes;
use snowflakes::SnowfallPlugin;

mod startup_screen;
use startup_screen::{
    cleanup_startup_screen, handle_copy_paste, handle_quick_connect_buttons, handle_startup_ui,
    show_startup_screen, AppState, ServerConfig,
};

mod death_screen;
use death_screen::{detect_player_death, handle_death_screen_input, show_death_screen, PlayerDied};

mod loading_screen;
use loading_screen::{
    check_loading_progress, cleanup_loading_screen, show_loading_screen, start_loading_assets,
};

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

// MyClientId is now imported from zombrise_shared::players::player

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
        .add_plugins(SuduxuPlugin)
        .add_plugins(GameAudioPlugin)
        .add_plugins(SnowfallPlugin)
        .init_state::<AppState>()
        .init_resource::<ServerConfig>()
        .insert_resource(CameraRotation {
            yaw: 0.0,
            pitch: -0.3,
        })
        .init_resource::<PlayerDied>()
        .init_resource::<MyClientId>()
        .init_resource::<ZombieAnimationEventsState>()
        .add_message::<ZombieAnimationEvent>()
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
            (
                handle_startup_ui,
                handle_copy_paste,
                handle_quick_connect_buttons,
            )
                .run_if(in_state(AppState::StartupScreen)),
        )
        // Loading state systems
        .add_systems(
            OnEnter(AppState::Loading),
            (show_loading_screen, start_loading_assets),
        )
        .add_systems(
            Update,
            check_loading_progress.run_if(in_state(AppState::Loading)),
        )
        .add_systems(OnExit(AppState::Loading), cleanup_loading_screen)
        .add_systems(
            OnEnter(AppState::Playing),
            (setup, setup_client, lock_cursor, activate_game_cameras),
        )
        .add_systems(OnExit(AppState::Playing), cleanup_playing_state)
        .add_systems(
            Update,
            (
                client_event_system,
                (
                    handle_client_auto_aim,
                    handle_camera_rotation,
                    handle_input,
                    camera_follow,
                )
                    .chain(),
                spawn_player_visuals,
                spawn_map_visuals,
                spawn_zombie_visuals,
                update_zombie_visuals_transform,
                cleanup_orphaned_zombie_visuals,
                setup_zombie_animation,
                update_zombie_animation_state,
                control_zombie_animation,
                add_zombie_animation_events,
                handle_zombie_animation_events,
                spawn_tree_visuals,
                fix_zombie_frustum_culling,
            )
                .run_if(in_state(AppState::Playing)),
        )
        .add_systems(
            Update,
            (
                setup_player_animation,
                trigger_player_attack_animation,
                update_player_attack_timer,
                update_player_idle_variations,
                animate_player_damage,
                display_health_bar,
                detect_player_death,
                show_death_screen,
                handle_death_screen_input,
                handle_escape_key,
                handle_lock_key,
                (
                    update_player_animation_state,
                    control_player_animation,
                    update_player_prev_positions,
                )
                    .chain(),
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
        .or_else(|| server_config.url.to_socket_addrs().ok()?.next())
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

    // Set the client ID immediately so we can identify our player
    // Note: client_id is the renet2 auth ID we use for authentication
    commands.insert_resource(MyClientId(client_id));
    println!("Client ID set to: {}", client_id);
}

fn setup_camera(mut commands: Commands) {
    println!("=== SETUP_CAMERA CALLED ===");

    let camera_3d_entity = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 0,
                is_active: false,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.64, 0.74, 0.88)),
                ..default()
            },
            Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            MainCamera,
            // Atmospheric winter fog - gives depth to the snowy landscape
            DistanceFog {
                color: Color::srgba(0.70, 0.75, 0.82, 1.0), // Cold, bluish-grey fog
                falloff: FogFalloff::Linear {
                    start: 25.0, // No fog closer than this
                    end: 100.0,  // Full fog at this distance
                },
                ..default()
            },
        ))
        .id();
    println!("3D camera spawned (inactive): {:?}", camera_3d_entity);

    let camera_2d_entity = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1,
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
    // Activate 3D camera
    if let Ok(mut camera) = camera_3d_query.single_mut() {
        camera.is_active = true;
    }

    // Set UI transparent
    if let Ok(mut camera) = camera_2d_query.single_mut() {
        camera.clear_color = ClearColorConfig::None;
    }
}

fn setup(mut commands: Commands) {
    // Sun light
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
    // Query for all game entities to clean up
    game_entities: Query<
        Entity,
        Or<(
            With<Player>,
            With<Zombie>,
            With<MapMarker>,
            With<TreeMarker>,
            With<PlayerVisualsSpawned>,
            With<ZombieVisualsSpawned>,
            With<TreeVisualsSpawned>,
            With<MapVisualsSpawned>,
            With<ZombieVisual>, // Also clean up detached visuals
        )>,
    >,
) {
    // Remove UI
    for entity in health_ui_query.iter() {
        commands.entity(entity).despawn();
    }

    // Remove game entities
    for entity in game_entities.iter() {
        commands.entity(entity).despawn();
    }

    // Remove network resources to ensure clean disconnection
    commands.remove_resource::<RenetClient>();
    commands.remove_resource::<NetcodeClientTransport>();
    commands.remove_resource::<MyClientId>();

    // Reset player dead state
    commands.insert_resource(PlayerDied(false));

    println!("Cleaned up playing state (entities and network resources)");
}

fn spawn_map_visuals(
    mut commands: Commands,
    query: Query<Entity, (Added<MapMarker>, Without<MapVisualsSpawned>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut map_assets_cache: Local<Option<MapAssets>>,
) {
    if map_assets_cache.is_none() {
        *map_assets_cache = Some(create_map_assets(
            &mut meshes,
            &mut materials,
            &asset_server,
        ));
    }

    let Some(map_assets) = map_assets_cache.as_ref() else {
        return;
    };

    for entity in query.iter() {
        commands.entity(entity).insert((
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            MapVisualsSpawned,
        ));

        spawn_snow_landscape(&mut commands, map_assets, entity);
    }
}

fn spawn_tree_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (Added<TreeMarker>, Without<TreeVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in query.iter() {
        // Check if this is the giant corner tree (at radius * 0.9 ≈ 25.2)
        let is_giant_tree = transform.translation.x > 24.0 && transform.translation.z > 24.0;

        let scale_factor = if is_giant_tree {
            5.0 // Giant tree scale
        } else {
            // Use position as seed for deterministic random scale
            let seed = (transform.translation.x.abs() + transform.translation.z.abs()) * 1000.0;
            0.6 + (seed.sin().abs() * 0.8) // Range: 0.6 to 1.4
        };

        commands
            .entity(entity)
            .insert((
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                TreeVisualsSpawned,
            ))
            .with_children(|parent| {
                parent.spawn((
                    SceneRoot(asset_server.load("Pine Tree with Snow.glb#Scene0")),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    Transform::from_scale(Vec3::splat(scale_factor)),
                    GlobalTransform::default(),
                    Name::new("Pine Tree with Snow"),
                ));
            });
    }
}

fn spawn_player_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (Added<Player>, Without<PlayerVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in query.iter() {
        commands.entity(entity).insert((
            PlayerVisualsSpawned,
            LocalPlayerPosition(transform.translation),
            LocalPlayerRotation(transform.rotation),
            PlayerAttacking::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // Offset model
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                SceneRoot(asset_server.load("player.glb#Scene0")),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                Transform::from_translation(Vec3::new(0.0, -1.1, 0.0))
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                GlobalTransform::default(),
                PlayerVisualMesh,
            ));
        });
    }
}

#[derive(Component)]
pub struct ZombieVisual;

fn spawn_zombie_visuals(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (Added<Zombie>, Without<ZombieVisualsSpawned>)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, transform) in query.iter() {
        // Mark the logic entity as having visuals to prevent duplicate processing
        commands.entity(entity).insert((
            ZombieVisualsSpawned,
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));

        // Spawn a separate entity for the visual mesh to allow smooth interpolation
        // unrelated to the network snap updates on the main zombie entity.
        commands.spawn((
            SceneRoot(asset_server.load("zombie.glb#Scene0")),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // Start at the zombie's current position
            Transform::from_translation(transform.translation + Vec3::new(0.0, -0.75, 0.0))
                .with_rotation(transform.rotation * Quat::from_rotation_y(std::f32::consts::PI)),
            GlobalTransform::default(),
            ZombieVisual,
            ZombieLink(entity),
        ));
    }
}

fn update_zombie_visuals_transform(
    mut visual_query: Query<(&mut Transform, &ZombieLink), With<ZombieVisual>>,
    zombie_query: Query<&Transform, (With<Zombie>, Without<ZombieVisual>)>,
    time: Res<Time>,
) {
    for (mut visual_transform, link) in visual_query.iter_mut() {
        if let Ok(target_transform) = zombie_query.get(link.0) {
            // Target position (with offset)
            // Note: The offset was applied in spawn_zombie_visuals:
            // Translation: zombie.translation + (0.0, -0.75, 0.0)
            // Rotation: zombie.rotation * PI_Y

            let target_translation = target_transform.translation + Vec3::new(0.0, -0.75, 0.0);
            let target_rotation =
                target_transform.rotation * Quat::from_rotation_y(std::f32::consts::PI);

            // Interpolation speed
            let t = time.delta_secs() * 10.0; // Adjustable smoothness

            // Interpolate position
            visual_transform.translation = visual_transform.translation.lerp(target_translation, t);

            // Interpolate rotation
            visual_transform.rotation = visual_transform.rotation.slerp(target_rotation, t);

            // Snap if too far (teleport)
            if visual_transform.translation.distance(target_translation) > 2.0 {
                visual_transform.translation = target_translation;
                visual_transform.rotation = target_rotation;
            }
        }
    }
}

fn cleanup_orphaned_zombie_visuals(
    mut commands: Commands,
    visual_query: Query<(Entity, &ZombieLink), With<ZombieVisual>>,
    zombie_query: Query<Entity, With<Zombie>>,
    children_query: Query<&Children>,
) {
    for (entity, link) in visual_query.iter() {
        if !zombie_query.contains(link.0) {
            despawn_with_children_recursive(&mut commands, entity, &children_query);
        }
    }
}

fn despawn_with_children_recursive(
    commands: &mut Commands,
    entity: Entity,
    children_query: &Query<&Children>,
) {
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            despawn_with_children_recursive(commands, child, children_query);
        }
    }
    commands.entity(entity).despawn();
}

fn handle_client_auto_aim(
    mut player_query: Query<(Entity, &mut Transform, &PlayerOwner), With<Player>>,
    mut local_rot_query: Query<&mut LocalPlayerRotation>,
    zombie_query: Query<&Transform, (With<Zombie>, Without<Player>)>,
    player_attacking_query: Query<(&PlayerAttacking, &PlayerOwner)>,
    my_client_id: Res<MyClientId>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    suduxu_input: Option<Res<ButtonInput<zombrise_shared::suduxu::SuduxuButton>>>,
) {
    let suduxu_clicked = suduxu_input.map_or(false, |s| {
        s.just_pressed(zombrise_shared::suduxu::SuduxuButton::A)
    });

    if keyboard_input.just_pressed(KeyCode::Space) || suduxu_clicked {
        let mut local_player_data = None;
        for (entity, transform, owner) in &mut player_query {
            if owner.0 == my_client_id.0 {
                local_player_data = Some((entity, transform));
                break;
            }
        }

        if let Some((entity, mut transform)) = local_player_data {
            // Check attacking state
            let is_attacking = player_attacking_query
                .iter()
                .any(|(a, o)| o.0 == my_client_id.0 && a.is_attacking);
            if is_attacking {
                return;
            }

            // Find closest zombie
            let mut closest_dist_sq = 100.0; // 10.0^2 range (matches server)
            let mut target_pos = None;

            for zombie_transform in &zombie_query {
                let dist_sq = transform
                    .translation
                    .distance_squared(zombie_transform.translation);
                if dist_sq < closest_dist_sq {
                    closest_dist_sq = dist_sq;
                    target_pos = Some(zombie_transform.translation);
                }
            }

            if let Some(pos) = target_pos {
                let diff = pos - transform.translation;
                let horizontal_diff = Vec3::new(diff.x, 0.0, diff.z);
                if horizontal_diff.length_squared() > 0.001 {
                    let target_rotation =
                        Quat::from_rotation_arc(Vec3::NEG_Z, horizontal_diff.normalize());
                    transform.rotation = target_rotation;

                    if let Ok(mut local_rot) = local_rot_query.get_mut(entity) {
                        local_rot.0 = target_rotation;
                    }
                }
            }
        }
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
    const PITCH_LIMIT: f32 = 1.5;

    for motion in mouse_motion.read() {
        camera_rotation.yaw -= motion.delta.x * SENSITIVITY;
        camera_rotation.pitch =
            (camera_rotation.pitch - motion.delta.y * SENSITIVITY).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
}

fn lock_cursor(mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Some(mut options) = cursor_query.iter_mut().next() {
        options.grab_mode = CursorGrabMode::Locked;
        options.visible = false;
    }
}

fn handle_escape_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Some(mut options) = cursor_query.iter_mut().next() {
            options.grab_mode = CursorGrabMode::None;
            options.visible = true;
        }
    }
}

fn handle_lock_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        if let Some(mut options) = cursor_query.iter_mut().next() {
            options.grab_mode = CursorGrabMode::Locked;
            options.visible = false;
        }
    }
}

fn animate_player_damage(
    player_query: Query<
        (&DamageFlash, &PlayerOwner, &Children),
        (With<Player>, Changed<DamageFlash>),
    >,
    visual_mesh_query: Query<&MeshMaterial3d<StandardMaterial>, With<PlayerVisualMesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    my_client_id: Res<MyClientId>,
) {
    for (damage_flash, owner, children) in player_query.iter() {
        if owner.0 == my_client_id.0 {
            // Find the visual mesh child
            for child in children.iter() {
                if let Ok(material_handle) = visual_mesh_query.get(child) {
                    if let Some(material) = materials.get_mut(material_handle) {
                        if damage_flash.timer > 0.0 {
                            // Flash red
                            let flash_intensity = (damage_flash.timer / 0.3).clamp(0.0, 1.0);
                            material.base_color = Color::srgb(
                                0.8 + 0.2 * flash_intensity,
                                0.7 - 0.5 * flash_intensity,
                                0.6 - 0.4 * flash_intensity,
                            );
                        } else {
                            material.base_color = Color::srgb(0.8, 0.7, 0.6);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component)]
struct PlayerVisualsSpawned;

#[derive(Component)]
struct PlayerVisualMesh;

#[derive(Component)]
struct ZombieVisualsSpawned;

#[derive(Component)]
struct MapVisualsSpawned;

#[derive(Component)]
struct TreeVisualsSpawned;

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

    if our_health.is_some() && health_ui_query.is_empty() {
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

                parent
                    .spawn((
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2).into()),
                    ))
                    .with_children(|parent| {
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

        // Color from health
        let bar_color = if health_percent > 60.0 {
            Color::srgb(0.2, 0.8, 0.2)
        } else if health_percent > 30.0 {
            Color::srgb(1.0, 0.8, 0.0)
        } else {
            Color::srgb(1.0, 0.2, 0.2)
        };

        // Update health bar fill width and color
        if let Ok((mut node, mut bg_color)) = health_fill_query.single_mut() {
            node.width = Val::Percent(health_percent);
            *bg_color = bar_color.into();
        }

        if let Ok((mut text, mut text_color)) = health_text_query.single_mut() {
            text.0 = format!(
                "Health: {:.0}/{:.0} ({:.0}%)",
                health.current, health.max, health_percent
            );

            text_color.0 = if health_percent > 60.0 {
                Color::srgb(0.2, 1.0, 0.2)
            } else if health_percent > 30.0 {
                Color::srgb(1.0, 0.8, 0.0)
            } else {
                Color::srgb(1.0, 0.2, 0.2)
            };
        }
    }
}

fn fix_zombie_frustum_culling(
    mut commands: Commands,
    skinned_mesh_query: Query<Entity, Added<SkinnedMesh>>,
    parent_query: Query<&ChildOf>,
    zombie_query: Query<Entity, With<ZombieVisual>>, // Updated to check ZombieVisual
) {
    for entity in skinned_mesh_query.iter() {
        // Check if this mesh belongs to a zombie
        let mut current = entity;
        let mut is_zombie = false;

        // Traverse up to find ZombieVisual component (since mesh is now child of Visual)
        while let Ok(child_of) = parent_query.get(current) {
            current = child_of.parent();
            if zombie_query.contains(current) {
                is_zombie = true;
                break;
            }
        }

        if is_zombie {
            // Expand AABB to prevent culling issues
            commands.entity(entity).insert(Aabb {
                center: Vec3::new(0.0, 1.0, 0.0).into(),
                half_extents: Vec3::splat(5.0).into(),
            });
        }
    }
}
