use bevy::prelude::*;
use clap::Parser;

use crate::{args::Args, NetworkAddress, NetworkMode, TicksPerSecond, ViewDistance};

pub struct McrsSettingsPlugin;

impl Plugin for McrsSettingsPlugin {
    fn build(&self, app: &mut App) {
        let args = Args::parse();

        if let Some(addr) = args.address_server {
            app.insert_resource(NetworkAddress { server: addr });
        } else {
            app.insert_resource(NetworkAddress::default());
        }

        let network_mode = NetworkMode::from(args.network_mode.as_deref());
        app.insert_resource(network_mode);

        let tps = TicksPerSecond::default();
        app.insert_resource(ticks_per_second(tps.0));
        app.insert_resource(tps);

        app.insert_resource(ViewDistance::default());
    }
}

pub fn ticks_per_second(tps: u32) -> Time<Fixed> {
    Time::<Fixed>::from_seconds(1. / (tps as f64))
}
