//! Protocol definition for Lightyear networking.
//! Defines inputs, components, channels, and messages.

use avian3d::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

use crate::entity2::Health;
use crate::players::player::{DamageFlash, Player, PlayerOwner};
use crate::shared::{MapMarker, TreeMarker};
use crate::zombie::zombie::{Zombie, ZombieAnimationState, ZombieDamageFlash, ZombieDying};

// ============== INPUTS ==============

/// Game inputs sent from client to server
#[derive(Serialize, Deserialize, Debug, Clone, Reflect, Default)]
#[reflect(PartialEq, Hash)]
pub enum GameInput {
    Move {
        direction: Vec2, // x, z movement direction
        yaw: f32,        // camera yaw for rotation
    },
    Attack,
    #[default]
    None,
}

// Manual PartialEq implementation for floats
impl PartialEq for GameInput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                GameInput::Move {
                    direction: d1,
                    yaw: y1,
                },
                GameInput::Move {
                    direction: d2,
                    yaw: y2,
                },
            ) => {
                d1.x.to_bits() == d2.x.to_bits()
                    && d1.y.to_bits() == d2.y.to_bits()
                    && y1.to_bits() == y2.to_bits()
            }
            (GameInput::Attack, GameInput::Attack) => true,
            (GameInput::None, GameInput::None) => true,
            _ => false,
        }
    }
}

// Manual Eq implementation
impl Eq for GameInput {}

// Manual Hash implementation since Vec2 and f32 don't implement Hash
impl std::hash::Hash for GameInput {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            GameInput::Move { direction, yaw } => {
                0u8.hash(state);
                direction.x.to_bits().hash(state);
                direction.y.to_bits().hash(state);
                yaw.to_bits().hash(state);
            }
            GameInput::Attack => 1u8.hash(state),
            GameInput::None => 2u8.hash(state),
        }
    }
}

// Required for Lightyear input system
impl MapEntities for GameInput {
    fn map_entities<M: EntityMapper>(&mut self, _entity_mapper: &mut M) {}
}

// ============== CHANNELS ==============

/// Main game channel for reliable ordered messages
pub struct GameChannel;

// ============== PROTOCOL PLUGIN ==============

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        // Register input plugin
        app.add_plugins(input::native::InputPlugin::<GameInput>::default());

        // Register components for replication
        // Player is predicted
        app.register_component::<Player>().add_prediction();

        app.register_component::<PlayerOwner>().add_prediction();

        app.register_component::<Health>();
        app.register_component::<DamageFlash>();
        app.register_component::<Zombie>();
        app.register_component::<ZombieDamageFlash>();
        app.register_component::<ZombieDying>();
        app.register_component::<ZombieAnimationState>();
        app.register_component::<MapMarker>();
        app.register_component::<TreeMarker>();

        // Transform MUST be registered for replication in Lightyear 0.25
        // Enable prediction for Transform to avoid jitter for predicted entities (Player)
        app.register_component::<Transform>().add_prediction();
        app.register_component::<GlobalTransform>();

        // Register physics components for prediction
        app.register_component::<LinearVelocity>().add_prediction();
        app.register_component::<AngularVelocity>().add_prediction();
        app.register_component::<RigidBody>();
        app.register_component::<Friction>();
        app.register_component::<Restitution>();
        app.register_component::<LinearDamping>();
        app.register_component::<AngularDamping>();
        app.register_component::<GravityScale>();
        app.register_component::<Position>().add_prediction();
        app.register_component::<Rotation>().add_prediction();

        // Register channel
        app.add_channel::<GameChannel>(ChannelSettings {
            mode: ChannelMode::OrderedReliable(ReliableSettings::default()),
            ..default()
        })
        .add_direction(NetworkDirection::ServerToClient);
    }
}
