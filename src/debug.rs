use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, RegisterDiagnostic},
    prelude::*,
};
use bevy_egui::{egui, EguiContexts};
use mcrs_universe::{universe::Universe, Blueprints};

pub const DIAGNOSTIC_FPS: DiagnosticPath = DiagnosticPath::const_new("game/fps");
pub const DIAGNOSTIC_FRAME_TIME: DiagnosticPath = DiagnosticPath::const_new("game/frame_time");

pub struct DebugDiagnosticPlugin;

impl Plugin for DebugDiagnosticPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_diagnostic(
            Diagnostic::new(DIAGNOSTIC_FRAME_TIME)
                .with_max_history_length(1000)
                .with_suffix("ms"),
        )
        .register_diagnostic(Diagnostic::new(DIAGNOSTIC_FPS).with_max_history_length(1000))
        .add_systems(Update, (debug_diagnostic_system, debug_diagnostic_ui));
    }
}

pub fn debug_diagnostic_system(mut diagnostics: Diagnostics, time: Res<Time<Real>>) {
    let delta_seconds = time.delta_secs_f64();
    if delta_seconds == 0.0 {
        return;
    }
    diagnostics.add_measurement(&DIAGNOSTIC_FRAME_TIME, || delta_seconds * 1000.0);
    diagnostics.add_measurement(&DIAGNOSTIC_FPS, || 1.0 / delta_seconds);
}

pub fn debug_diagnostic_ui(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    egui::Window::new("Debug menu").show(contexts.ctx_mut(), |ui| {
        if let Some(value) = diagnostics
            .get(&DIAGNOSTIC_FPS)
            .and_then(|fps| fps.smoothed())
        {
            ui.label(format!("fps: {value:>4.2}"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints = if let Some(diag) = diagnostics.get(&DIAGNOSTIC_FPS) {
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
            .get(&DIAGNOSTIC_FRAME_TIME)
            .and_then(|ms| ms.value())
        {
            ui.label(format!("time: {value:>4.2} ms"));
            use egui_plot::{Line, PlotPoints};
            let n = 1000;
            let line_points: PlotPoints =
                if let Some(diag) = diagnostics.get(&DIAGNOSTIC_FRAME_TIME) {
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

pub struct WidgetBlockDebug<'a> {
    pub pos: IVec3,
    pub universe: &'a Universe,
    pub bp: &'a Blueprints,
}

impl<'a> WidgetBlockDebug<'a> {
    pub fn new(pos: IVec3, universe: &'a Universe, bp: &'a Blueprints) -> Self {
        Self { pos, universe, bp }
    }
}

impl<'a> egui::Widget for WidgetBlockDebug<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.group(|ui| {
            if let Some(block) = self.universe.read_chunk_block(&self.pos) {
                egui::Grid::new("Block").striped(true).show(ui, |ui| {
                    let block_bp = self.bp.blocks.get(&block.id);
                    ui.label("Position");
                    ui.add(egui::Label::new(format!("{}", self.pos)));
                    ui.end_row();

                    ui.label("Type");
                    ui.label(format!("{}", block_bp.name));
                    ui.end_row();

                    ui.label("Id");
                    ui.label(format!("{:?}", block_bp.id));
                    ui.end_row();

                    ui.label("Brightness");
                    ui.label(format!("{}", block_bp.light_level));
                    ui.end_row();

                    ui.label("Lighting (torch)");
                    ui.label(format!("{}", block.light0));
                    ui.end_row();

                    ui.label("Lighting (sun)");
                    ui.label(format!("{}", block.light1));
                    ui.end_row();
                });
            }
        })
        .response
    }
}
