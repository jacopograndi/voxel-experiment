use bevy::{
    diagnostic::{Diagnostic, Diagnostics, RegisterDiagnostic},
    prelude::*,
    render::RenderPlugin,
};

use crate::{ui::debug_diagnostic_ui, DIAGNOSTIC_FPS, DIAGNOSTIC_FRAME_TIME};

pub struct McrsDebugPlugin;

impl Plugin for McrsDebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(McrsDiagnosticPlugin);
    }
}

pub struct McrsDiagnosticPlugin;

impl Plugin for McrsDiagnosticPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<RenderPlugin>() {
            return;
        };
        app.register_diagnostic(
            Diagnostic::new(DIAGNOSTIC_FRAME_TIME, "frame_time", 1000).with_suffix("ms"),
        )
        .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS, "fps", 1000))
        .add_systems(Update, (debug_diagnostic_system, debug_diagnostic_ui));
    }
}

pub fn debug_diagnostic_system(mut diagnostics: Diagnostics, time: Res<Time<Real>>) {
    let delta_seconds = time.delta_seconds_f64();
    if delta_seconds == 0.0 {
        return;
    }
    diagnostics.add_measurement(DIAGNOSTIC_FRAME_TIME, || delta_seconds * 1000.0);
    diagnostics.add_measurement(DIAGNOSTIC_FPS, || 1.0 / delta_seconds);
}
