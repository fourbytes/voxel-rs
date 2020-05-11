use std::collections::{HashMap, HashSet};
use voxel_rs_common::block::BlockId;
use voxel_rs_common::{
    player::RenderDistance,
    world::chunk::ChunkPos,
};

pub mod light;
mod worldgen;
pub mod server;

// TODO: refactor
const D: [[i64; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// The data that the server stores for every player.
#[derive(Debug, Clone)]
struct PlayerData {
    loaded_chunks: HashSet<ChunkPos>,
    render_distance: RenderDistance,
    block_to_place: BlockId,
}

impl Default for PlayerData {
    fn default() -> Self {
        Self {
            loaded_chunks: Default::default(),
            render_distance: Default::default(),
            block_to_place: 1,
        }
    }
}

