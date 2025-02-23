use bevy::{asset::LoadState, log::LogPlugin, prelude::*, state::app::StatesPlugin};
use bevy_egui::EguiPlugin;
use camera::McrsCameraPlugin;
use clap::Parser;

use mcrs_physics::plugin::{FixedPhysicsSet, McrsPhysicsPlugin};
use mcrs_render::{
    chunk_mesh::TextureHandles, plugin::McrsVoxelRenderPlugin, settings::RenderSettings,
};
use mcrs_universe::McrsUniversePlugin;

mod camera;
mod chemistry;
mod debug;
mod input;
mod net;
mod player;
mod saveload;
mod settings;
mod terrain;
mod ui;

use debug::DebugDiagnosticPlugin;
use input::*;
use net::*;
use player::{spawn_player, terrain_editing};
use plugin::{FixedNetSet, NetClientPlugin, NetServerPlugin};
use saveload::*;
use settings::{Args, McrsSettings};
use terrain::*;
use ui::*;

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum FixedMainSet {
    Terrain,
    SaveLoad,
}

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum UiSet {
    Overlay,
}

#[derive(States, Default, Clone, PartialEq, Eq, Hash, Debug)]
pub enum AppState {
    #[default]
    LoadingAssets,
    Playing,
}

fn main() -> AppExit {
    let mut app = App::new();

    // todo: encapsulate in a settings plugin?
    let settings: McrsSettings = Args::parse().into();
    app.insert_resource(Time::<Fixed>::from_seconds(
        1f64 / settings.ticks_per_second as f64,
    ));
    app.insert_resource::<NetSettings>(settings.clone().into());
    app.insert_resource::<RenderSettings>(settings.clone().into());
    app.insert_resource(settings.clone());

    app.add_plugins((McrsUniversePlugin, McrsPhysicsPlugin, SaveLoadPlugin));
    app.init_resource::<UniverseChanges>();
    app.init_resource::<LightSources>();
    app.init_resource::<ChunkGenerationRequest>();
    app.init_resource::<SunBeams>();

    match settings.network_mode {
        NetworkMode::Client => {
            add_client(&mut app);
            app.insert_state(AppState::default());
        }
        NetworkMode::Server => {
            app.add_plugins((MinimalPlugins, TransformPlugin, LogPlugin::default(), StatesPlugin));
            add_server(&mut app);
            app.insert_state(AppState::Playing);
        }
        NetworkMode::ClientAndServer => {
            add_client(&mut app);
            add_server(&mut app);
            app.insert_state(AppState::default());
        }
        NetworkMode::Offline => {
            add_client(&mut app);
            add_server(&mut app);
            app.add_systems(Update, spawn_player.run_if(in_state(AppState::Playing)));
            app.insert_state(AppState::default());
        }
    }

    app.configure_sets(
        FixedUpdate,
        (
            FixedNetSet::Receive,
            FixedPhysicsSet::Tick,
            FixedMainSet::Terrain,
            FixedMainSet::SaveLoad,
            FixedNetSet::Send,
        )
            .chain()
            .run_if(in_state(AppState::Playing)),
    );
    app.configure_sets(
        Update,
        (UiSet::Overlay, InputSet::Gather)
            .chain()
            .run_if(in_state(AppState::Playing)),
    );

    app.run()
}

fn add_client(app: &mut App) {
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    //present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
        McrsVoxelRenderPlugin,
        EguiPlugin,
        DebugDiagnosticPlugin,
        NetClientPlugin,
        McrsCameraPlugin,
    ));

    // Load assets
    app.add_systems(OnEnter(AppState::LoadingAssets), load_texture);
    app.add_systems(
        Update,
        load_texture_check_finished.run_if(in_state(AppState::LoadingAssets)),
    );

    // Client systems
    app.add_systems(
        OnEnter(AppState::Playing),
        (load_texture, ui_center_cursor, setup_hotbar).chain(),
    );
    app.add_systems(
        Update,
        (
            send_fake_window_resize_once,
            hotbar_interaction.in_set(UiSet::Overlay),
            (player_input, move_local_players)
                .chain()
                .in_set(InputSet::Gather),
            terrain_editing.after(InputSet::Gather),
        )
            .run_if(in_state(AppState::Playing)),
    );
}

fn add_server(app: &mut App) {
    app.add_plugins(NetServerPlugin);
    app.add_systems(
        FixedUpdate,
        (
            chunk_generation,
            apply_terrain_changes,
            apply_lighting_sources,
        )
            .chain()
            .in_set(FixedMainSet::Terrain)
            .run_if(in_state(AppState::Playing)),
    );
}

pub fn load_texture(mut texture_handle: ResMut<TextureHandles>, asset_server: Res<AssetServer>) {
    texture_handle.blocks = asset_server.load("textures/blocks.png");
}

pub fn load_texture_check_finished(
    texture_handle: ResMut<TextureHandles>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    match asset_server.get_load_state(&texture_handle.blocks) {
        Some(LoadState::Loaded) => {
            next_state.set(AppState::Playing);
        }
        Some(LoadState::Failed(e)) => {
            eprintln!("Failed to load the blocks texture: {e}");
            panic!();
        }
        _ => {}
    }
}
