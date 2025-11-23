use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    StartupScreen,
    Playing,
}

#[derive(Resource)]
pub struct ServerConfig {
    pub url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: "127.0.0.1:5000".to_string(),
        }
    }
}

#[derive(Component)]
pub struct StartupScreenMarker;

#[derive(Component)]
pub(crate) struct ServerUrlInput;

#[derive(Component)]
pub(crate) struct ConnectButton;

pub fn show_startup_screen(mut commands: Commands, server_config: Res<ServerConfig>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: Color::srgb(0.15, 0.15, 0.2).into(),
                ..default()
            },
            StartupScreenMarker,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(
                TextBundle::from_section(
                    "Dragon Queen 3D",
                    TextStyle {
                        font_size: 60.0,
                        color: Color::srgb(0.9, 0.8, 0.3),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::bottom(Val::Px(50.0)),
                    ..default()
                }),
            );

            // Server URL label
            parent.spawn(
                TextBundle::from_section(
                    "Server Address:",
                    TextStyle {
                        font_size: 24.0,
                        color: Color::srgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                }),
            );

            // Input box container
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(400.0),
                            height: Val::Px(50.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            padding: UiRect::all(Val::Px(10.0)),
                            margin: UiRect::bottom(Val::Px(30.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.2, 0.25).into(),
                        border_color: Color::srgb(0.4, 0.4, 0.5).into(),
                        ..default()
                    },
                    ServerUrlInput,
                ))
                .with_children(|input_parent| {
                    input_parent.spawn(TextBundle::from_section(
                        server_config.url.clone(),
                        TextStyle {
                            font_size: 20.0,
                            color: Color::srgb(1.0, 1.0, 1.0),
                            ..default()
                        },
                    ));
                });

            // Connect button
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(200.0),
                            height: Val::Px(60.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.2, 0.6, 0.2).into(),
                        ..default()
                    },
                    ConnectButton,
                ))
                .with_children(|button_parent| {
                    button_parent.spawn(TextBundle::from_section(
                        "Connect",
                        TextStyle {
                            font_size: 30.0,
                            color: Color::srgb(1.0, 1.0, 1.0),
                            ..default()
                        },
                    ));
                });
        });
}

pub fn cleanup_startup_screen(
    mut commands: Commands,
    startup_screen_query: Query<Entity, With<StartupScreenMarker>>,
) {
    for entity in startup_screen_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn handle_startup_ui(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ConnectButton>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut server_config: ResMut<ServerConfig>,
    input_query: Query<&Children, With<ServerUrlInput>>,
    mut text_query: Query<&mut Text>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Handle button interaction
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.15, 0.5, 0.15).into();
                next_state.set(AppState::Playing);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.25, 0.7, 0.25).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.2, 0.6, 0.2).into();
            }
        }
    }

    // Handle keyboard input for server URL
    if let Ok(children) = input_query.get_single() {
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                // Handle backspace
                if keyboard_input.just_pressed(KeyCode::Backspace) {
                    server_config.url.pop();
                    text.sections[0].value = server_config.url.clone();
                }

                // Handle enter to connect
                if keyboard_input.just_pressed(KeyCode::Enter) {
                    next_state.set(AppState::Playing);
                }

                // Handle character input
                for key in keyboard_input.get_just_pressed() {
                    if let Some(character) = key_to_char(
                        *key,
                        keyboard_input.pressed(KeyCode::ShiftLeft)
                            || keyboard_input.pressed(KeyCode::ShiftRight),
                    ) {
                        server_config.url.push(character);
                        text.sections[0].value = server_config.url.clone();
                    }
                }
            }
        }
    }
}

fn key_to_char(key: KeyCode, shift: bool) -> Option<char> {
    match key {
        KeyCode::Digit0 => Some(if shift { ')' } else { '0' }),
        KeyCode::Digit1 => Some(if shift { '!' } else { '1' }),
        KeyCode::Digit2 => Some(if shift { '@' } else { '2' }),
        KeyCode::Digit3 => Some(if shift { '#' } else { '3' }),
        KeyCode::Digit4 => Some(if shift { '$' } else { '4' }),
        KeyCode::Digit5 => Some(if shift { '%' } else { '5' }),
        KeyCode::Digit6 => Some(if shift { '^' } else { '6' }),
        KeyCode::Digit7 => Some(if shift { '&' } else { '7' }),
        KeyCode::Digit8 => Some(if shift { '*' } else { '8' }),
        KeyCode::Digit9 => Some(if shift { '(' } else { '9' }),
        KeyCode::Period => Some(if shift { '>' } else { '.' }),
        KeyCode::Semicolon => Some(if shift { ':' } else { ';' }),
        _ => None,
    }
}
