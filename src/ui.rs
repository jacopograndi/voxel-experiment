use bevy::prelude::*;

pub fn ui(mut commands: Commands) {
    // UI center cursor
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(
                (Node {
                    width: Val::Px(5.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.3).into()),
            ));
        });
}
