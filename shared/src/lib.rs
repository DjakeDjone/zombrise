#![allow(clippy::type_complexity)]

pub mod entity2;
pub mod players;
pub mod protocol;
pub mod shared;
#[cfg(all(feature = "client", not(target_arch = "wasm32")))]
pub mod suduxu;
pub mod zombie;
