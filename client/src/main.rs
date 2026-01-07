#![allow(clippy::type_complexity)]
//! Client main for Zombrise

use bevy::animation::{AnimationPlayer, AnimationTarget};
use bevy::ecs::hierarchy::ChildOf;
use bevy::gltf::{
    GltfExtras, GltfMaterialExtras, GltfMaterialName, GltfMeshExtras, GltfMeshName, GltfSceneExtras,
};
use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use bevy_mesh::skinning::SkinnedMesh;
use bevy_simple_text_input::TextInputPlugin;

use avian3d::prelude::*;
use lightyear::prelude::client::*;

use zombrise_shared::players::player::{CameraRotation, MainCamera, MyClientId, Player};
use zombrise_shared::players::player_animation::{
    control_player_animation, setup_player_animation, trigger_player_attack_animation,
    update_player_animation_state, update_player_attack_timer, update_player_idle_variations,
    update_player_prev_positions, PlayerAttacking,
};

use zombrise_shared::shared::{MapMarker, SharedPlugin, TreeMarker};

use zombrise_shared::zombie::zombie::{
    add_zombie_animation_events, control_zombie_animation, setup_zombie_animation,
    update_zombie_animation_state, Zombie, ZombieAnimationEventsState,
};

mod map;

mod audio;
use audio::GameAudioPlugin;

mod snowflakes;
use snowflakes::SnowfallPlugin;

mod physics;
use physics::ClientPhysicsPlugin;

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

mod game;
use game::{
    camera::{
        activate_game_cameras, camera_follow, handle_camera_rotation, handle_escape_key,
        handle_lock_key, lock_cursor, setup_camera,
    },
    fire_effects::{
        animate_fire_particles, setup_fire_assets, spawn_zombie_fire, update_dying_zombie_visuals,
        update_zombie_fire, FireParticle,
    },
    health_ui::{display_health_bar, HealthBarUI},
    player_visuals::{
        animate_player_damage, spawn_player_visuals, update_other_player_visuals,
        PlayerVisualsSpawned,
    },
    world_visuals::{spawn_map_visuals, spawn_tree_visuals, MapVisualsSpawned, TreeVisualsSpawned},
    zombie_visuals::{
        cleanup_orphaned_zombie_visuals, fix_zombie_frustum_culling, spawn_zombie_visuals,
        update_zombie_visuals_transform, ZombieVisual, ZombieVisualsSpawned,
    },
};

mod networking;
use networking::{add_input_manager, setup_client};

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
    .add_plugins(PhysicsPlugins::default())
    .add_plugins(ClientPhysicsPlugin)
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
    // Register types for replication
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
    .register_type::<bevy::camera::primitives::Aabb>()
    .register_type::<GltfMeshName>()
    .register_type::<GltfMaterialName>()
    .register_type::<GltfExtras>()
    .register_type::<GltfSceneExtras>()
    .register_type::<GltfMeshExtras>()
    .register_type::<GltfMaterialExtras>()
    // Startup screen
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
    // Loading state
    .add_systems(
        OnEnter(AppState::Loading),
        (show_loading_screen, start_loading_assets),
    )
    .add_systems(
        Update,
        check_loading_progress.run_if(in_state(AppState::Loading)),
    )
    .add_systems(OnExit(AppState::Loading), cleanup_loading_screen)
    // Playing state
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
    // Camera and input
    .add_systems(
        Update,
        (add_input_manager, handle_camera_rotation, camera_follow).chain(),
    )
    // Visual spawning and interpolation
    .add_systems(
        Update,
        (
            spawn_player_visuals,
            update_other_player_visuals,
            spawn_map_visuals,
            spawn_zombie_visuals,
            update_zombie_visuals_transform,
            cleanup_orphaned_zombie_visuals,
        ),
    )
    // Zombie animation
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
    // Fire effects
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
    // Player systems
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

/// Setup game lighting
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

/// Cleanup when exiting playing state
fn cleanup_playing_state(
    mut commands: Commands,
    health_ui_query: Query<Entity, With<HealthBarUI>>,
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
            With<ZombieVisual>,
            With<FireParticle>,
        )>,
    >,
    // Query for network client entities
    network_client_query: Query<Entity, With<lightyear::prelude::client::NetcodeClient>>,
    // Query for entities with InputMarker
    input_marker_query: Query<
        Entity,
        With<lightyear::prelude::input::native::InputMarker<zombrise_shared::protocol::GameInput>>,
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

    // Despawn network client entities (this will disconnect from server)
    for entity in network_client_query.iter() {
        commands.entity(entity).despawn();
    }

    // Remove entities with InputMarker (they should have been despawned with Player entities,
    // but we're double-checking to prevent multiple entity errors on reconnect)
    for entity in input_marker_query.iter() {
        if commands.get_entity(entity).is_ok() {
            commands.entity(entity).despawn();
        }
    }

    // Remove network resources
    commands.remove_resource::<MyClientId>();

    // Reset player dead state
    commands.insert_resource(PlayerDied(false));
}
