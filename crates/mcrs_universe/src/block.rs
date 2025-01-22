use std::{fmt::Display, str::FromStr};

use bevy::prelude::{Deref, DerefMut};
use bytemuck::{Pod, Zeroable};
use mcrs_macros::EnumIter;
use serde::{Deserialize, Serialize};

use crate::{is_default, HasNameId, MAX_LIGHT};

/// 1 cubic meter ingame
#[repr(C)]
#[derive(Debug, Clone, Pod, Zeroable, Copy, Default, PartialEq, Eq)]
pub struct Block {
    pub id: BlockId,
    pub properties: FlagBank,
    // for now i'm using light0 as torchlight and light1 as sunlight
    // in the future they could be a u16 divided into 4 u4
    // that encode red, green and blue torchlight and sunlight.
    pub light0: u8,
    pub light1: u8,
}
impl Block {
    // Generation and flag checking/setting utilities
    pub fn new(block_info: &BlockBlueprint) -> Self {
        Self {
            id: block_info.id,
            light0: block_info.light_level,
            light1: 0,
            properties: block_info.flags,
        }
    }

    pub fn get_light(&self, light_type: LightType) -> u8 {
        match light_type {
            LightType::Torch => self.light0,
            LightType::Sun => self.light1,
        }
    }
    pub fn set_light(&mut self, light_type: LightType, v: u8) {
        assert!((0..=MAX_LIGHT).contains(&v), "brightness: {}", v);
        match light_type {
            LightType::Torch => self.light0 = v,
            LightType::Sun => self.light1 = v,
        }
    }
}

/// Specification of the properties of a block.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct BlockBlueprint {
    pub name: String,
    pub id: BlockId,
    pub flags: FlagBank,

    #[serde(default, skip_serializing_if = "is_default")]
    pub light_level: u8,

    #[serde(default, skip_serializing_if = "is_default")]
    pub voxel_texture_path: String,

    #[serde(default, skip_serializing_if = "is_default")]
    pub block_texture_offset: Vec<u32>,

    #[serde(default, skip_serializing_if = "is_default")]
    pub drop_item_id: BlockId,
}
impl HasNameId<BlockId> for BlockBlueprint {
    fn id(&self) -> BlockId {
        self.id
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}
impl BlockBlueprint {
    /// True if the block emits light
    pub fn is_light_source(&self) -> bool {
        self.light_level > 0
    }
}

/// Logical block Id.
/// For now, it's also the offset in the 3d texture buffer.
#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq, Clone, Hash, Copy, Deref, DerefMut, Pod, Zeroable)]
pub struct BlockId(u8);
impl From<u8> for BlockId {
    fn from(v: u8) -> Self {
        BlockId(v)
    }
}

// Tell serde to only serialize the inner number
impl Serialize for BlockId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for BlockId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|id| BlockId(id))
    }
}

/// Bit index of each block flag in human readable form
#[derive(Debug, Copy, Clone, Serialize, Deserialize, EnumIter)]
pub enum BlockFlag {
    Collidable,
    Opaque,
    Flag3,
    Flag4,
    Flag5,
    Flag6,
    Flag7,
    Flag8,
}

impl From<BlockFlag> for u8 {
    fn from(v: BlockFlag) -> Self {
        v as u8
    }
}
impl ToString for BlockFlag {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}
impl FromStr for BlockFlag {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(format!("{:?}", s).parse()?)
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct FlagBank {
    _flags: u8,
}

impl FlagBank {
    pub fn set(&mut self, flag: impl Into<u8>) {
        self._flags |= 0b1 << flag.into();
    }

    pub fn unset(&mut self, flag: impl Into<u8>) {
        self._flags &= !(0b1 << flag.into());
    }

    pub fn check(&self, flag: impl Into<u8>) -> bool {
        (self._flags >> flag.into()) & 0b1 == 1
    }
}

impl From<FlagBank> for Vec<String> {
    fn from(v: FlagBank) -> Self {
        BlockFlag::iter()
            .filter_map(|flag| v.check(flag).then_some(flag.to_string()))
            .collect()
    }
}

impl From<FlagBank> for Vec<BlockFlag> {
    fn from(v: FlagBank) -> Self {
        BlockFlag::iter()
            .filter_map(|flag| v.check(flag).then_some(flag))
            .collect()
    }
}

impl From<Vec<String>> for FlagBank {
    fn from(vec: Vec<String>) -> Self {
        let mut flagbank = FlagBank::default();
        for v in vec {
            flagbank.set(BlockFlag::from_str(&v).unwrap());
        }
        flagbank
    }
}

impl From<Vec<BlockFlag>> for FlagBank {
    fn from(vec: Vec<BlockFlag>) -> Self {
        let mut flagbank = FlagBank::default();
        for v in vec {
            flagbank.set(v);
        }
        flagbank
    }
}

impl Serialize for FlagBank {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <FlagBank as Into<Vec<BlockFlag>>>::into(*self).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FlagBank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .map(|flagbank_vec: Vec<BlockFlag>| flagbank_vec.into())
    }
}

/// j: This is a performance optimization.
/// The block's light level is max(torch, sun).
///
/// Sunlight propagates differently: when it travels down it isn't dimmed.
/// It comes from the top of the topmost loaded chunk.
/// (we use a heightmap to do it faster when blocks are modified)
/// Torchlight is dimmed every time it travels regardless of direction.
///
/// If we were to have a single light type, we couldn't support the day-night cycle.
/// During night, sunlight is set to zero. During the day, sunlight is set to max.
/// During transitions it's interpolated.
/// If sunlight == torchlight, every time the day's light level changes
/// every block's light must be recalculated, and that's very slow.
/// If sunlight != torchlight, we do it only when a chunk is generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightType {
    Torch,
    Sun,
}
impl Display for LightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LightType::Torch => "torch",
                LightType::Sun => "sun",
            }
        )
    }
}
