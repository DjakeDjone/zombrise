//! Fire particle effects for dying zombies and players.

use bevy::prelude::*;

use super::zombie_visuals::ZombieVisual;
use zombrise_shared::players::player::{Player, PlayerDying};
use zombrise_shared::zombie::zombie::{Zombie, ZombieDying, ZombieLink};

/// Marker component for fire particle entities
#[derive(Component)]
pub struct FireParticle {
    /// Lifetime remaining for this particle
    pub lifetime: f32,
    /// Velocity of the particle
    pub velocity: Vec3,
    /// Initial size
    pub initial_size: f32,
}

/// Marker for zombies that have fire spawned
#[derive(Component)]
pub struct ZombieFireSpawned;

/// Marker for players that have fire spawned
#[derive(Component)]
pub struct PlayerFireSpawned;

/// Resource to cache fire particle assets
#[derive(Resource)]
pub struct FireParticleAssets {
    pub mesh: Handle<Mesh>,
    pub material_orange: Handle<StandardMaterial>,
    pub material_yellow: Handle<StandardMaterial>,
    pub material_red: Handle<StandardMaterial>,
}

/// Sets up fire particle assets
pub fn setup_fire_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));

    let material_orange = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.4, 0.0, 0.9),
        emissive: LinearRgba::new(5.0, 2.0, 0.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let material_yellow = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.8, 0.0, 0.85),
        emissive: LinearRgba::new(5.0, 4.0, 0.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let material_red = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.1, 0.0, 0.8),
        emissive: LinearRgba::new(4.0, 0.5, 0.0, 1.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    commands.insert_resource(FireParticleAssets {
        mesh,
        material_orange,
        material_yellow,
        material_red,
    });
}

/// Spawns fire particles on dying zombies
pub fn spawn_zombie_fire(
    mut commands: Commands,
    zombie_visuals: Query<
        (Entity, &Transform, &ZombieLink),
        (With<ZombieVisual>, Without<ZombieFireSpawned>),
    >,
    dying_zombies: Query<&ZombieDying, With<Zombie>>,
    fire_assets: Option<Res<FireParticleAssets>>,
) {
    let Some(assets) = fire_assets else { return };

    for (visual_entity, transform, link) in &zombie_visuals {
        // Check if the linked zombie is dying
        let Ok(dying) = dying_zombies.get(link.0) else {
            continue;
        };

        // Only start fire during burn phase
        if dying.timer < dying.fall_duration {
            continue;
        }

        spawn_fire_burst(
            &mut commands,
            &assets,
            transform.translation,
            visual_entity,
            true,
        );
    }
}

/// Spawns fire particles on dying players
pub fn spawn_player_fire(
    mut commands: Commands,
    dying_players: Query<
        (Entity, &Transform, &PlayerDying),
        (With<Player>, Without<PlayerFireSpawned>),
    >,
    fire_assets: Option<Res<FireParticleAssets>>,
) {
    let Some(assets) = fire_assets else { return };

    for (entity, transform, dying) in &dying_players {
        // Only start fire during burn phase
        if dying.timer < dying.fall_duration {
            continue;
        }

        spawn_fire_burst(&mut commands, &assets, transform.translation, entity, false);
    }
}

fn spawn_fire_burst(
    commands: &mut Commands,
    assets: &FireParticleAssets,
    position: Vec3,
    parent_entity: Entity,
    is_zombie: bool,
) {
    // Mark as having fire spawned
    if is_zombie {
        commands.entity(parent_entity).insert(ZombieFireSpawned);
    } else {
        commands.entity(parent_entity).insert(PlayerFireSpawned);
    }

    // Spawn initial burst of fire particles
    let rng = || fastrand::f32();

    for _ in 0..15 {
        let offset = Vec3::new((rng() - 0.5) * 0.8, rng() * 0.5, (rng() - 0.5) * 0.8);
        let velocity = Vec3::new((rng() - 0.5) * 0.5, 1.0 + rng() * 2.0, (rng() - 0.5) * 0.5);
        let size = 0.15 + rng() * 0.25;
        let lifetime = 0.5 + rng() * 1.0;

        // Choose random fire color
        let material = match (rng() * 3.0) as u32 {
            0 => assets.material_orange.clone(),
            1 => assets.material_yellow.clone(),
            _ => assets.material_red.clone(),
        };

        commands.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position + offset).with_scale(Vec3::splat(size)),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            FireParticle {
                lifetime,
                velocity,
                initial_size: size,
            },
            Name::new("FireParticle"),
        ));
    }
}

