use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use suduxu_rs::Suduxu;

fn main() {
    println!("Initializing Suduxu Sensor Monitor...");

    // Initialize library
    let suduxu = match Suduxu::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load Suduxu library: {}", e);
            std::process::exit(1);
        }
    };

    // Register sensor callback
    suduxu.set_sensor_callback(|data| {
        // Since we don't know the fields of SensorDataRaw yet, we print the raw bytes debug info.
        // It's a [u8; 64], so it will print as an array of bytes.
        println!("Received Sensor Data: {:?}", data);
    });

    // Also register event callback to see what's happening
    suduxu.set_event_callback(|event| {
        println!("Received Event: {}", event);
    });

    // Start the server
    println!("Starting Suduxu server...");
    suduxu.start();

    if suduxu.is_running() {
        println!("Suduxu server is running. Press Ctrl+C to stop.");
    } else {
        eprintln!("Warning: Server might not have started correctly.");
    }

    // Keep the main thread alive.
    // We use a simple loop handle Ctrl+C gracefullness if we wanted,
    // but for this simple test, a loop with sleep is fine.

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set a ctrl-c handler to exit cleanly
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        // Tick the server if necessary?
        // The C# code called `tick(Time.deltaTime)`.
        // If the `start_suduxu` starts a background thread that does the ticking, we might not need to call `tick`.
        // However, `suduxu_demo.cs` calls `tick` in `Update()`.
        // IF `start_suduxu` runs the NETWORK loop, `tick` might run the LOGIC loop.
        // Let's call tick just in case, similar to C# Update loop.
        // Assuming 60Hz.

        suduxu.tick(1.0 / 60.0);

        thread::sleep(Duration::from_millis(16));
    }

    println!("\nStopping Suduxu server...");
    suduxu.stop();
    println!("Exited cleanly.");
}
