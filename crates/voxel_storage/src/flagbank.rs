use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default, PartialEq, Eq)]
pub struct FlagBank {
    _flags: u16
}

impl FlagBank {

    pub fn empty() -> Self {
        Self {
            _flags: 0
        }
    }

    pub fn set<T>(&mut self, flag: T) where T: Into<u8> {
        self._flags |= 0b1 << flag.into();
    }

    pub fn unset<T>(&mut self, flag: T) where T: Into<u8> {
        self._flags &= !(0b1 << flag.into());
    }

    pub fn check<T>(&self, flag: T) -> bool where T: Into<u8> {
        (self._flags >> flag.into()) & 0b1 == 1
    }
}