use bevy::prelude::*;

use crate::FixedMainSet;

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveLevelEvent>()
            .add_event::<LoadLevelEvent>()
            .add_event::<CreateLevelEvent>()
            .add_event::<QuitLevelEvent>()
            .add_systems(
                FixedUpdate,
                (save_level, load_level, create_level, quit_level)
                    .chain()
                    .in_set(FixedMainSet::SaveLoad),
            );
    }
}

#[derive(Resource, Debug, Clone)]
pub struct Level {
    pub name: String,
}

#[derive(Event, Debug, Clone)]
pub struct CreateLevelEvent {
    pub level_name: String,
}

#[derive(Event, Debug, Clone)]
pub struct LoadLevelEvent {
    pub level_name: String,
}

#[derive(Event, Debug, Clone)]
pub struct SaveLevelEvent;

#[derive(Event, Debug, Clone)]
pub struct QuitLevelEvent;

pub fn create_level(
    event_reader: EventReader<CreateLevelEvent>,
    mut commands: Commands,
    existing_level: Option<Res<Level>>,
) {
    let Some(event) = get_single_event(event_reader) else {
        return;
    };

    if existing_level.is_some() {
        warn!("Another level is loaded");
        return;
    }

    commands.insert_resource(Level {
        name: event.level_name,
    });
}

pub fn load_level(event_reader: EventReader<LoadLevelEvent>) {
    let Some(event) = get_single_event(event_reader) else {
        return;
    };

    // get the data from the files, the root directory will be called "event.level_name"
    // then setup
    println!("{}", event.level_name);
}

pub fn save_level(event_reader: EventReader<SaveLevelEvent>) {
    let Some(_) = get_single_event(event_reader) else {
        return;
    };

    // actually save here
    // gather all the data, put it in a string.
    // the minecraft model requires that the save data spans multiple files
}

pub fn quit_level(
    event_reader: EventReader<QuitLevelEvent>,
    mut commands: Commands,
    existing_level: Option<Res<Level>>,
) {
    let Some(_) = get_single_event(event_reader) else {
        return;
    };

    if existing_level.is_none() {
        warn!("No level is loaded");
        return;
    }

    commands.remove_resource::<Level>();
    // also destroy universe as well as all game entities?
}

/// Helper function to run only one level operation per type per tick.
pub fn get_single_event<T: Event + Clone + std::fmt::Debug>(
    mut event_reader: EventReader<T>,
) -> Option<T> {
    let Some(event) = event_reader.read().next() else {
        return None;
    };
    let event = event.clone();
    if !event_reader.is_empty() {
        warn!(
            "Too many events of type {:?}, only one will be executed per tick (events num: {}).",
            event,
            event_reader.len()
        );
        event_reader.clear();
    }
    return Some(event.clone());
}
