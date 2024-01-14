use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use mcrs_blueprints::{blocks::BlockBlueprint, Blueprints};
use mcrs_input::PlayerInput;

pub fn hotbar(
    mut input: ResMut<PlayerInput>,
    mut hand: Local<u8>,
    mut contexts: EguiContexts,
    blueprints: Res<Blueprints>,
) {
    if *hand == 0 {
        *hand = 1;
    }
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
                    let selected = *hand == *blueprint.id;
                    let button =
                        egui::Button::new(format!("{}", blueprint.name)).selected(selected);
                    let response = ui.add(button);
                    if response.clicked() {
                        *hand = *blueprint.id;
                    }
                }
            });
        });

    input.block_in_hand = *hand;
}
