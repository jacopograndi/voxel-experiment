use super::get_sun_heightfield;
use bevy::{prelude::*, utils::HashMap};
use mcrs_universe::CHUNK_AREA;

#[derive(Default, Clone, Debug)]
pub struct SunBeam {
    pub bottom: i32,
    pub top: i32,
}

impl SunBeam {
    fn new(start: i32, end: i32) -> Self {
        Self {
            bottom: start,
            top: end,
        }
    }

    pub fn new_top(xz: &IVec2) -> Self {
        let sun_height = get_sun_heightfield(*xz);
        Self::new(sun_height, sun_height)
    }

    // Extend an existing beam with another adjacent or overlapping one.
    pub fn extend(&mut self, new_beam: SunBeam) {
        assert!(
            self.top + 1 >= new_beam.bottom && new_beam.top >= self.bottom - 1,
            "not adjacent: {:?}, {:?}",
            self,
            new_beam
        );
        self.bottom = self.bottom.min(new_beam.bottom);
        self.top = self.top.max(new_beam.top);
    }

    /// If `at` is inside the beam, return the two parts of the beam:
    /// ```(self.start..=at, (at+1)..=self.end)```
    pub fn cut(&mut self, at: i32) -> Option<(SunBeam, SunBeam)> {
        if (self.bottom..=self.top).contains(&at) {
            let lower = SunBeam::new(self.bottom, at);
            let higher = SunBeam::new(at + 1, self.top);
            self.bottom = (at + 1).min(self.top);
            Some((lower, higher))
        } else {
            None
        }
    }

    pub fn contains(&self, at: &i32) -> bool {
        (self.bottom..=self.top).contains(at)
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub struct SunBeams {
    pub beams: HashMap<IVec2, SunBeam>,
}

impl SunBeams {
    pub fn get_at_mut<'a>(&'a mut self, xz: &'a IVec2) -> &'a mut SunBeam {
        self.beams.entry(*xz).or_insert(SunBeam::new_top(xz))
    }

    pub fn extend_beam(&mut self, xz: &IVec2, new_beam: SunBeam) {
        let sun_beam = self.get_at_mut(xz);
        sun_beam.extend(new_beam);
    }

    pub fn cut_beam(&mut self, xz: &IVec2, at: i32) -> Option<(SunBeam, SunBeam)> {
        let sun_beam = self.get_at_mut(xz);
        sun_beam.cut(at)
    }
}

#[derive(Clone, Debug)]
pub struct RegionSunBeam([SunBeam; CHUNK_AREA]);
