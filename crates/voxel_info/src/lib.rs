use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use bevy::utils::HashMap;
use ron::from_str;

use voxel_flag_bank::BlockFlag;

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockInfo {
    pub name: String,
    pub id: u8,
    pub flags: Vec<BlockFlag>,
    pub light_level: u8,
    pub voxel_texture_path: String,
    pub drop_item_id: u8,
}

pub fn get_block_info() -> HashMap<u8, BlockInfo> {
    let blockinfo_data = read_to_string("./assets/blockinfo.ron").unwrap();
    let blockinfo_array: Vec<BlockInfo> = from_str(&blockinfo_data).unwrap();
    let mut blockinfo_map: HashMap<u8, BlockInfo> = HashMap::new();
    for blockinfo in blockinfo_array {
        blockinfo_map.insert(blockinfo.id, blockinfo);
    }
    blockinfo_map
}