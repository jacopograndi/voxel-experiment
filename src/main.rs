use std::{
    collections::VecDeque,
    f32::consts::PI,
};

use bevy::{
    core_pipeline::fxaa::Fxaa,
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    prelude::*,
    utils::HashSet,
    window::{PresentMode, WindowPlugin},
};

use bevy_egui::{egui, EguiContexts, EguiPlugin};

use voxel_physics::{
    character::{
        CameraController, Character, CharacterController, CharacterId, Friction, Velocity,
    },
    plugin::VoxelPhysicsPlugin,
    raycast,
};
use voxel_render::{
    boxes_world::{Ghost, VoxTextureIndex, VoxTextureLoadQueue},
    voxel_world::VIEW_DISTANCE,
    VoxelCameraBundle, VoxelRenderPlugin,
};
use voxel_storage::{
    block::{Block, LightType, MAX_LIGHT},
    chunk::Chunk,
    universe::Universe,
    VoxelStoragePlugin, CHUNK_SIDE, CHUNK_VOLUME, BlockType,
};

use voxel_flag_bank::{BlockFlag, ChunkFlag};

pub const DIAGNOSTIC_FPS: DiagnosticId =
    DiagnosticId::from_u128(288146834822086093791974408528866909484);
pub const DIAGNOSTIC_FRAME_TIME: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352065418785002088010278);

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
        VoxelRenderPlugin,
        VoxelPhysicsPlugin,
        VoxelStoragePlugin,
        EguiPlugin,
    ))
    .register_diagnostic(
        Diagnostic::new(DIAGNOSTIC_FRAME_TIME, "frame_time", 1000).with_suffix("ms"),
    )
    .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS, "fps", 1000))
    .insert_resource(ClearColor(Color::MIDNIGHT_BLUE))
    .add_systems(Startup, setup)
    .add_systems(Update, ui)
    .add_systems(Update, load_and_gen_chunks)
    .add_systems(Update, control)
    .add_systems(Update, diagnostic_system)
    .add_systems(Update, spin);

    app.add_systems(Update, voxel_break);

    app.run();
}

