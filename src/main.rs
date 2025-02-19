use bevy::{log::LogPlugin, prelude::*};
use bevy_egui::EguiPlugin;
use camera::McrsCameraPlugin;
use clap::Parser;

use mcrs_net::{
    plugin::{FixedNetSet, McrsNetClientPlugin, McrsNetServerPlugin},
    NetSettings, NetworkMode,
};
use mcrs_physics::plugin::{FixedPhysicsSet, McrsPhysicsPlugin};
use mcrs_render::{
    chunk_mesh::TextureHandles, plugin::McrsVoxelRenderPlugin, settings::RenderSettings,
};
use mcrs_universe::McrsUniversePlugin;

mod camera;
mod chemistry;
mod debug;
mod input;
mod player;
mod saveload;
mod settings;
mod terrain;
mod ui;

use debug::DebugDiagnosticPlugin;
use input::*;
use player::{spawn_player, terrain_editing};
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

fn main() -> AppExit {
    let mut app = App::new();

    app.configure_sets(
        FixedUpdate,
        (
            FixedNetSet::Receive,
            FixedPhysicsSet::Tick,
            FixedMainSet::Terrain,
            FixedMainSet::SaveLoad,
            FixedNetSet::Send,
        )
            .chain(),
    );
    app.configure_sets(Update, (UiSet::Overlay, InputSet::Gather).chain());

    // todo: encapsulate in a settings plugin?
    let settings: McrsSettings = Args::parse().into();
    app.insert_resource(Time::<Fixed>::from_seconds(
        1f64 / settings.ticks_per_second as f64,
    ));
    app.insert_resource::<NetSettings>(settings.clone().into());
    app.insert_resource::<RenderSettings>(settings.clone().into());
    app.insert_resource(settings.clone());

    app.add_plugins(McrsUniversePlugin);
    app.init_resource::<UniverseChanges>();
    app.init_resource::<LightSources>();
    app.init_resource::<ChunkGenerationRequest>();
    app.init_resource::<SunBeams>();

    match settings.network_mode {
        NetworkMode::Client => {
            add_client(&mut app);
        }
        NetworkMode::Server => {
            app.add_plugins((MinimalPlugins, TransformPlugin, LogPlugin::default()));
            add_server(&mut app);
        }
        NetworkMode::ClientAndServer => {
            add_client(&mut app);
            add_server(&mut app);
        }
    }
    app.add_systems(Update, spawn_player);

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
        McrsNetClientPlugin,
        McrsCameraPlugin,
    ));
    app.add_systems(
        Startup,
        (load_texture, ui_center_cursor, setup_hotbar).chain(),
    );
    app.add_systems(Update, send_fake_window_resize_once);
    app.add_systems(Update, hotbar_interaction.in_set(UiSet::Overlay));
    app.add_systems(
        Update,
        (player_input, move_local_players)
            .chain()
            .in_set(InputSet::Gather),
    );
    app.add_systems(Update, terrain_editing.after(InputSet::Gather));
}

fn add_server(app: &mut App) {
    app.add_plugins((McrsNetServerPlugin, McrsPhysicsPlugin, SaveLoadPlugin));
    app.add_systems(
        FixedUpdate,
        ((
            request_base_chunks,
            chunk_generation,
            apply_terrain_changes,
            apply_lighting_sources,
        )
            .chain()
            .in_set(FixedMainSet::Terrain),),
    );
}

pub fn load_texture(mut texture_handle: ResMut<TextureHandles>, asset_server: Res<AssetServer>) {
    texture_handle.blocks = asset_server.load("textures/blocks.png");
}