/// Continuously spawn fire particles on burning zombies
pub fn update_zombie_fire(
    mut commands: Commands,
    zombie_visuals: Query<(&Transform, &ZombieLink), (With<ZombieVisual>, With<ZombieFireSpawned>)>,
    dying_zombies: Query<&ZombieDying, With<Zombie>>,
    fire_assets: Option<Res<FireParticleAssets>>,
    time: Res<Time>,
) {
    let Some(assets) = fire_assets else { return };

    for (transform, link) in &zombie_visuals {
        // Check if the linked zombie is dying
        let Ok(dying) = dying_zombies.get(link.0) else {
            continue;
        };

        // Only during burn phase
        if dying.timer < dying.fall_duration {
            continue;
        }

        // Spawn new particles periodically (roughly 10-15 per second)
        if fastrand::f32() < time.delta_secs() * 12.0 {
            spawn_continuous_fire(&mut commands, &assets, transform.translation);
        }
    }
}

/// Continuously spawn fire particles on burning players
pub fn update_player_fire(
    mut commands: Commands,
    dying_players: Query<(&Transform, &PlayerDying), (With<Player>, With<PlayerFireSpawned>)>,
    fire_assets: Option<Res<FireParticleAssets>>,
    time: Res<Time>,
) {
    let Some(assets) = fire_assets else { return };

    for (transform, dying) in &dying_players {
        // Only during burn phase
        if dying.timer < dying.fall_duration {
            continue;
        }

        // Spawn new particles periodically (roughly 10-15 per second)
        if fastrand::f32() < time.delta_secs() * 12.0 {
            spawn_continuous_fire(&mut commands, &assets, transform.translation);
        }
    }
}

fn spawn_continuous_fire(commands: &mut Commands, assets: &FireParticleAssets, position: Vec3) {
    let rng = || fastrand::f32();

    let offset = Vec3::new((rng() - 0.5) * 0.6, rng() * 0.3, (rng() - 0.5) * 0.6);
    let velocity = Vec3::new((rng() - 0.5) * 0.3, 0.8 + rng() * 1.5, (rng() - 0.5) * 0.3);
    let size = 0.1 + rng() * 0.2;
    let lifetime = 0.4 + rng() * 0.8;

    let material = match (rng() * 3.0) as u32 {
        0 => assets.material_orange.clone(),
        1 => assets.material_yellow.clone(),
        _ => assets.material_red.clone(),
    };

    commands.spawn((
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(position + offset).with_scale(Vec3::splat(size)),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
        FireParticle {
            lifetime,
            velocity,
            initial_size: size,
        },
        Name::new("FireParticle"),
    ));
}

/// Animate fire particles - rise up, flicker, and fade
pub fn animate_fire_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut FireParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let elapsed = time.elapsed_secs();

    for (entity, mut transform, mut particle) in query.iter_mut() {
        particle.lifetime -= dt;

        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Move upward with slight turbulence
        transform.translation += particle.velocity * dt;

        // Add flickering motion
        let flicker_x = (elapsed * 8.0 + transform.translation.x * 5.0).sin() * 0.02;
        let flicker_z = (elapsed * 7.0 + transform.translation.z * 5.0).cos() * 0.02;
        transform.translation.x += flicker_x;
        transform.translation.z += flicker_z;

        // Slow down upward velocity over time
        particle.velocity.y *= 0.98;
        particle.velocity.x *= 0.95;
        particle.velocity.z *= 0.95;

        // Scale down as lifetime decreases (fade effect)
        let life_ratio = particle.lifetime / 1.0;
        let scale = particle.initial_size * life_ratio.max(0.1);
        transform.scale = Vec3::splat(scale);
    }
}

/// Handle dying zombie visual effects
pub fn update_dying_zombie_visuals(
    zombie_logic_query: Query<&ZombieDying, With<Zombie>>,
    visual_query: Query<(Entity, &ZombieLink), With<ZombieVisual>>,
) {
    for (_visual_entity, link) in &visual_query {
        if let Ok(dying) = zombie_logic_query.get(link.0) {
            let burn_progress = if dying.timer > dying.fall_duration {
                (dying.timer - dying.fall_duration) / dying.burn_duration
            } else {
                0.0
            };

            // Could add material darkening here if needed
            let _ = burn_progress;
        }
    }
}
