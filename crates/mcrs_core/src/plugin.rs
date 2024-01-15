use bevy::{log::LogPlugin, prelude::*, window::PresentMode};
use mcrs_blueprints::plugin::McrsBlueprintsPlugin;
use mcrs_input::plugin::{InputSet, McrsInputPlugin};
use mcrs_net::plugin::{McrsNetClientPlugin, McrsNetServerPlugin, NetSet};
use mcrs_physics::plugin::{McrsPhysicsPlugin, PhysicsSet};
use mcrs_render::plugin::McrsVoxelRenderPlugin;
use mcrs_settings::{plugin::McrsSettingsPlugin, NetworkMode};
use mcrs_storage::McrsVoxelStoragePlugin;

use crate::camera::{camera_controller_movement, cursor_grab};

#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum CoreSet {
    Update,
}

pub struct McrsCorePlugin;

impl Plugin for McrsCorePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            FixedUpdate,
            (
                NetSet::Receive,
                PhysicsSet::Update,
                CoreSet::Update,
                NetSet::Send,
                InputSet::Consume,
            )
                .chain(),
        );

        app.add_plugins((
            McrsSettingsPlugin,
            McrsVoxelStoragePlugin,
            McrsBlueprintsPlugin,
        ));

        match app.world.get_resource::<NetworkMode>() {
            Some(NetworkMode::Server) => {
                app.add_plugins((MinimalPlugins, TransformPlugin, LogPlugin::default()));
                app.add_plugins((McrsNetServerPlugin, McrsPhysicsPlugin));
            }
            Some(NetworkMode::Client) => {
                app.add_plugins((McrsNetClientPlugin, McrsCameraPlugin));
                app_client(app);
            }
            _ => {
                app.add_plugins((McrsNetServerPlugin, McrsPhysicsPlugin));
                app.add_plugins((McrsNetClientPlugin, McrsCameraPlugin));
                app_client(app);
            }
        }
        app.add_plugins(McrsInputPlugin);
    }
}

fn app_client(app: &mut App) {
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
        McrsVoxelRenderPlugin,
    ));
}

pub struct McrsCameraPlugin;

impl Plugin for McrsCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (camera_controller_movement, cursor_grab));
    }
}
