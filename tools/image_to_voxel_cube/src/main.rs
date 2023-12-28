use std::collections::HashMap;

use dot_vox::{Color, DotVoxData, Model, Voxel};
use image::RgbaImage;

const PATH_IN: &str = "resources/blocks.png";
const PATH_OUT: &str = "voxels/";

fn main() {
    let blocks = vec![
        Block {
            name: "stone",
            method: Method::Single(Uv(1, 0)),
        },
        Block {
            name: "dirt",
            method: Method::Single(Uv(2, 0)),
        },
        Block {
            name: "grass",
            method: Method::Single(Uv(4, 0)),
        },
        Block {
            name: "planks-oak",
            method: Method::Single(Uv(5, 0)),
        },
        Block {
            name: "wood-oak",
            method: Method::equal_sides(Uv(5, 1), Uv(5, 1), Uv(4, 1)),
        },
        Block {
            name: "glowstone",
            method: Method::Single(Uv(9, 6)),
        },
    ];

    let sheet = image::open(PATH_IN).unwrap();
    for block in blocks.iter() {
        let mut cubemap = RgbaImage::new(SIZE * 6, SIZE);
        match block.method {
            Method::Single(ref uv) => {
                for z in 0..6 {
                    let face = sheet.crop_imm(uv.0 * SIZE, uv.1 * SIZE, SIZE, SIZE);
                    image::imageops::replace(&mut cubemap, &face, (z * SIZE) as i64, 0);
                }
            }
            Method::Cubemap(ref uvs) => {
                for z in 0..6 {
                    let face = sheet.crop_imm(uvs[z].0 * SIZE, uvs[z].1 * SIZE, SIZE, SIZE);
                    image::imageops::replace(&mut cubemap, &face, z as i64 * SIZE as i64, 0);
                }
            }
        };

        let vox = img_to_vox(&cubemap);
        let mut outfile =
            std::fs::File::create(PATH_OUT.to_string() + block.name + ".vox").unwrap();
        vox.write_vox(&mut outfile).unwrap();

        println!("Generated: {}", block.name);
    }
}

struct Block {
    name: &'static str,
    method: Method,
}

enum Method {
    Single(Uv),
    Cubemap([Uv; 6]),
}

impl Method {
    fn equal_sides(top: Uv, bottom: Uv, side: Uv) -> Self {
        Self::Cubemap([top, side.clone(), side.clone(), side.clone(), side, bottom])
    }
}

const SIZE: u32 = 16;

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
struct PaletteColor([u8; 4]);
impl From<PaletteColor> for Color {
    fn from(value: PaletteColor) -> Self {
        Color {
            r: value.0[0],
            g: value.0[1],
            b: value.0[2],
            a: value.0[3],
        }
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct VoxId(u8);

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
struct Uv(u32, u32);

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
struct Pos(u32, u32, u32);

fn img_to_vox(img: &RgbaImage) -> DotVoxData {
    let mut serial: u8 = 0;
    let mut palette: HashMap<PaletteColor, VoxId> = HashMap::new();
    let mut table: HashMap<Uv, VoxId> = HashMap::new();

    for z in 0..6 {
        for x in 0..SIZE {
            for y in 0..SIZE {
                let uv = Uv(x + z * SIZE, y);
                let pixel: [u8; 4] = img.get_pixel(uv.0, uv.1).0;
                let col = PaletteColor(pixel);
                let id = if let Some(id) = palette.get(&col) {
                    id.clone()
                } else {
                    let id = VoxId(serial);
                    palette.insert(col, id.clone());
                    serial += 1;
                    id
                };
                table.insert(uv, id);
            }
        }
    }

    let mut voxels: HashMap<Pos, VoxId> = HashMap::new();

    // -y
    for x in 0..SIZE {
        for y in 0..SIZE {
            let pos = Pos(x, y, SIZE - 1);
            let uv = Uv(x + SIZE * 5, y);
            let id = table.get(&uv).unwrap();
            voxels.insert(pos, id.clone());
        }
    }

    // -z
    for x in 0..SIZE {
        for y in 0..SIZE {
            let pos = Pos(x, SIZE - 1, y);
            let uv = Uv(x + SIZE * 4, y);
            let id = table.get(&uv).unwrap();
            voxels.insert(pos, id.clone());
        }
    }

    // -x
    for x in 0..SIZE {
        for y in 0..SIZE {
            let pos = Pos(SIZE - 1, x, y);
            let uv = Uv(x + SIZE * 3, y);
            let id = table.get(&uv).unwrap();
            voxels.insert(pos, id.clone());
        }
    }

    // +z
    for x in 0..SIZE {
        for y in 0..SIZE {
            let pos = Pos(x, 0, y);
            let uv = Uv(x + SIZE * 2, y);
            let id = table.get(&uv).unwrap();
            voxels.insert(pos, id.clone());
        }
    }

    // +x
    for x in 0..SIZE {
        for y in 0..SIZE {
            let pos = Pos(0, x, y);
            let uv = Uv(x + SIZE * 1, y);
            let id = table.get(&uv).unwrap();
            voxels.insert(pos, id.clone());
        }
    }

    // +y
    for x in 0..SIZE {
        for y in 0..SIZE {
            let pos = Pos(x, y, 0);
            let uv = Uv(x, y);
            let id = table.get(&uv).unwrap();
            voxels.insert(pos, id.clone());
        }
    }

    let models = vec![Model {
        size: dot_vox::Size {
            x: SIZE,
            y: SIZE,
            z: SIZE,
        },
        voxels: voxels
            .iter()
            .map(|(pos, i)| Voxel {
                x: pos.0 as u8,
                y: pos.1 as u8,
                z: pos.2 as u8,
                i: i.0 as u8,
            })
            .collect(),
    }];

    let mut sorted_palette: Vec<(&PaletteColor, &VoxId)> = palette.iter().collect();
    sorted_palette.sort_by(|a, b| a.1.cmp(&b.1));

    DotVoxData {
        version: 150,
        models,
        palette: sorted_palette.iter().map(|c| c.0.clone().into()).collect(),
        materials: vec![],
        scenes: vec![],
        layers: vec![],
    }
}
