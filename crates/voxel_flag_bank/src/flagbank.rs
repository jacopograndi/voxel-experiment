use bytemuck::{Pod, Zeroable};

use crate::IsFlagBank;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default, PartialEq, Eq)]
pub struct FlagBank {
    _flags: u8
}

impl FlagBank {

    pub fn empty() -> Self {
        Self {
            _flags: 0
        }
    }

    pub fn set<T>(&mut self, flag: T) where T: IsFlagBank {
        self._flags |= 0b1 << flag.to_u8();
    }

    pub fn unset<T>(&mut self, flag: T) where T: IsFlagBank {
        self._flags &= !(0b1 << flag.to_u8());
    }

    pub fn check<T>(&self, flag: T) -> bool where T: IsFlagBank {
        (self._flags >> flag.to_u8()) & 0b1 == 1
    }
}