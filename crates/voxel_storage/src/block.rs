use bytemuck::{Pod, Zeroable};

enum Flags {
    SOLID,

}


#[repr(C)]
#[derive(Debug, Clone, Pod, Zeroable, Copy, Default, PartialEq, Eq)]
pub struct Block {
    pub id: u8,
    pub light: u8,
    _properties: u16,
}

impl Block {

    pub fn new(id:u8, solid: bool) -> Self {
        let mut p: u16 = 0;
        if solid { p = 1; }
        Self {
            id: id,
            _properties: p,
            light: 0,
        }
    }

    pub fn set_solid(&mut self) {
        self._properties = 1;
    }

    pub fn is_solid(&self) -> bool {
        self._properties == 1
    }

}
