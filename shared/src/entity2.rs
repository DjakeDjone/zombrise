use bevy::{ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Reflect, Clone, PartialEq)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
        }
    }
}