// just for prototype
fn voxel_break(
    camera_query: Query<(&CameraController, &GlobalTransform)>,
    mut universe: ResMut<Universe>,
    mouse: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::R) {
        let chunks = universe.chunks.iter().map(|v| v.0.clone()).collect();
        recalc_lights(&mut universe, chunks);
    }
    if let Ok((_cam, tr)) = camera_query.get_single() {
        #[derive(PartialEq)]
        enum Act {
            PlaceBlock,
            RemoveBlock,
            Inspect,
        }
        let act = match (
            mouse.just_pressed(MouseButton::Left),
            mouse.just_pressed(MouseButton::Right),
            mouse.just_pressed(MouseButton::Middle),
        ) {
            (true, _, _) => Some(Act::RemoveBlock),
            (_, true, _) => Some(Act::PlaceBlock),
            (_, _, true) => Some(Act::Inspect),
            _ => None,
        };
        if let Some(act) = act {
            if let Some(hit) = raycast::raycast(tr.translation(), tr.forward(), 4.5, &universe) {
                match act {
                    Act::Inspect => {
                        println!(
                            "hit(pos:{}, block:{:?}, dist:{}), head(block:{:?})",
                            hit.pos,
                            universe.read_chunk_block(&hit.grid_pos),
                            hit.distance,
                            universe.read_chunk_block(&tr.translation().floor().as_ivec3()),
                        );
                    }
                    Act::RemoveBlock => {
                        println!("removed block");

                        let pos = hit.grid_pos;

                        let mut light_suns = vec![];
                        let mut light_torches = vec![];

                        if let Some(voxel) = universe.read_chunk_block(&pos) {
                            // todo: use BlockInfo.is_light_source
                            if voxel.is(BlockType::Dirt) {
                                let new = propagate_darkness(&mut universe, pos, LightType::Torch);
                                propagate_light(&mut universe, new, LightType::Torch)
                            }
                        }

                        universe.set_chunk_block(
                            &pos,
                            Block::new(BlockType::Air),
                        );

                        let planar = IVec2::new(pos.x, pos.z);
                        if let Some(height) = universe.heightfield.get(&planar) {
                            if pos.y == *height {
                                // recalculate the highest sunlit point
                                let mut beam = pos.y - 100;
                                for y in 0..=100 {
                                    let h = pos.y - y;
                                    let sample = IVec3::new(pos.x, h, pos.z);
                                    if let Some(voxel) = universe.read_chunk_block(&sample) {
                                        if voxel.properties.check(BlockFlag::Opaque) {
                                            beam = h;
                                            break;
                                        } else {
                                            light_suns.push(sample);

                                            let mut lit = voxel.clone();
                                            lit.set_light(LightType::Sun, 15);
                                            universe.set_chunk_block(&sample, lit);
                                        }
                                    }
                                }
                                universe.heightfield.insert(planar, beam);
                            }
                        }

                        for dir in DIRS.iter() {
                            let sample = pos + *dir;
                            if let Some(voxel) = universe.read_chunk_block(&sample) {
                                if !voxel.properties.check(BlockFlag::Opaque) {
                                    if voxel.get_light(LightType::Sun) > 1 {
                                        light_suns.push(sample);
                                    }
                                    if voxel.get_light(LightType::Torch) > 1 {
                                        light_torches.push(sample);
                                    }
                                }
                            }
                        }

                        propagate_light(&mut universe, light_suns, LightType::Sun);
                        propagate_light(&mut universe, light_torches, LightType::Torch);
                    }
                    Act::PlaceBlock => {
                        println!("placed block");

                        let pos = hit.grid_pos + hit.normal;

                        let mut dark_suns = vec![];

                        if keys.pressed(KeyCode::Key3) {
                            // todo: use BlockInfo
                            universe.set_chunk_block(
                                &pos,
                                Block::new(BlockType::Wood),
                            );
                            universe.read_chunk_block(&pos).unwrap().set_light(LightType::Torch, 14);
                            propagate_light(&mut universe, vec![pos], LightType::Torch)
                        } else {
                            let new = propagate_darkness(&mut universe, pos, LightType::Torch);

                            universe.set_chunk_block(
                                &pos,
                                Block::new(BlockType::Wood),
                            );

                            propagate_light(&mut universe, new, LightType::Torch);
                        }

                        let planar = IVec2::new(pos.x, pos.z);
                        if let Some(height) = universe.heightfield.get(&planar) {
                            if pos.y > *height {
                                // recalculate the highest sunlit point
                                for y in (*height)..pos.y {
                                    let sample = IVec3::new(pos.x, y, pos.z);
                                    dark_suns.push(sample);
                                }
                                universe.heightfield.insert(planar, pos.y);
                            }
                        }

                        for sun in dark_suns {
                            let new = propagate_darkness(&mut universe, sun, LightType::Sun);
                            propagate_light(&mut universe, new, LightType::Sun)
                        }
                    }
                };
            } else {
                //dbg!("no hit");
            }
        }
    }
}

fn gen_chunk(pos: IVec3) -> Chunk {
    if pos.y < 0 {
        Chunk::filled()
    } else {
        Chunk::empty()
    }
}

