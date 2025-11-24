use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use zombrise_shared::players::player::{Health, Player, PlayerOwner};

#[derive(Resource, Default)]
pub struct PlayerDied(pub bool);

#[derive(Component)]
pub struct DeathScreenMarker;

/// Detects when the player's health reaches zero or when the player entity is despawned
pub fn detect_player_death(
    player_query: Query<(&Health, &PlayerOwner), With<Player>>,
    client_id: Res<crate::MyClientId>,
    mut player_died: ResMut<PlayerDied>,
) {
    // Try to find our player
    let our_player = player_query.iter().find(|(_, owner)| owner.0.get() == client_id.0);
    
    if let Some((health, _)) = our_player {
        // Player exists - check health status
        if health.current <= 0.0 && !player_died.0 {
            player_died.0 = true;
            info!("Player died - health reached zero!");
        } else if health.current > 0.0 && player_died.0 {
            // Player has respawned or reconnected with health
            player_died.0 = false;
            info!("Player respawned!");
        }
    } else {
        // Player entity not found - they were despawned from the server
        if !player_died.0 {
            player_died.0 = true;
            info!("Player died - entity was despawned from server!");
        }
    }
}

/// Shows the death screen overlay when the player dies
pub fn show_death_screen(
    mut commands: Commands,
    player_died: Res<PlayerDied>,
    death_screen_query: Query<Entity, With<DeathScreenMarker>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if player_died.0 && death_screen_query.is_empty() {
        // Unlock cursor when dead
        if let Ok(mut window) = window_query.get_single_mut() {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }

        // Spawn death screen UI
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_direction: FlexDirection::Column,
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    background_color: Color::srgba(0.0, 0.0, 0.0, 0.85).into(),
                    z_index: ZIndex::Global(1000),
                    ..default()
                },
                DeathScreenMarker,
            ))
            .with_children(|parent| {
                // "YOU DIED" text with shadow effect
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Relative,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "YOU DIED",
                            TextStyle {
                                font_size: 100.0,
                                color: Color::srgb(0.9, 0.1, 0.1),
                                ..default()
                            },
                        ));
                    });

                // Subtitle text
                parent.spawn(
                    TextBundle::from_section(
                        "The zombies got you...",
                        TextStyle {
                            font_size: 32.0,
                            color: Color::srgb(0.85, 0.85, 0.85),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::top(Val::Px(30.0)),
                        ..default()
                    }),
                );

                // Info text
                parent.spawn(
                    TextBundle::from_section(
                        "Press ESC to return to menu",
                        TextStyle {
                            font_size: 24.0,
                            color: Color::srgb(0.6, 0.6, 0.6),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::top(Val::Px(50.0)),
                        ..default()
                    }),
                );
            });
    } else if !player_died.0 && !death_screen_query.is_empty() {
        // Player has respawned - clean up death screen
        for entity in death_screen_query.iter() {
            commands.entity(entity).despawn_recursive();
        }
        
        // Re-lock cursor
        if let Ok(mut window) = window_query.get_single_mut() {
            window.cursor.grab_mode = CursorGrabMode::Locked;
            window.cursor.visible = false;
        }
    }
}

/// Allows the player to return to the startup screen by pressing ESC
pub fn handle_death_screen_input(
    mut next_state: ResMut<NextState<crate::startup_screen::AppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    death_screen_query: Query<Entity, With<DeathScreenMarker>>,
    mut player_died: ResMut<PlayerDied>,
) {
    if player_died.0 && keys.just_pressed(KeyCode::Escape) {
        // Clean up death screen
        for entity in death_screen_query.iter() {
            commands.entity(entity).despawn_recursive();
        }

        // Reset death state
        player_died.0 = false;

        // Return to startup screen
        next_state.set(crate::startup_screen::AppState::StartupScreen);
    }
}
