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

    pub fn set(&mut self, flag: u8) {
        self._flags |= 0b1 << flag;
    }

    pub fn unset(&mut self, flag: u8) {
        self._flags &= !(0b1 << flag);
    }

    pub fn check(&self, flag: u8) -> bool {
        (self._flags >> flag) & 0b1 == 1
    }
}