fn recalc_lights(universe: &mut Universe, chunks: Vec<IVec3>) {
    println!("lighting {:?} chunks", chunks.len());

    // calculate sunlight beams
    let mut suns: Vec<IVec3> = vec![];
    let mut planars = HashSet::<IVec2>::new();
    let mut highest = i32::MIN;
    for pos in chunks.iter() {
        let chunk = universe.chunks.get_mut(pos).unwrap();
        chunk.properties.set(ChunkFlag::Dirty);
        // let mut grid = chunk.get_w_ref();
        for x in 0..CHUNK_SIDE {
            for z in 0..CHUNK_SIDE {
                let mut sunlight = MAX_LIGHT;
                for y in (0..CHUNK_SIDE).rev() {
                    let xyz = IVec3::new(x as i32, y as i32, z as i32);
                    if chunk.read_block(xyz).properties.check(BlockFlag::Opaque) {
                        sunlight = 0;
                    }
                    if sunlight > 0 {
                        suns.push(*pos + xyz);
                    }
                    chunk.set_block_light(xyz, LightType::Sun, sunlight);
                    chunk.set_block_light(xyz, LightType::Torch, 0);
                    highest = highest.max(pos.y + y as i32);
                }
                let planar = IVec2::new(x as i32 + pos.x, z as i32 + pos.z);
                planars.insert(planar);
            }
        }
    }

    for planar in planars.iter() {
        let mut beam = 0;
        let mut block_found = false;
        for y in 0..1000 {
            let h = highest - y;
            let sample = IVec3::new(planar.x, h, planar.y);

            if let Some(voxel) = universe.read_chunk_block(&sample) {
                block_found = true;
                if voxel.properties.check(BlockFlag::Opaque) {
                    beam = h;
                    break;
                }
            } else {
                if block_found {
                    break;
                }
            }
        }
        if let Some(height) = universe.heightfield.get_mut(planar) {
            *height = (*height).min(beam);
        } else {
            universe.heightfield.insert(*planar, beam);
        }
    }

    // find new light sources
    let mut torches: Vec<IVec3> = vec![];
    for pos in chunks.iter() {
        let chunk = universe.chunks.get(pos).unwrap();
        for i in 0..CHUNK_VOLUME {
            let xyz = Chunk::_idx2xyz(i);
            // todo: fetch from BlockInfo when implemented
            if chunk.read_block(xyz).is(BlockType::Dirt) {
                torches.push(*pos + xyz);
                chunk.set_block_light(xyz, LightType::Torch, 15);
            }
        }
    }

    if !suns.is_empty() {
        propagate_light(universe, suns, LightType::Sun);
    }

    if !torches.is_empty() {
        propagate_light(universe, torches, LightType::Torch);
    }
}

const DIRS: [IVec3; 6] = [
    IVec3::X,
    IVec3::Y,
    IVec3::Z,
    IVec3::NEG_X,
    IVec3::NEG_Y,
    IVec3::NEG_Z,
];
const MAX_LIGHTITNG_PROPAGATION: usize = 100000000;

fn propagate_darkness(universe: &mut Universe, source: IVec3, lt: LightType) -> Vec<IVec3> {
    let voxel = universe.read_chunk_block(&source).unwrap();
    let val = voxel.get_light(lt);
    let mut dark = voxel.clone();
    dark.set_light(lt, 0);
    universe.set_chunk_block(&source, dark);

    println!("1 source of {lt} darkness val:{val}");

    let mut new_lights: Vec<IVec3> = vec![];
    let mut frontier: VecDeque<IVec3> = [source].into();
    for iter in 0..MAX_LIGHTITNG_PROPAGATION {
        if let Some(pos) = frontier.pop_front() {
            for dir in DIRS.iter() {
                let target = pos + *dir;
                let mut unlit: Option<Block> = None;
                if let Some(neighbor) = universe.read_chunk_block(&target) {
                    let target_light = neighbor.get_light(lt);
                    if target_light != 0 && target_light < val {
                        let mut l = neighbor;
                        l.set_light(lt, 0);
                        unlit = Some(l);
                    } else if target_light >= val {
                        new_lights.push(target);
                    }
                }
                if let Some(voxel) = unlit {
                    universe.set_chunk_block(&target, voxel);
                    frontier.push_back(target);
                    let (c, _) = universe.pos_to_chunk_and_inner(&target);
                    universe.chunks.get_mut(&c).unwrap().properties.set(ChunkFlag::Dirty);
                }
            }
        } else {
            println!("{} iters for {lt} darkness", iter);
            break;
        }
    }
    new_lights
}

