---
name: bevy-017-expert
description: Use this skill when writing, refactoring, or debugging Rust code for the Bevy 0.17 game engine. It contains standards for Observers, StateScoped entities, and UI rendering.
---

# Bevy 0.17 Development Mission
Your goal is to implement high-performance, idiomatic Bevy code using the 0.17+ standards.

## Key Instructions

### Observers
Always prefer the `On<Event>` syntax over manual `Trigger` queries.
**Example:** `app.observe(|trigger: On<MyEvent>| { ... });`

### Cleanup
Never write manual cleanup systems for game states. Use `StateScoped(GameState::Variant)` and ensure `app.enable_state_scoped_entities::<GameState>()` is called.

### UI Rendering
If generating UI code, remind the user that `bevy_ui_render` must be enabled in `Cargo.toml`.

### Boilerplate
Organize logic into Plugins.

## Standards Reference

### Modern Bevy (0.17+) Development Standards

#### 1. Core Paradigm: ECS & Observers
- **Components**: Small, data-only structs. Use `#[derive(Component)]`.
- **Reflect Auto-Registration (0.17)**: Types deriving `Reflect` no longer require manual registration in the app via `.register_type::<T>()` in most common cases.
- **Observers (The On Syntax)**: 0.17 formalizes the `On<Event>` syntax for observers.
  - **Pattern**: `commands.trigger(MyEvent);`
  - **Listener**: `app.observe(|trigger: On<MyEvent>| { ... });`
  - **Targeted**: `app.observe(|trigger: On<MyEvent, MyComponent>| { ... });`
- **Entity Relationships**: Use Entity IDs for links. In 0.17, `commands.entity(parent).add_child(child)` remains the standard for hierarchies.

#### 2. States & Transitions
- **Same-State Transitions (0.17)**: You can now trigger transitions to the state the app is already in, which is useful for "restarting" levels or resetting state-scoped logic.
- **State-Scoped Entities**: Tag entities with `StateScoped(GameState::Playing)`. Enable with `app.enable_state_scoped_entities::<GameState>();`.
- **Computed States**: Use for hierarchical state logic (e.g., `InMenu` is true if state is `MainMenu` OR `Settings`).

#### 3. UI Development & Rendering
- **Feature Flags (Critical)**: In 0.17, the `bevy_ui` feature no longer implies rendering. If using `default-features = false`, you must include both `bevy_ui` and `bevy_ui_render`.
- **Headless Widgets (0.17)**: Experimental support for Button, Slider, and Checkbox that emit events (Activate, ValueChange) via the observer system.
- **Gradients & Borders**: Bevy UI now supports background/border gradients and per-side border colors.
- **Layout**: Uses Flexbox (Taffy). Prefer `Node` with `Display::Flex`. 0.17 introduces better support for `rem` units and sticky nodes.

#### 4. Advanced Rendering (0.17)
- **Solari (Raytracing)**: Experimental real-time raytraced indirect lighting. Enable via `AtmosphereMode::Raymarched` for high-fidelity skies.
- **DLSS**: Support for Deep Learning Super Sampling on Nvidia GPUs for upscaling and anti-aliasing.
- **Light Textures**: Use textures to modulate light intensity (gobos/projected patterns).
- **Tilemap Chunk Rendering**: New built-in performant way to render tilemaps in chunks.

#### 5. System Scheduling
- **Run Conditions**: Prefer `.run_if(in_state(GameState::Playing))`.
- **FixedUpdate**: Use for physics/gameplay logic. 0.17 includes improved testing infrastructure for fixed timesteps.
- **Hotpatching (0.17)**: Initial integration for sub-second hot-reloading of Rust system code (limited to ECS systems).

#### 6. Anti-Patterns
- Don't manually cleanup state entities; use `StateScoped`.
- Don't ignore UI rendering features; ensure `bevy_ui_render` is active if UI is invisible.
- Don't use raw `Trigger<E>` if the simpler `On<E>` syntax suffices for your observer.