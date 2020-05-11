use voxel_rs_common::light::{compute_light, ChunkLightingState};
use voxel_rs_common::world::{LightChunk, HighestOpaqueBlock};
use voxel_rs_common::world::chunk::{Chunk, ChunkPos};
use std::sync::Arc;
use voxel_rs_common::worker::{Worker, WorkerState};

/// The chunk-specific data that is needed to generate light for it.
pub struct ChunkLightingData {
    pub chunks: Vec<Option<Arc<Chunk>>>,
    pub highest_opaque_blocks: Vec<Arc<HighestOpaqueBlock>>,
}

impl WorkerState<ChunkLightingData, Arc<LightChunk>> for ChunkLightingState {
    fn compute(&mut self, pos: ChunkPos, data: ChunkLightingData) -> Arc<LightChunk> {
        Arc::new(LightChunk {
            light: compute_light(
                data.chunks,
                data.highest_opaque_blocks,
                &mut self.queue_reuse,
                &mut self.light_data_reuse,
                &mut self.opaque_reuse,
            ).light_level.to_vec(),
            pos,
        })
    }
}

pub type ChunkLightingWorker = Worker<ChunkLightingData, Arc<LightChunk>, ChunkLightingState>;
