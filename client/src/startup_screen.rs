use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::text::prelude::*;
use bevy::ui::prelude::*;
use bevy_simple_text_input::{
    TextInput, TextInputSettings, TextInputSubmitEvent, TextInputTextFont, TextInputValue, TextInputTextColor,
};

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
            // url: "138.199.203.159:5000".to_string(),
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
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.2).into()),
            StartupScreenMarker,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Zombrise 3D"),
                TextFont {
                    font_size: 60.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.8, 0.3)),
                Node {
                    margin: UiRect::bottom(Val::Px(50.0)),
                    ..default()
                },
            ));

            // Server URL label
            parent.spawn((
                Text::new("Server Address:"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            // Input box
            parent.spawn((
                Node {
                    width: Val::Px(400.0),
                    height: Val::Px(50.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(10.0)),
                    margin: UiRect::bottom(Val::Px(30.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.25).into()),
                BorderColor(Color::srgb(0.4, 0.4, 0.5).into()),
                TextInput,
                TextInputTextFont(TextFont {
                    font_size: 20.0,
                    ..default()
                }),
                TextInputTextColor(TextColor(Color::srgb(1.0, 1.0, 1.0))),
                TextInputValue(server_config.url.clone()),
                TextInputSettings {
                    retain_on_submit: true,
                    ..default()
                },
                ServerUrlInput,
            ));

            // Connect button
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(60.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.6, 0.2).into()),
                    ConnectButton,
                ))
                .with_children(|button_parent| {
                    button_parent.spawn((
                        Text::new("Connect"),
                        TextFont {
                            font_size: 30.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ));
                });
        });
}

pub fn cleanup_startup_screen(
    mut commands: Commands,
    startup_screen_query: Query<Entity, With<StartupScreenMarker>>,
) {
    for entity in startup_screen_query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn handle_startup_ui(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<ConnectButton>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
    mut server_config: ResMut<ServerConfig>,
    input_query: Query<&TextInputValue, With<ServerUrlInput>>,
    mut submit_events: EventReader<TextInputSubmitEvent>,
) {
    // Handle button interaction
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = Color::srgb(0.15, 0.5, 0.15).into();
                // Update server config from input before connecting
                if let Ok(input_value) = input_query.single() {
                    server_config.url = input_value.0.clone();
                }
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

    // Handle Enter key submission
    for event in submit_events.read() {
        if let Ok(input_value) = input_query.get(event.entity) {
            server_config.url = input_value.0.clone();
            next_state.set(AppState::Playing);
        }
    }
}

pub fn handle_copy_paste(
    mut input_query: Query<&mut TextInputValue, With<ServerUrlInput>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut evr_kbd: EventReader<KeyboardInput>,
) {
    // Check if Ctrl (or Cmd on Mac) is pressed
    let ctrl_pressed = keyboard_input.pressed(KeyCode::ControlLeft)
        || keyboard_input.pressed(KeyCode::ControlRight)
        || keyboard_input.pressed(KeyCode::SuperLeft)
        || keyboard_input.pressed(KeyCode::SuperRight);

    if !ctrl_pressed {
        return;
    }

    // Process keyboard events
    for ev in evr_kbd.read() {
        if !ev.state.is_pressed() {
            continue;
        }

        if let Ok(mut input_value) = input_query.single_mut() {
            match ev.key_code {
                // Copy: Ctrl+C
                KeyCode::KeyC => {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Err(e) = clipboard.set_text(&input_value.0) {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                }
                // Paste: Ctrl+V
                KeyCode::KeyV => {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            input_value.0 = text;
                        }
                    }
                }
                // Cut: Ctrl+X
                KeyCode::KeyX => {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Err(e) = clipboard.set_text(&input_value.0) {
                            eprintln!("Failed to cut to clipboard: {}", e);
                        } else {
                            input_value.0.clear();
                        }
                    }
                }
                // Select All: Ctrl+A (just for completeness, though selection isn't visible)
                KeyCode::KeyA => {
                    // The text input doesn't support visible selection,
                    // but we can at least acknowledge the shortcut
                }
                _ => {}
            }
        }
    }
}