fn propagate_light(universe: &mut Universe, sources: Vec<IVec3>, lt: LightType) {
    const DIRS: [IVec3; 6] = [
        IVec3::X,
        IVec3::Y,
        IVec3::Z,
        IVec3::NEG_X,
        IVec3::NEG_Y,
        IVec3::NEG_Z,
    ];
    const MAX_LIGHTITNG_PROPAGATION: usize = 100000000;

    println!("{} sources of {lt} light", sources.len());
    let mut frontier: VecDeque<IVec3> = sources.clone().into();
    for iter in 0..MAX_LIGHTITNG_PROPAGATION {
        if let Some(pos) = frontier.pop_front() {
            let voxel = universe.read_chunk_block(&pos).unwrap();
            let light = voxel.get_light(lt);
            for dir in DIRS.iter() {
                let target = pos + *dir;
                let mut lit: Option<Block> = None;
                if let Some(neighbor) = universe.read_chunk_block(&target) {
                    if !neighbor.properties.check(BlockFlag::Opaque) && neighbor.get_light(lt) + 2 <= light {
                        let mut l = neighbor;
                        l.set_light(lt, light - 1);
                        lit = Some(l);
                    }
                }
                if let Some(voxel) = lit {
                    universe.set_chunk_block(&target, voxel);
                    frontier.push_back(target);
                    let (c, _) = universe.pos_to_chunk_and_inner(&target);
                    universe.chunks.get_mut(&c).unwrap().properties.set(ChunkFlag::Dirty);
                }
            }
        } else {
            println!("{} iters for {lt} light", iter);
            break;
        }
    }
}

fn load_and_gen_chunks(mut universe: ResMut<Universe>, camera: Query<(&Camera, &Transform)>) {
    let load_view_distance: u32 = VIEW_DISTANCE;

    let camera_pos = if let Ok((_, tr)) = camera.get_single() {
        tr.translation
    } else {
        return;
    };

    let camera_chunk_pos = (camera_pos / CHUNK_SIDE as f32).as_ivec3() * CHUNK_SIDE as i32;

    // hardcoded chunk size
    let load_view_distance_chunk = load_view_distance as i32 / CHUNK_SIDE as i32;
    let lvdc = load_view_distance_chunk;

    let mut added = vec![];

    // sphere centered on the player
    for x in -lvdc..=lvdc {
        for y in -lvdc..=lvdc {
            for z in -lvdc..=lvdc {
                let rel = IVec3::new(x, y, z) * CHUNK_SIDE as i32;
                if rel.as_vec3().length_squared() < load_view_distance.pow(2) as f32 {
                    let pos = camera_chunk_pos + rel;
                    if !universe.chunks.contains_key(&pos) {
                        universe.chunks.insert(
                            pos,
                            gen_chunk(pos),
                        );
                        added.push(pos);
                    }
                }
            }
        }
    }

    if !added.is_empty() {
        recalc_lights(&mut universe, added);
    }
}

fn setup(mut commands: Commands, mut queue: ResMut<VoxTextureLoadQueue>) {
    queue
        .to_load
        .push(("assets/voxels/stone.vox".to_string(), VoxTextureIndex(1)));
    queue
        .to_load
        .push(("assets/voxels/dirt.vox".to_string(), VoxTextureIndex(2)));
    queue
        .to_load
        .push(("assets/voxels/wood-oak.vox".to_string(), VoxTextureIndex(3)));
    queue.to_load.push((
        "assets/voxels/glowstone.vox".to_string(),
        VoxTextureIndex(4),
    ));

    // player character
    commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_xyz(0.0, 5.0, 0.0)),
            Character {
                id: CharacterId(0),
                size: Vec3::new(0.5, 1.5, 0.5),
                air_speed: 0.001,
                ground_speed: 0.03,
                jump_strenght: 0.17,
            },
            CharacterController {
                acceleration: Vec3::splat(0.0),
                jumping: false,
            },
            Velocity::default(),
            Friction {
                air: Vec3::splat(0.99),
                ground: Vec3::splat(0.78),
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                VoxelCameraBundle {
                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: 1.57,
                        ..default()
                    }),
                    ..default()
                },
                Fxaa::default(),
                CameraController::default(),
            ));
        });

    // center cursor
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

    commands.spawn((
        SpatialBundle::from_transform(Transform {
            translation: Vec3::new(0.0, 13.0 / 16.0 * 0.5, 0.0),
            ..default()
        }),
        Ghost {
            vox_texture_index: VoxTextureIndex(0),
        },
    ));

    commands.spawn((
        SpatialBundle::from_transform(Transform {
            translation: Vec3::new(3.0, 14.0 / 16.0 * 0.5, -2.0),
            rotation: Quat::from_rotation_y(PI / 2.0),
            ..default()
        }),
        Ghost {
            vox_texture_index: VoxTextureIndex(1),
        },
        Party::default(),
    ));
}

