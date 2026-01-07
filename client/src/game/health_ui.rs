//! Health bar UI systems.

use bevy::prelude::*;

use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{MyClientId, Player, PlayerOwner};

/// Marker for health bar UI container
#[derive(Component)]
pub struct HealthBarUI;

/// Marker for health bar fill element
#[derive(Component)]
pub struct HealthBarFill;

/// Marker for health text element
#[derive(Component)]
pub struct HealthText;

/// Display and update health bar UI
pub fn display_health_bar(
    mut commands: Commands,
    player_query: Query<(&Health, &PlayerOwner), With<Player>>,
    my_client_id: Option<Res<MyClientId>>,
    health_ui_query: Query<Entity, With<HealthBarUI>>,
    mut health_fill_query: Query<
        (&mut Node, &mut BackgroundColor),
        (With<HealthBarFill>, Without<HealthText>),
    >,
    mut health_text_query: Query<(&mut Text, &mut TextColor), With<HealthText>>,
) {
    let Some(my_client_id) = my_client_id else {
        // Cleanup health UI if client ID is gone (disconnected)
        for entity in health_ui_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    // Find our player's health
    let mut our_health: Option<&Health> = None;
    for (health, owner) in player_query.iter() {
        if owner.0 == my_client_id.0 {
            our_health = Some(health);
            break;
        }
    }

    // Clean up health UI if player doesn't exist
    if our_health.is_none() && !health_ui_query.is_empty() {
        for entity in health_ui_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    if our_health.is_some() && health_ui_query.is_empty() {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(20.0),
                    top: Val::Px(20.0),
                    width: Val::Px(300.0),
                    height: Val::Px(50.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                HealthBarUI,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("Health: 100/100 (100%)"),
                    TextFont {
                        font_size: 20.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        margin: UiRect::bottom(Val::Px(5.0)),
                        ..default()
                    },
                    HealthText,
                ));

                parent
                    .spawn((
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.8, 0.2)),
                            HealthBarFill,
                        ));
                    });
            });
    }

    if let Some(health) = our_health {
        let health_percent = (health.current / health.max * 100.0).max(0.0);

        // Color from health
        let bar_color = if health_percent > 60.0 {
            Color::srgb(0.2, 0.8, 0.2)
        } else if health_percent > 30.0 {
            Color::srgb(1.0, 0.8, 0.0)
        } else {
            Color::srgb(1.0, 0.2, 0.2)
        };

        // Update health bar fill width and color
        if let Ok((mut node, mut bg_color)) = health_fill_query.single_mut() {
            node.width = Val::Percent(health_percent);
            *bg_color = bar_color.into();
        }

        if let Ok((mut text, mut text_color)) = health_text_query.single_mut() {
            text.0 = format!(
                "Health: {:.0}/{:.0} ({:.0}%)",
                health.current, health.max, health_percent
            );

            text_color.0 = if health_percent > 60.0 {
                Color::srgb(0.2, 1.0, 0.2)
            } else if health_percent > 30.0 {
                Color::srgb(1.0, 0.8, 0.0)
            } else {
                Color::srgb(1.0, 0.2, 0.2)
            };
        }
    }
}
