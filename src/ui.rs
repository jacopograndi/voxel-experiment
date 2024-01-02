use bevy::{
    ecs::system::Commands,
    hierarchy::BuildChildren,
    prelude::default,
    render::color::Color,
    ui::{node_bundles::NodeBundle, AlignItems, JustifyContent, Style, Val},
};

pub fn client_ui(mut commands: Commands) {
    // UI center cursor
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(5.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                background_color: Color::rgba(0.1, 0.1, 0.1, 0.3).into(),
                ..default()
            });
        });
}
