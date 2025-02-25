use crate::LocalPlayer;
use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    window::{PrimaryWindow, WindowResized},
};
use mcrs_render::chunk_mesh::TextureHandles;
use mcrs_universe::{
    block::{BlockFace, BlockId},
    Blueprints,
};
use serde::{Deserialize, Serialize};

pub fn ui_center_cursor(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(5.0),
                    height: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.3).into()),
            ));
        });
}

#[derive(Debug, Default, Serialize, Deserialize, Component, Resource, Clone)]
pub struct PlayerHand {
    pub hotbar_index: i32,
    pub block_id: Option<BlockId>,
}

pub fn hotbar_interaction(
    mut hand_query: Query<(&mut PlayerHand, &LocalPlayer)>,
    mut slot_query: Query<(&mut BorderColor, &HotbarSlot, &Children)>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut image_query: Query<&mut ImageNode>,
    keys: Res<ButtonInput<KeyCode>>,
    bp: Res<Blueprints>,
) {
    let Ok((mut hand, _)) = hand_query.get_single_mut() else {
        return;
    };

    for wheel_event in mouse_wheel_events.read() {
        let amt = (wheel_event.x + wheel_event.y) * -1.0;
        let sign = if amt >= 0.0 { 1 } else { -1 };
        hand.hotbar_index += sign;
        hand.hotbar_index = hand.hotbar_index.rem_euclid(9);
        if let Some(block_id) = slot_query
            .iter()
            .find_map(|(_, slot, _)| (slot.index == hand.hotbar_index).then(|| slot.block_id))
        {
            hand.block_id = block_id;
        }
    }

    for (mut color, hotbar_slot, children) in &mut slot_query {
        let key = match hotbar_slot.index {
            0 => KeyCode::Digit1,
            1 => KeyCode::Digit2,
            2 => KeyCode::Digit3,
            3 => KeyCode::Digit4,
            4 => KeyCode::Digit5,
            5 => KeyCode::Digit6,
            6 => KeyCode::Digit7,
            7 => KeyCode::Digit8,
            8 => KeyCode::Digit9,
            9 => KeyCode::Digit0,
            _ => panic!("unknown hotbar index"),
        };
        if keys.just_pressed(key) {
            hand.block_id = hotbar_slot.block_id;
            hand.hotbar_index = hotbar_slot.index;
        }
        if hand.hotbar_index == hotbar_slot.index {
            color.0 = Color::srgb(0.9, 0.9, 0.9);
        } else {
            color.0 = Color::srgb(0.4, 0.4, 0.4);
        }

        if let Ok(mut image_node) = image_query.get_mut(children[0]) {
            let bl = bp.blocks.get(&hotbar_slot.block_id.unwrap());
            let (x, y) = match bl.block_texture_offset.as_ref().unwrap() {
                BlockFace::Same((x, y)) => (x, y),
                BlockFace::Cube { left: (x, y), .. } => (x, y),
            };
            image_node
                .texture_atlas
                .as_mut()
                .map(|atlas| atlas.index = (x + y * 16) as usize);
        }
    }
}

#[derive(Component, Default)]
pub struct HotbarSlot {
    index: i32,
    block_id: Option<BlockId>,
}

pub fn setup_hotbar(
    mut commands: Commands,
    bp: Res<Blueprints>,
    texture_handles: Res<TextureHandles>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture_atlas = TextureAtlasLayout::from_grid(UVec2::splat(16), 16, 16, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(0.),
            width: Val::Percent(100.),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                BorderColor(Color::srgb(0.0, 0.0, 0.0).into()),
            ))
            .with_children(|bar| {
                for (i, name) in [
                    "Stone",
                    "Cobblestone",
                    "Brick",
                    "Dirt",
                    "Oak Planks",
                    "Wood",
                    "Grass",
                    "Glowstone",
                    "Diamond Block",
                ]
                .into_iter()
                .enumerate()
                {
                    bar.spawn((
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            flex_direction: FlexDirection::Column,
                            border: UiRect::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.95).into()),
                        BorderColor(Color::srgb(0.4, 0.4, 0.4).into()),
                        HotbarSlot {
                            index: i as i32,
                            block_id: bp.blocks.get_named_checked(name).map(|bl| bl.id),
                        },
                    ))
                    .with_child(ImageNode::from_atlas_image(
                        texture_handles.blocks.clone(),
                        TextureAtlas::from(texture_atlas_handle.clone()),
                    ));
                }
            });
        });
}

pub fn send_fake_window_resize(
    mut primary_window: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut events: EventWriter<WindowResized>,
    mut timer: Local<Option<Timer>>,
    time: Res<Time>,
) {
    if timer.is_none() {
        *timer = Some(Timer::new(
            std::time::Duration::from_millis(50),
            TimerMode::Once,
        ))
    }
    let Some(timer) = &mut *timer else {
        return;
    };

    timer.tick(time.delta());

    if timer.finished() {
        timer.reset();
        if let Ok((entity, window)) = primary_window.get_single_mut() {
            events.send(WindowResized {
                window: entity,
                width: window.width(),
                height: window.height(),
            });
        }
    }
}
