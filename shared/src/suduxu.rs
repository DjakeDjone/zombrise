use bevy::prelude::*;
use std::ffi::{c_int, c_ushort};
use std::sync::Arc;
use std::thread;

// --- Enums ---
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SuduxuButton {
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
    A = 4,
    B = 5,
    X = 6,
    Y = 7,
    Minus = 8,
    Plus = 9,
    One = 10,
    Two = 11,
}

impl SuduxuButton {
    pub const ALL: [SuduxuButton; 12] = [
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
}

#[repr(i32)]
#[derive(Clone, Copy, Debug)]
pub enum ButtonInputState {
    Down = 0,
    #[allow(dead_code)]
    Up = 1,
}

// --- Dynamic Library Resource ---
#[derive(Resource, Clone)]
pub struct SuduxuLibrary {
    // We use Arc to make it cloneable for easy access,
    // though internally libloading::Library is thread-safe on unix.
    pub lib: Arc<libloading::Library>,
}

// Function pointer types
type StartSuduxuFn = unsafe extern "C" fn();
#[allow(dead_code)]
type StopSuduxuFn = unsafe extern "C" fn();
type IsRunningFn = unsafe extern "C" fn() -> bool;
type GetButtonInStateFn = unsafe extern "C" fn(c_ushort, c_int, c_int) -> bool;
type TickFn = unsafe extern "C" fn(f32);

// --- Dummy Stack Probe ---
// Fixes "undefined symbol: __rust_probestack" error if the library still tries to reference it.
#[no_mangle]
pub unsafe extern "C" fn __rust_probestack() {}

// --- Plugin ---
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
    println!("Suduxu: Attempting to load dynamic library...");

    // Try to find the library.
    // In dev: likely in project root or target/debug
    // We'll look in current directory (project root) first.
    let lib_path = "libsuduxu.so";

    let lib = unsafe { libloading::Library::new(lib_path) };

    match lib {
        Ok(library) => {
            println!("Suduxu: Library loaded successfully.");
            let lib = Arc::new(library);
            commands.insert_resource(SuduxuLibrary { lib: lib.clone() });

            // Start the server thread
            thread::spawn(move || unsafe {
                let start_func: libloading::Symbol<StartSuduxuFn> =
                    lib.get(b"start_suduxu").unwrap();
                println!("Suduxu: Thread starting suduxu server...");
                start_func();
                println!("Suduxu: Thread finished.");
            });
        }
        Err(e) => {
            eprintln!("Suduxu: Failed to load library '{}': {:?}", lib_path, e);
            // Optionally try absolute path if relative failed
            if let Ok(cwd) = std::env::current_dir() {
                let abs_path = cwd.join(lib_path);
                eprintln!("Suduxu: Attempting absolute path: {:?}", abs_path);
                let lib = unsafe { libloading::Library::new(abs_path) };
                match lib {
                    Ok(library) => {
                        println!("Suduxu: Library loaded successfully (absolute path).");
                        let lib = Arc::new(library);
                        commands.insert_resource(SuduxuLibrary { lib: lib.clone() });

                        // Start the server thread
                        thread::spawn(move || unsafe {
                            let start_func: libloading::Symbol<StartSuduxuFn> =
                                lib.get(b"start_suduxu").unwrap();
                            println!("Suduxu: Thread starting suduxu server...");
                            start_func();
                            println!("Suduxu: Thread finished.");
                        });
                    }
                    Err(e2) => {
                        eprintln!("Suduxu: Absolute path also failed: {:?}", e2);
                    }
                }
            }
        }
    }
}

fn update_suduxu_system(
    mut input: ResMut<ButtonInput<SuduxuButton>>,
    time: Res<Time>,
    library: Option<Res<SuduxuLibrary>>,
) {
    let Some(library) = library else {
        return;
    };

    // Tick
    unsafe {
        if let Ok(tick_func) = library.lib.get::<TickFn>(b"tick") {
            tick_func(time.delta_secs());
        }
    }

    // Process input
    input.clear();
    let client_id = 1; // Assuming client id 1

    if let Ok(get_button_func) = unsafe {
        library
            .lib
            .get::<GetButtonInStateFn>(b"get_button_in_state")
    } {
        for &btn in SuduxuButton::ALL.iter() {
            let is_down =
                unsafe { get_button_func(client_id, btn as i32, ButtonInputState::Down as i32) };

            if is_down {
                input.press(btn);
            } else {
                input.release(btn);
            }
        }
    }
}
