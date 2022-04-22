use std::{collections::HashMap, sync::Arc};

use cgmath::{vec3, Vector3, Vector2, vec2};
use wgpu::{BindGroup, Device};

use crate::model::{Faces, Model, ModelData};

pub const WIDTH: i64 = 16;
pub const HEIGHT: i64 = 16;
pub const LENGTH: i64 = 16;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    None,
    Air,
    Stone,
    Dirt
}
#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub kind: BlockKind,
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

pub struct Chunk {
    blocks: Vec<Block>,
    pub chunk_x: i64,
    pub chunk_y: i64,
    pub chunk_z: i64,
}
impl Chunk {
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self {
            // blocks: vec![Block::{}; WIDTH * HEIGHT * LENGTH],
            blocks: (0..(WIDTH * HEIGHT * LENGTH))
                .map(|i| {
                    let (bx, by, bz) = Self::unflatten(i);
                    Block {
                        kind: BlockKind::Air,
                        x: bx,
                        y: by,
                        z: bz,
                    }
                })
                .collect::<Vec<Block>>(),
            chunk_x: x,
            chunk_y: y,
            chunk_z: z,
        }
    }
    pub fn get_block<'a>(&'a self, x: i64, y: i64, z: i64) -> Option<&'a Block> {
        self.blocks.get(Self::flatten(x, y, z))
    }
    pub fn set_block(&mut self, b: Block) {
        self.blocks[Self::flatten(b.x, b.y, b.z)] = b;
    }
    pub fn get_block_kinds(&self) -> Vec<BlockKind> {
        self.blocks.iter().map(|b| b.kind).collect()
    }
    fn flatten(x: i64, y: i64, z: i64) -> usize {
        ((z * WIDTH * HEIGHT) + (y * WIDTH) + x) as usize
    }
    fn unflatten(idx: i64) -> (i64, i64, i64) {
        let mut idx = idx;
        let z = idx / (WIDTH * HEIGHT);
        idx -= (z * WIDTH * HEIGHT);
        let y = idx / WIDTH;
        let x = idx % WIDTH;
        (x, y, z)
    }
    fn block_north_of(&self, b: &Block) -> BlockKind {
        self.get_block(b.x, b.y, b.z + 1)
            .map(|s| s.kind)
            .unwrap_or(BlockKind::None)
    }
    fn block_south_of(&self, b: &Block) -> BlockKind {
        self.get_block(b.x, b.y, b.z - 1)
            .map(|s| s.kind)
            .unwrap_or(BlockKind::None)
    }
    fn block_east_of(&self, b: &Block) -> BlockKind {
        self.get_block(b.x + 1, b.y, b.z)
            .map(|s| s.kind)
            .unwrap_or(BlockKind::None)
    }
    fn block_west_of(&self, b: &Block) -> BlockKind {
        self.get_block(b.x - 1, b.y, b.z)
            .map(|s| s.kind)
            .unwrap_or(BlockKind::None)
    }
    fn block_above(&self, b: &Block) -> BlockKind {
        self.get_block(b.x, b.y + 1, b.z)
            .map(|s| s.kind)
            .unwrap_or(BlockKind::None)
    }
    fn block_below(&self, b: &Block) -> BlockKind {
        self.get_block(b.x, b.y - 1, b.z)
            .map(|s| s.kind)
            .unwrap_or(BlockKind::None)
    }
    pub fn models(&self, device: &Device, bind_group: Arc<BindGroup>) -> Vec<Model> {
        // let mut models = vec![];
        let mut hm: HashMap<Faces, Vec<ModelData>> = HashMap::new();

        for block in &self.blocks {
            if let BlockKind::Air = block.kind {
            } else {
                let north = self.block_north_of(block) == BlockKind::Air;
                let south = self.block_south_of(block) == BlockKind::Air;
                let east = self.block_east_of(block) == BlockKind::Air;
                let west = self.block_west_of(block) == BlockKind::Air;
                let above = self.block_above(block) == BlockKind::Air;
                let below = self.block_below(block) == BlockKind::Air;
                let f = Faces {
                    north,
                    south,
                    east,
                    west,
                    bottom: below,
                    top: above,
                };
                // let m = Model::new(
                //     &device,
                //     &f,
                //     vec![vec3(block.x as f32, block.y as f32, block.z as f32)],
                //     bind_group.clone(),
                // );
                let pos = vec3(block.x as f32 , block.y as f32, block.z as f32);
                hm.entry(f).or_insert(vec![]).push(ModelData {
                    position: pos,
                    kind: block.kind
                });
                // hm.insert(f, );
                // models.push(Model::new(
                //     &device,
                //     &f,
                //     vec![vec3(block.x as f32, block.y as f32, block.z as f32)],
                //     bind_group.clone(),
                // ));
            }
        }
        let mut moleds = vec![];
        println!("{} unique faces", hm.keys().len());
        // let kinds = self.get_block_kinds();
        for (faces, position) in hm {
            moleds.push(Model::new(&device, &faces, position, bind_group.clone()))
        }

        moleds
    }
}
impl BlockKind {
    pub fn get_tex_coords(&self) -> Vector2<f32> {
            match self {
                BlockKind::None => todo!(),
                BlockKind::Air => todo!(),
                BlockKind::Stone => vec2(0.0, 0.0),
                BlockKind::Dirt => vec2(1.0, 0.0)
            }
    }
}