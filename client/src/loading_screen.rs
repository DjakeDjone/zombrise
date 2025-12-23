use bevy::asset::LoadState;
use bevy::gltf::Gltf;
use bevy::prelude::*;

use crate::startup_screen::AppState;
use zombrise_shared::players::player_animation::PlayerAnimationConfig;

#[derive(Component)]
pub struct LoadingScreenMarker;

#[derive(Component)]
pub struct LoadingProgressBar;

#[derive(Component)]
pub struct LoadingStatusText;

/// Resource that holds handles to all assets being loaded
#[derive(Resource, Default)]
pub struct GameAssets {
    pub zombie_model: Handle<Gltf>,
    pub player_idle: Handle<AnimationClip>,
    pub player_idle_nervous: Handle<AnimationClip>,
    pub player_walking: Handle<AnimationClip>,
    pub player_attacking: Handle<AnimationClip>,
    pub loading_complete: bool,
}

/// Resource to track loading progress
#[derive(Resource, Default)]
pub struct LoadingProgress {
    pub assets_loaded: usize,
    pub total_assets: usize,
}

/// Starts asset loading
pub fn start_loading_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("=== START_LOADING_ASSETS ===");

    // Load assets
    let config = PlayerAnimationConfig::default();
    let zombie_model: Handle<Gltf> = asset_server.load("zombie.glb");
    let player_idle = asset_server.load(config.idle_animation.path);
    let player_idle_nervous = asset_server.load(config.idle_nervous_animation.path);
    let player_walking = asset_server.load(config.walking_animation.path);
    let player_attacking = asset_server.load(config.attacking_animation.path);

    commands.insert_resource(GameAssets {
        zombie_model,
        player_idle,
        player_idle_nervous,
        player_walking,
        player_attacking,
        loading_complete: false,
    });

    commands.insert_resource(LoadingProgress {
        assets_loaded: 0,
        total_assets: 5,
    });

    println!("=== ASSETS QUEUED FOR LOADING ===");
}

/// Displays loading screen
pub fn show_loading_screen(mut commands: Commands) {
    println!("=== SHOW_LOADING_SCREEN ===");

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.12).into()),
            LoadingScreenMarker,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Loading..."),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
                Node {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                },
            ));

            parent
                .spawn((
                    Node {
                        width: Val::Px(400.0),
                        height: Val::Px(30.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.2).into()),
                    BorderColor::all(Color::srgb(0.4, 0.4, 0.5)),
                ))
                .with_children(|bar_parent| {
                    bar_parent.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.7, 0.4).into()),
                        LoadingProgressBar,
                    ));
                });

            parent.spawn((
                Text::new("Preparing assets..."),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.8)),
                Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                LoadingStatusText,
            ));
        });

    println!("=== SHOW_LOADING_SCREEN COMPLETE ===");
}

/// Updates loading progress
pub fn check_loading_progress(
    asset_server: Res<AssetServer>,
    mut game_assets: ResMut<GameAssets>,
    mut progress: ResMut<LoadingProgress>,
    mut progress_bar_query: Query<&mut Node, With<LoadingProgressBar>>,
    mut status_text_query: Query<&mut Text, With<LoadingStatusText>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if game_assets.loading_complete {
        return;
    }

    // Check asset states
    let zombie_state = asset_server.get_load_state(&game_assets.zombie_model);
    let idle_state = asset_server.get_load_state(&game_assets.player_idle);
    let idle_nervous_state = asset_server.get_load_state(&game_assets.player_idle_nervous);
    let walking_state = asset_server.get_load_state(&game_assets.player_walking);
    let attacking_state = asset_server.get_load_state(&game_assets.player_attacking);

    let states = [
        zombie_state,
        idle_state,
        idle_nervous_state,
        walking_state,
        attacking_state,
    ];

    let mut loaded_count = 0;

    // Count loaded assets
    for state in states.iter() {
        if matches!(state, Some(LoadState::Loaded)) {
            loaded_count += 1;
        }
    }

    let current_status = if loaded_count == 0 {
        "Starting load...".to_string()
    } else {
        format!("Loading assets ({} / 5)...", loaded_count)
    };

    progress.assets_loaded = loaded_count;

    let progress_percent = if progress.total_assets > 0 {
        (progress.assets_loaded as f32 / progress.total_assets as f32) * 100.0
    } else {
        0.0
    };

    if let Ok(mut node) = progress_bar_query.single_mut() {
        node.width = Val::Percent(progress_percent);
    }

    if let Ok(mut text) = status_text_query.single_mut() {
        text.0 = current_status;
    }

    if progress.assets_loaded >= progress.total_assets {
        println!("=== ALL ASSETS LOADED, TRANSITIONING TO PLAYING ===");
        game_assets.loading_complete = true;
        next_state.set(AppState::Playing);
    }
}

/// Cleanup loading screen
pub fn cleanup_loading_screen(
    mut commands: Commands,
    loading_screen_query: Query<Entity, With<LoadingScreenMarker>>,
) {
    for entity in loading_screen_query.iter() {
        commands.entity(entity).despawn();
    }
}
