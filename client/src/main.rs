#![allow(clippy::type_complexity)]

use bevy::animation::{AnimationPlayer, AnimationTarget};
use bevy::camera::primitives::Aabb;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::relationship::Relationship;
use bevy::gltf::{
    GltfExtras, GltfMaterialExtras, GltfMaterialName, GltfMeshExtras, GltfMeshName, GltfSceneExtras,
};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow};
use bevy_mesh::skinning::SkinnedMesh;
use bevy_simple_text_input::TextInputPlugin;
use std::time::SystemTime;

use lightyear::prelude::client::*;

use std::net::{SocketAddr, ToSocketAddrs};
use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{
    CameraRotation, DamageFlash, LocalPlayerPosition, LocalPlayerRotation, MainCamera, MyClientId,
    Player, PlayerOwner,
};
use zombrise_shared::players::player_animation::{
    control_player_animation, setup_player_animation, trigger_player_attack_animation,
    update_player_animation_state, update_player_attack_timer, update_player_idle_variations,
    update_player_prev_positions, PlayerAttacking,
};

use zombrise_shared::shared::{MapMarker, SharedPlugin, TreeMarker, ZombieDying};

use zombrise_shared::zombie::zombie::{
    add_zombie_animation_events, control_zombie_animation, setup_zombie_animation,
    update_zombie_animation_state, Zombie, ZombieAnimationEventsState, ZombieLink,
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

// MyClientId is now imported from zombrise_shared::players::player

// --- Marker Components ---
#[derive(Component)]
pub struct ZombieVisual;

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
// -------------------------

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(zombrise_shared::suduxu::SuduxuPlugin);

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Fifo,
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin {
                level: bevy::log::Level::INFO,
                filter: "wgpu=error,bevy_render=info,bevy_ecs=info,lightyear=info".to_string(),
                ..default()
            }),
    )
    .add_plugins(ClientPlugins::default())
    .add_plugins(SharedPlugin)
    .add_plugins(TextInputPlugin)
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
        (
            setup,
            setup_client,
            lock_cursor,
            activate_game_cameras,
            setup_fire_assets,
        ),
    )
    .add_systems(OnExit(AppState::Playing), cleanup_playing_state)
    .add_systems(
        Update,
        (add_input_manager, handle_camera_rotation, camera_follow).chain(),
    )
    .add_systems(
        Update,
        (
            spawn_player_visuals,
            spawn_map_visuals,
            spawn_zombie_visuals,
            update_zombie_visuals_transform,
            cleanup_orphaned_zombie_visuals,
        ),
    )
    .add_systems(
        Update,
        (
            setup_zombie_animation,
            update_zombie_animation_state,
            control_zombie_animation,
            add_zombie_animation_events,
            spawn_tree_visuals,
        ),
    )
    .add_systems(
        Update,
        (
            fix_zombie_frustum_culling,
            spawn_zombie_fire,
            update_zombie_fire,
            animate_fire_particles,
            update_dying_zombie_visuals,
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

fn setup_client(mut commands: Commands, server_config: Res<ServerConfig>) {
    #[cfg(target_arch = "wasm32")]
    {
        // Networking disabled on WASM
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use lightyear::prelude::client::{NetcodeClient, NetcodeConfig};
        use lightyear::prelude::Authentication;
        use lightyear_udp::UdpIo;

        let server_addr: SocketAddr = server_config
            .url
            .to_socket_addrs()
            .expect("Failed to resolve server address")
            .find(|addr| addr.is_ipv4()) // Prefer IPv4
            .or_else(|| server_config.url.to_socket_addrs().ok()?.next())
            .expect("No address found for server");

        info!("Connecting to server at: {}", server_addr);

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let client_id = current_time.as_millis() as u64;

        // Create authentication with Manual mode for testing
        let auth = Authentication::Manual {
            server_addr,
            client_id,
            private_key: [0u8; 32], // Match server's private key
            protocol_id: 0,         // Match server's protocol id
        };

        // Create NetcodeClient with authentication
        let netcode_config = NetcodeConfig::default();
        match NetcodeClient::new(auth, netcode_config) {
            Ok(netcode_client) => {
                // Spawn the client networking entity with LocalAddr for UDP binding
                use lightyear::prelude::{LocalAddr, ReplicationReceiver, ReplicationSender};
                let client_local_addr = SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                    0, // Let OS assign an available port
                );

                let client_entity = commands
                    .spawn((
                        Name::new("NetworkClient"),
                        LocalAddr(client_local_addr),
                        netcode_client,
                        UdpIo::default(),
                        ReplicationReceiver::default(),
                        ReplicationSender::default(),
                    ))
                    .id();

                // Trigger Connect event to initiate connection
                use lightyear::prelude::client::Connect;
                commands.trigger(Connect {
                    entity: client_entity,
                });
            }
            Err(e) => {
                error!("Failed to create NetcodeClient: {:?}", e);
            }
        }

        // Set the client ID immediately so we can identify our player
        commands.insert_resource(MyClientId(client_id));
    }
}

fn add_input_manager(
    mut commands: Commands,
    player_query: Query<(Entity, &PlayerOwner), With<Player>>,
    my_client_id: Res<MyClientId>,
    input_query: Query<
        Entity,
        With<lightyear::prelude::input::native::InputMarker<zombrise_shared::protocol::GameInput>>,
    >,
) {
    // Add InputMarker to the local player entity so inputs are attached to it
    for (entity, owner) in &player_query {
        // Only add input components if they are not already present
        if owner.0 == my_client_id.0 && input_query.get(entity).is_err() {
            use lightyear::prelude::input::native::{ActionState, InputMarker};
            use zombrise_shared::protocol::GameInput;

            commands.entity(entity).insert((
                InputMarker::<GameInput>::default(),
                ActionState::<GameInput>::default(),
            ));
        }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
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
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.15, 0.15, 0.2)),
            ..default()
        },
        IsDefaultUiCamera,
    ));
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
            With<FireParticle>, // Clean up fire particles
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

    // Remove network resources / disconnect
    // Client disconnection is handled automatically by Lightyear
    commands.remove_resource::<MyClientId>();

    // Reset player dead state
    commands.insert_resource(PlayerDied(false));
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
    mut mouse_motion: EventReader<MouseMotion>,
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
    if let Ok(mut options) = cursor_query.single_mut() {
        options.grab_mode = CursorGrabMode::Locked;
        options.visible = false;
    }
}

