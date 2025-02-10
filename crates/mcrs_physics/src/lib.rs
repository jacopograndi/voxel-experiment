use bevy::prelude::{Res, ResMut, Resource};

pub mod character;
pub mod intersect;
pub mod plugin;
pub mod raycast;

#[cfg(test)]
pub mod tests;

const MARGIN_EPSILON: f32 = 0.0001;

/// Used to tick the physics state manually
#[derive(Resource, Default, Debug)]
pub enum TickStep {
    /// Tick normally
    #[default]
    Tick,

    /// Tick on demand
    /// Only tick when step is true, after the tick it's set as false
    Step { step: bool },
}

impl TickStep {
    pub const STOP: Self = TickStep::Step { step: false };
}

pub fn run_if_tickstep(tickstep: Res<TickStep>) -> bool {
    match *tickstep {
        TickStep::Tick => true,
        TickStep::Step { step } => step,
    }
}

pub fn reset_tickstep(mut tickstep: ResMut<TickStep>) {
    match tickstep.as_mut() {
        TickStep::Step { step } => *step = false,
        _ => {}
    }
}

// Used to print in testing
#[cfg(test)]
const DEBUG_PRINT: bool = true;
fn test_print(_s: String) {
    #[cfg(test)]
    if DEBUG_PRINT {
        println!("{}", _s);
    }
}
