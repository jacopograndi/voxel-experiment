use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use mcrs_blueprints::{blocks::BlockBlueprint, Blueprints};
use mcrs_input::PlayerInput;

pub fn ui(
    mut commands: Commands,
    mut input: ResMut<PlayerInput>,
    mut hand: Local<u8>,
    mut contexts: EguiContexts,
    blueprints: Res<Blueprints>,
) {
    egui::Window::new("Hotbar")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::ZERO)
        .title_bar(false)
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                let mut blueprints: Vec<&BlockBlueprint> = blueprints.blocks.iter().collect();
                blueprints.sort_by(|a, b| a.id.cmp(&b.id));
                for blueprint in blueprints.iter() {
                    if blueprint.name == "Air" {
                        continue;
                    }
                    let response = ui.button(format!("{}", blueprint.name));
                    if response.clicked() {
                        *hand = *blueprint.id;
                    }
                }
            });
        });

    input.block_in_hand = *hand;

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