fn handle_escape_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut options) = cursor_query.single_mut() {
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
        if let Ok(mut options) = cursor_query.single_mut() {
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
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.8, 0.2)),
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
            current = child_of.get(); // Relationship trait provides get() method
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

// ============== ZOMBIE DEATH FIRE EFFECT ==============

/// Marker component for fire particle entities
#[derive(Component)]
pub struct FireParticle {
    /// Lifetime remaining for this particle
    pub lifetime: f32,
    /// Velocity of the particle
    pub velocity: Vec3,
    /// Initial size
    pub initial_size: f32,
}

/// Marker for zombies that have fire spawned
#[derive(Component)]
pub struct ZombieFireSpawned;

/// Resource to cache fire particle assets
#[derive(Resource)]
struct FireParticleAssets {
    mesh: Handle<Mesh>,
    material_orange: Handle<StandardMaterial>,
    material_yellow: Handle<StandardMaterial>,
    material_red: Handle<StandardMaterial>,
}

/// Sets up fire particle assets
fn setup_fire_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));

    let material_orange = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.4, 0.0, 0.9),
        emissive: LinearRgba::new(5.0, 2.0, 0.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let material_yellow = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.8, 0.0, 0.85),
        emissive: LinearRgba::new(5.0, 4.0, 0.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let material_red = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.1, 0.0, 0.8),
        emissive: LinearRgba::new(4.0, 0.5, 0.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(FireParticleAssets {
        mesh,
        material_orange,
        material_yellow,
        material_red,
    });
}

/// Spawns fire particles on dying zombies
fn spawn_zombie_fire(
    mut commands: Commands,
    zombie_visuals: Query<
        (Entity, &Transform, &ZombieLink),
        (With<ZombieVisual>, Without<ZombieFireSpawned>),
    >,
    dying_zombies: Query<&ZombieDying, With<Zombie>>,
    fire_assets: Option<Res<FireParticleAssets>>,
) {
    let Some(assets) = fire_assets else { return };

    for (visual_entity, transform, link) in &zombie_visuals {
        // Check if the linked zombie is dying
        let Ok(dying) = dying_zombies.get(link.0) else {
            continue;
        };

        // Only start fire during burn phase
        if dying.timer < dying.fall_duration {
            continue;
        }

        spawn_fire_burst(&mut commands, &assets, transform.translation, visual_entity);
    }
}

