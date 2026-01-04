use crate::death_screen::PlayerDied;
use crate::startup_screen::AppState;
use bevy::prelude::*;
use game_audio::audio_player::MusicPlayer;
use game_audio::game_song::{create_zombie_song, Intensity};
use std::time::Duration;
use zombrise_shared::players::player::{MyClientId, Player, PlayerOwner};
use zombrise_shared::players::player_animation::PlayerAttacking;
use zombrise_shared::zombie::zombie::Zombie;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AudioState::default())
            .add_systems(Startup, setup_audio_nonsend)
            .add_systems(Update, (update_music_state, manage_music_playback));
    }
}

pub struct AudioSystem {
    pub player: MusicPlayer,
    pub current_intensity: Intensity,
    pub timer: Timer,
}

#[derive(Resource, Default)]
struct AudioState {
    target_intensity: Option<Intensity>,
    time_since_last_check: Timer,
}

pub fn setup_audio_nonsend(world: &mut World) {
    let music_file_name = "Chateau Grand-v1.8.sf2";

    let mut sf2_path = format!("assets/{}", music_file_name);

    if !std::path::Path::new(&sf2_path).exists() {
        let workspace_path = format!("client/assets/{}", music_file_name);
        if std::path::Path::new(&workspace_path).exists() {
            sf2_path = workspace_path;
        } else {
            let install_path = format!("/usr/share/zombrise/assets/{}", music_file_name);
            if std::path::Path::new(&install_path).exists() {
                sf2_path = install_path;
            } else if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let exe_asset = exe_dir.join("assets").join(music_file_name);
                    if exe_asset.exists() {
                        sf2_path = exe_asset.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    if !std::path::Path::new(&sf2_path).exists() {
        eprintln!("WARNING: SoundFont not found. Audio will not play. Searched: assets/{}, client/assets/{}, and /usr/share/zombrise/assets/{}", music_file_name, music_file_name, music_file_name);
    }

    match MusicPlayer::new(&sf2_path) {
        Ok(player) => {
            world.insert_non_send_resource(AudioSystem {
                player,
                current_intensity: Intensity::Calm,
                timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            });
        }
        Err(e) => eprintln!("Failed to initialize audio: {}", e),
    }
}

fn update_music_state(
    time: Res<Time>,
    mut audio_state: ResMut<AudioState>,
    player_query: Query<(&GlobalTransform, &PlayerOwner, Option<&PlayerAttacking>), With<Player>>,
    zombie_query: Query<&GlobalTransform, With<Zombie>>,
    my_client_id: Option<Res<MyClientId>>,
    player_died: Res<PlayerDied>,
    state: Res<State<AppState>>,
) {
    audio_state.time_since_last_check.tick(time.delta());

    if audio_state.time_since_last_check.is_finished()
        || audio_state.time_since_last_check.elapsed().as_secs_f32() == 0.0
    {
        if audio_state.time_since_last_check.duration() == Duration::ZERO {
            audio_state.time_since_last_check = Timer::from_seconds(1.0, TimerMode::Repeating);
        }

        if *state.get() == AppState::StartupScreen || player_died.0 || my_client_id.is_none() {
            audio_state.target_intensity = Some(Intensity::Calm);
            return;
        }

        let client_id = my_client_id.as_ref().unwrap();
        let mut my_pos = Vec3::ZERO;
        let mut found_me = false;

        let mut am_attacking = false;

        for (transform, owner, attacking) in player_query.iter() {
            if owner.0 == client_id.0 {
                my_pos = transform.translation();
                found_me = true;
                if let Some(attack) = attacking {
                    if attack.attack_timer > 0.0 {
                        am_attacking = true;
                    }
                }
                break;
            }
        }

        if found_me {
            let mut close_zombies = 0;
            for z_transform in zombie_query.iter() {
                if z_transform.translation().distance(my_pos) < 15.0 {
                    close_zombies += 1;
                }
            }

            let new_intensity = if am_attacking || close_zombies > 2 {
                Intensity::Combat
            } else if close_zombies > 0 {
                Intensity::Tension
            } else {
                Intensity::Calm
            };

            audio_state.target_intensity = Some(new_intensity);
        }
    }
}

fn manage_music_playback(
    time: Res<Time>,
    audio_state: Res<AudioState>,
    audio_system: Option<NonSendMut<AudioSystem>>,
) {
    if let Some(mut system) = audio_system {
        system.timer.tick(time.delta());

        let mut changed = false;
        if let Some(target) = audio_state.target_intensity {
            if target != system.current_intensity {
                system.current_intensity = target;
                changed = true;
            }
        }

        if changed {
            // Loop
            let song = create_zombie_song(system.current_intensity);
            system.player.play_song(&song);
            // Reset timer
            system.timer.set_duration(Duration::from_secs(15));
            system.timer.reset();
        } else if system.timer.is_finished() {
            // Loop
            let song = create_zombie_song(system.current_intensity);
            system.player.play_song(&song);
            system.timer.set_duration(Duration::from_secs(15));
            system.timer.reset();
        }
    }
}
