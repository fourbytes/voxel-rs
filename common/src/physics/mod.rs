use crate::world::BlockPos;
pub use ncollide3d::bounding_volume::{BoundingVolume, AABB};

pub mod camera;
pub mod player;
pub mod simulation;

/// A "block container", i.e. either the client's World or the server's World.
/// This trait allows the physics simulation to work transparently with both World structs.
pub trait BlockContainer {
    fn is_block_full(&self, pos: BlockPos) -> bool;
}
