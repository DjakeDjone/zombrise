use bevy::asset::LoadState;
use bevy::gltf::Gltf;
use bevy::prelude::*;

use crate::startup_screen::AppState;

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
    let zombie_model: Handle<Gltf> = asset_server.load("zombie.glb");

    commands.insert_resource(GameAssets {
        zombie_model,
        loading_complete: false,
    });

    commands.insert_resource(LoadingProgress {
        assets_loaded: 0,
        total_assets: 1,
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

    let mut loaded_count = 0;
    let current_status: &str;

    match zombie_state {
        Some(LoadState::Loaded) => {
            loaded_count += 1;
            current_status = "Zombie model loaded!";
        }
        Some(LoadState::Loading) => {
            current_status = "Loading zombie model...";
        }
        Some(LoadState::Failed(_)) => {
            current_status = "Failed to load zombie model!";
        }
        Some(LoadState::NotLoaded) => {
            current_status = "Waiting for zombie model...";
        }
        None => {
            current_status = "Initializing...";
        }
    }

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
        text.0 = current_status.to_string();
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
