#![allow(clippy::type_complexity)]
//! Server main for Zombrise using Lightyear 0.25 networking

use bevy::prelude::*;
use bevy::{
    app::ScheduleRunnerPlugin, asset::AssetPlugin, scene::ScenePlugin, state::app::StatesPlugin,
};
use std::time::Duration;

use avian3d::prelude::*;
use lightyear::prelude::server::*;

use zombrise_shared::shared::SharedPlugin;

mod systems;

use systems::{
    combat::{apply_pending_player_damage, handle_player_attack, zombie_collision_damage},
    networking::{despawn_clients, setup_networking, spawn_clients},
    player::{
        detect_player_death, passive_health_regeneration, update_attack_cooldown,
        update_damage_flash, update_dying_players,
    },
    world::{cleanup_wandering_zombies, remove_fallen_entities, setup_server, update_map_size},
    zombie::{update_dying_zombies, update_zombie_damage_flash},
    zombie_ai::{spawn_zombies, zombie_movement, ZombieSpawnTimer},
};

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .add_plugins(AssetPlugin::default())
        .add_plugins(bevy::log::LogPlugin {
            level: bevy::log::Level::INFO,
            filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
            ..default()
        })
        .add_plugins(ScenePlugin)
        .add_plugins(StatesPlugin)
        .add_plugins(bevy_mesh::MeshPlugin) // Required for Avian3D
        .add_plugins(ServerPlugins::default()) // Lightyear ServerPlugins
        .add_plugins(SharedPlugin)
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(ZombieSpawnTimer::default())
        .add_observer(spawn_clients)
        .add_observer(despawn_clients)
        .add_systems(Startup, (setup_networking, setup_server).chain())
        .add_systems(
            FixedUpdate,
            (
                handle_player_attack,
                apply_pending_player_damage,
                zombie_movement,
                zombie_collision_damage,
                update_damage_flash,
                update_zombie_damage_flash,
                update_attack_cooldown,
                update_dying_zombies,
                detect_player_death,
                update_dying_players,
                remove_fallen_entities,
                cleanup_wandering_zombies,
                passive_health_regeneration,
            ),
        )
        .add_systems(Update, (update_map_size, spawn_zombies))
        .run();
}
