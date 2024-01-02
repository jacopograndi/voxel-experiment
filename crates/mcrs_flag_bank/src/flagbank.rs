use bytemuck::{Pod, Zeroable};

use crate::IsFlagBank;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct FlagBank {
    _flags: u8,
}

impl FlagBank {
    pub fn set<T>(&mut self, flag: T)
    where
        T: IsFlagBank,
    {
        self._flags |= 0b1 << flag.to_u8();
    }

    pub fn unset<T>(&mut self, flag: T)
    where
        T: IsFlagBank,
    {
        self._flags &= !(0b1 << flag.to_u8());
    }

    pub fn check<T>(&self, flag: T) -> bool
    where
        T: IsFlagBank,
    {
        (self._flags >> flag.to_u8()) & 0b1 == 1
    }
}
