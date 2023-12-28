use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    prelude::*,
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

pub const DIAGNOSTIC_FPS: DiagnosticId =
    DiagnosticId::from_u128(288146834822086093791974408528866909484);
pub const DIAGNOSTIC_FRAME_TIME: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352065418785002088010278);

pub struct DebugDiagnosticPlugin;

impl Plugin for DebugDiagnosticPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(EguiPlugin)
            .register_diagnostic(
                Diagnostic::new(DIAGNOSTIC_FRAME_TIME, "frame_time", 1000).with_suffix("ms"),
            )
            .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS, "fps", 1000))
            .add_systems(Update, (debug_diagnostic_system, debug_diagnostic_ui));
    }
}

pub fn debug_diagnostic_ui(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
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

pub fn debug_diagnostic_system(mut diagnostics: Diagnostics, time: Res<Time<Real>>) {
    let delta_seconds = time.delta_seconds_f64();
    if delta_seconds == 0.0 {
        return;
    }
    diagnostics.add_measurement(DIAGNOSTIC_FRAME_TIME, || delta_seconds * 1000.0);
    diagnostics.add_measurement(DIAGNOSTIC_FPS, || 1.0 / delta_seconds);
}
