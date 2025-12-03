use bevy::prelude::*;
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
    let our_player = player_query
        .iter()
        .find(|(_, owner)| owner.0 == client_id.0);

    if let Some((health, _)) = our_player {
        if health.current <= 0.0 && !player_died.0 {
            player_died.0 = true;
        } else if health.current > 0.0 && player_died.0 {
            player_died.0 = false;
        }
    } else {
        if !player_died.0 {
            player_died.0 = true;
        }
    }
}

/// Shows the death screen overlay when the player dies
pub fn show_death_screen(
    mut commands: Commands,
    player_died: Res<PlayerDied>,
    death_screen_query: Query<Entity, With<DeathScreenMarker>>,
    health_ui_query: Query<Entity, With<crate::HealthBarUI>>,
) {
    if player_died.0 && death_screen_query.is_empty() {
        // Clean up health bar UI when showing death screen
        for entity in health_ui_query.iter() {
            commands.entity(entity).despawn();
        }

        // Unlock cursor when dead
        // if let Ok(mut window) = window_query.single_mut() {
        //     window.cursor.grab_mode = CursorGrabMode::None;
        //     window.cursor.visible = true;
        // }

        // Spawn death screen UI
        commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    position_type: PositionType::Absolute,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85).into()),
                ZIndex(1000),
                DeathScreenMarker,
            ))
            .with_children(|parent| {
                parent
                    .spawn(Node {
                        position_type: PositionType::Relative,
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn((
                            Text::new("YOU DIED"),
                            TextFont {
                                font_size: 100.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.9, 0.1, 0.1)),
                        ));
                    });

                // Subtitle text
                parent.spawn((
                    Text::new("The zombies got you..."),
                    TextFont {
                        font_size: 32.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                    Node {
                        margin: UiRect::top(Val::Px(30.0)),
                        ..default()
                    },
                ));

                // Info text
                parent.spawn((
                    Text::new("Press ESC to return to menu"),
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.6, 0.6)),
                    Node {
                        margin: UiRect::top(Val::Px(50.0)),
                        ..default()
                    },
                ));
            });
    } else if !player_died.0 && !death_screen_query.is_empty() {
        // Player has respawned - clean up death screen
        for entity in death_screen_query.iter() {
            commands.entity(entity).despawn();
        }

        // Re-lock cursor
        // if let Ok(mut window) = window_query.single_mut() {
        //     window.cursor.grab_mode = CursorGrabMode::Locked;
        //     window.cursor.visible = false;
        // }
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
            commands.entity(entity).despawn();
        }

        // Reset death state
        player_died.0 = false;

        // Return to startup screen
        next_state.set(crate::startup_screen::AppState::StartupScreen);
    }
}