fn spawn_fire_burst(
    commands: &mut Commands,
    assets: &FireParticleAssets,
    position: Vec3,
    parent_entity: Entity,
) {
    // Mark as having fire spawned
    commands.entity(parent_entity).insert(ZombieFireSpawned);

    // Spawn initial burst of fire particles
    let rng = || fastrand::f32();

    for _ in 0..15 {
        let offset = Vec3::new((rng() - 0.5) * 0.8, rng() * 0.5, (rng() - 0.5) * 0.8);

        let velocity = Vec3::new((rng() - 0.5) * 0.5, 1.0 + rng() * 2.0, (rng() - 0.5) * 0.5);

        let size = 0.15 + rng() * 0.25;
        let lifetime = 0.5 + rng() * 1.0;

        // Choose random fire color
        let material = match (rng() * 3.0) as u32 {
            0 => assets.material_orange.clone(),
            1 => assets.material_yellow.clone(),
            _ => assets.material_red.clone(),
        };

        commands.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position + offset).with_scale(Vec3::splat(size)),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            FireParticle {
                lifetime,
                velocity,
                initial_size: size,
            },
            Name::new("FireParticle"),
        ));
    }
}

/// Continuously spawn fire particles on burning zombies
fn update_zombie_fire(
    mut commands: Commands,
    zombie_visuals: Query<(&Transform, &ZombieLink), (With<ZombieVisual>, With<ZombieFireSpawned>)>,
    dying_zombies: Query<&ZombieDying, With<Zombie>>,
    fire_assets: Option<Res<FireParticleAssets>>,
    time: Res<Time>,
) {
    let Some(assets) = fire_assets else { return };

    for (transform, link) in &zombie_visuals {
        // Check if the linked zombie is dying
        let Ok(dying) = dying_zombies.get(link.0) else {
            continue;
        };

        // Only during burn phase
        if dying.timer < dying.fall_duration {
            continue;
        }

        // Spawn new particles periodically (roughly 10-15 per second)
        if fastrand::f32() < time.delta_secs() * 12.0 {
            let rng = || fastrand::f32();

            let offset = Vec3::new((rng() - 0.5) * 0.6, rng() * 0.3, (rng() - 0.5) * 0.6);

            let velocity = Vec3::new((rng() - 0.5) * 0.3, 0.8 + rng() * 1.5, (rng() - 0.5) * 0.3);

            let size = 0.1 + rng() * 0.2;
            let lifetime = 0.4 + rng() * 0.8;

            let material = match (rng() * 3.0) as u32 {
                0 => assets.material_orange.clone(),
                1 => assets.material_yellow.clone(),
                _ => assets.material_red.clone(),
            };

            commands.spawn((
                Mesh3d(assets.mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(transform.translation + offset)
                    .with_scale(Vec3::splat(size)),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                FireParticle {
                    lifetime,
                    velocity,
                    initial_size: size,
                },
                Name::new("FireParticle"),
            ));
        }
    }
}

/// Animate fire particles - rise up, flicker, and fade
fn animate_fire_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut FireParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.lifetime -= dt;

        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Move upward with slight turbulence
        transform.translation += particle.velocity * dt;

        // Add flickering motion
        let flicker_x = (elapsed * 8.0 + transform.translation.x * 5.0).sin() * 0.02;
        let flicker_z = (elapsed * 7.0 + transform.translation.z * 5.0).cos() * 0.02;
        transform.translation.x += flicker_x;
        transform.translation.z += flicker_z;

        // Slow down upward velocity over time
        particle.velocity.y *= 0.98;
        particle.velocity.x *= 0.95;
        particle.velocity.z *= 0.95;

        // Scale down as lifetime decreases (fade effect)
        let life_ratio = particle.lifetime / 1.0; // Assume max lifetime ~1.0
        let scale = particle.initial_size * life_ratio.max(0.1);
        transform.scale = Vec3::splat(scale);
    }
}

/// Handle dying zombie visual effects - make zombie fade/darken as it burns
fn update_dying_zombie_visuals(
    zombie_logic_query: Query<&ZombieDying, With<Zombie>>,
    visual_query: Query<(Entity, &ZombieLink), With<ZombieVisual>>,
) {
    // For zombie visuals linked to dying zombies, check if we need to update
    for (_visual_entity, link) in &visual_query {
        if let Ok(dying) = zombie_logic_query.get(link.0) {
            // During burn phase, the zombie should appear charred
            // This is handled by the fire effect covering the model
            let burn_progress = if dying.timer > dying.fall_duration {
                (dying.timer - dying.fall_duration) / dying.burn_duration
            } else {
                0.0
            };

            // Could add material darkening here if needed
            let _ = burn_progress; // Currently just using fire overlay
        }
    }
}
