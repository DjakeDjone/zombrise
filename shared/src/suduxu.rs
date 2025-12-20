use bevy::prelude::*;
use std::sync::Arc;
pub use suduxu_rs::ButtonInputType as SuduxuButton;
use suduxu_rs::{ButtonInputState, Suduxu};

#[derive(Resource, Clone)]
pub struct SuduxuResource(pub Arc<Suduxu>);

pub struct SuduxuPlugin;

impl Plugin for SuduxuPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<ButtonInput<SuduxuButton>>() {
            app.init_resource::<ButtonInput<SuduxuButton>>();
        }
        app.add_systems(Startup, startup_suduxu_system)
            .add_systems(Update, update_suduxu_system);
    }
}

fn startup_suduxu_system(mut commands: Commands) {
    println!("Suduxu: Initializing...");

    match Suduxu::new() {
        Ok(suduxu) => {
            println!("Suduxu: Library loaded successfully.");
            suduxu.start();
            commands.insert_resource(SuduxuResource(Arc::new(suduxu)));
        }
        Err(e) => {
            eprintln!("Suduxu: Failed to load library: {}", e);
        }
    }
}

fn update_suduxu_system(
    mut input: ResMut<ButtonInput<SuduxuButton>>,
    time: Res<Time>,
    suduxu: Option<Res<SuduxuResource>>,
) {
    let Some(suduxu) = suduxu else {
        return;
    };

    suduxu.0.tick(time.delta_secs());
    input.clear();

    // Find first active client
    let mut client_id = 0;
    for id in 1..=4 {
        let client_info = suduxu.0.find_client_by_id(id);
        if !client_info.is_empty() {
            client_id = id;
            // println!("Using Suduxu Client ID: {}", client_id); // Debug
            break;
        }
    }

    if client_id == 0 {
        return;
    }

    let all_buttons = [
        SuduxuButton::Up,
        SuduxuButton::Right,
        SuduxuButton::Down,
        SuduxuButton::Left,
        SuduxuButton::A,
        SuduxuButton::B,
        SuduxuButton::X,
        SuduxuButton::Y,
        SuduxuButton::Minus,
        SuduxuButton::Plus,
        SuduxuButton::One,
        SuduxuButton::Two,
    ];

    for &btn in all_buttons.iter() {
        if suduxu.0.get_button(client_id, btn, ButtonInputState::Down) {
            println!("Suduxu Button Pressed: {:?}", btn);
            input.press(btn);
        } else {
            input.release(btn);
        }
    }
}