#[derive(Component, Clone, Default, Debug)]
struct Party {
    scale: Option<Vec3>,
}

fn spin(mut q: Query<(&mut Transform, &mut Party)>, time: Res<Time<Real>>) {
    for (mut tr, mut party) in q.iter_mut() {
        tr.rotate_y(0.1);
        if let None = party.scale {
            party.scale = Some(tr.scale)
        }
        tr.scale = party.scale.unwrap() * f32::cos(time.elapsed_seconds());
    }
}

fn control(
    mut character_query: Query<(&mut CharacterController, &Transform)>,
    keys: Res<Input<KeyCode>>,
) {
    for (mut controller, tr) in character_query.iter_mut() {
        let mut delta = Vec3::ZERO;
        if keys.pressed(KeyCode::W) {
            delta += tr.forward();
        }
        if keys.pressed(KeyCode::S) {
            delta -= tr.forward();
        }
        if keys.pressed(KeyCode::A) {
            delta += tr.left();
        }
        if keys.pressed(KeyCode::D) {
            delta -= tr.left();
        }
        delta = delta.normalize_or_zero();
        controller.acceleration = delta;
        if keys.pressed(KeyCode::Space) {
            controller.jumping = true;
        } else {
            controller.jumping = false;
        }
    }
}

fn ui(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    egui::Window::new("Debug menu").show(contexts.ctx_mut(), |ui| {
        if let Some(value) = diagnostics
            .get(DIAGNOSTIC_FPS)
            .and_then(|fps| fps.smoothed())
        {
            ui.label(format!("fps: {value:>4.2}"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(DIAGNOSTIC_FPS) {
                diag.values()
                    .take(n)
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v])
                    .collect()
            } else {
                PlotPoints::default()
            };
            let line = Line::new(line_points).fill(0.0);
            egui_plot::Plot::new("fps")
                .include_y(0.0)
                .height(70.0)
                .show_axes([false, true])
                .show(ui, |plot_ui| plot_ui.line(line))
                .response;
        } else {
            ui.label("no fps data");
        }
        if let Some(value) = diagnostics
            .get(DIAGNOSTIC_FRAME_TIME)
            .and_then(|ms| ms.value())
        {
            ui.label(format!("time: {value:>4.2} ms"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(DIAGNOSTIC_FRAME_TIME)
            {
                diag.values()
                    .take(n)
                    .enumerate()
                    .map(|(i, v)| [i as f64, *v])
                    .collect()
            } else {
                PlotPoints::default()
            };
            let line = Line::new(line_points).fill(0.0);
            egui_plot::Plot::new("frame time")
                .include_y(0.0)
                .height(70.0)
                .show_axes([false, true])
                .show(ui, |plot_ui| plot_ui.line(line))
                .response;
        } else {
            ui.label("no frame time data");
        }
        ui.separator()
    });
}

pub fn diagnostic_system(mut diagnostics: Diagnostics, time: Res<Time<Real>>) {
    let delta_seconds = time.delta_seconds_f64();
    if delta_seconds == 0.0 {
        return;
    }
    diagnostics.add_measurement(DIAGNOSTIC_FRAME_TIME, || delta_seconds * 1000.0);
    diagnostics.add_measurement(DIAGNOSTIC_FPS, || 1.0 / delta_seconds);
}
