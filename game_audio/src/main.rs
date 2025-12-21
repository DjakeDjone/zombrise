mod audio_player;
mod game_song;
mod music_generation;

use audio_player::MusicPlayer;
use game_song::{create_zombie_song, Intensity};
use std::{path::Path, thread, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sf2_path = "assets/Chateau Grand-v1.8.sf2";
    if !Path::new(sf2_path).exists() {
        eprintln!("Error: SoundFont file not found at {}", sf2_path);
        return Ok(());
    }

    println!("Initializing Music Player...");
    let mut player = MusicPlayer::new(sf2_path)?;

    // Test different intensities
    let intensity = std::env::args()
        .nth(1)
        .map(|arg| match arg.to_lowercase().as_str() {
            "calm" => Intensity::Calm,
            "tension" => Intensity::Tension,
            "combat" => Intensity::Combat,
            _ => {
                println!(
                    "Unknown intensity '{}', using Calm. Options: calm, tension, combat",
                    arg
                );
                Intensity::Calm
            }
        })
        .unwrap_or(Intensity::Calm);

    println!("Creating Zombie Song with intensity: {:?}", intensity);
    let song = create_zombie_song(intensity);

    println!("Playing Song... (Press Ctrl+C to stop)");
    player.play_song(&song);

    // Wait for the song to play (8 measures at varying tempos)
    // Calm: 65 BPM, Tension: 90 BPM, Combat: 135 BPM
    // 8 measures * 4 beats = 32 beats
    let beats = 32.0;
    let tempo = match intensity {
        Intensity::Calm => 65.0,
        Intensity::Tension => 90.0,
        Intensity::Combat => 135.0,
    };
    let duration_secs = (beats / tempo) * 60.0;

    println!("Song duration: {:.1} seconds", duration_secs);
    thread::sleep(Duration::from_secs_f64(duration_secs + 1.0));

    println!("Done.");
    Ok(())
}
