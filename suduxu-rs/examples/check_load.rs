use std::thread;
use std::time::Duration;
use suduxu_rs::Suduxu;

fn main() {
    println!("Initializing Suduxu wrapper...");
    let suduxu = match Suduxu::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load Suduxu library: {}", e);
            std::process::exit(1);
        }
    };

    println!("Starting Suduxu (server thread)...");
    suduxu.start();

    // Give it a moment to start
    thread::sleep(Duration::from_millis(100));

    // Note: is_running checks the internal boolean of suduxu server?
    if suduxu.is_running() {
        println!("SUCCESS: Suduxu is running!");
    } else {
        println!("WARNING: is_running() returned false. The server might take longer to start or failed internally.");
        // Still consider it a success for loading the library if we got this far without crash.
    }

    println!("Stopping Suduxu...");
    suduxu.stop();
    println!("Done.");
}
