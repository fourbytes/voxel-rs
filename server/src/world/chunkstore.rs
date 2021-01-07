use super::ServerChunk;
use anyhow::{Error, Result};
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::Entry, HashMap};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use voxel_rs_common::world::ChunkPos;

const SECTOR_SIZE: u8 = 8;

pub(crate) type SectorPos = Point2<i64>;
pub(crate) type SectorChunkPos = Point2<u8>;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Sector {
    chunks: HashMap<SectorChunkPos, ServerChunk>,
}

impl Sector {}

#[derive(Default)]
pub(crate) struct ChunkStore {
    world_path: PathBuf,
    open_sectors: Box<HashMap<SectorPos, (File, Sector)>>,
}

impl ChunkStore {
    pub fn get_sector(&self, sector_pos: SectorPos) -> Option<&Sector> {
        self.open_sectors.get(&sector_pos).map(|(_, sec)| sec)
    }

    pub fn get_sector_mut<'c>(&'c mut self, sector_pos: SectorPos) -> Result<&'c mut Sector> {
        Ok(&mut self
            .open_sectors
            .entry(sector_pos)
            .or_insert_with(|| {
                let sector_path = self
                    .world_path
                    .join(format!("{:?},{:?}.voxs", sector_pos.x, sector_pos.y));
                let fd = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(sector_path)
                    .expect("Couldn't open sector file.");
                let data = bincode::deserialize_from(fd).unwrap_or_default();
                (fd, data)
            })
            .1)
    }

    fn get_sector_pos(chunk_pos: ChunkPos) -> SectorPos {
        SectorPos::new(
            chunk_pos.px / SECTOR_SIZE as i64,
            chunk_pos.pz / SECTOR_SIZE as i64,
        )
    }

    fn get_sector_chunk_pos(chunk_pos: ChunkPos) -> SectorChunkPos {
        SectorChunkPos::new(
            (chunk_pos.px % SECTOR_SIZE as i64) as u8,
            (chunk_pos.pz % SECTOR_SIZE as i64) as u8,
        )
    }

    pub fn get_chunk<'c>(&'c self, chunk_pos: ChunkPos) -> Result<Option<&'c ServerChunk>> {
        let sector_pos = Self::get_sector_pos(chunk_pos);
        let sector_chunk_pos = Self::get_sector_chunk_pos(chunk_pos);
        let sector = self.get_sector(sector_pos).unwrap();

        Ok(sector.chunks.get(&sector_chunk_pos))
    }

    pub fn get_chunk_mut<'c>(
        &'c mut self,
        chunk_pos: ChunkPos,
    ) -> Result<Option<&'c mut ServerChunk>> {
        let sector_pos = Self::get_sector_pos(chunk_pos);
        let sector_chunk_pos = Self::get_sector_chunk_pos(chunk_pos);
        let sector = self.get_sector_mut(sector_pos)?;

        Ok(sector.chunks.get_mut(&sector_chunk_pos))
    }

    pub fn chunk_entry(
        &mut self,
        chunk_pos: ChunkPos,
    ) -> Result<Entry<SectorChunkPos, ServerChunk>> {
        let sector_pos = Self::get_sector_pos(chunk_pos);
        let sector_chunk_pos = Self::get_sector_chunk_pos(chunk_pos);
        let sector = self.get_sector_mut(sector_pos)?;

        Ok(sector.chunks.entry(sector_chunk_pos))
    }

    pub fn chunk_exists(&mut self, chunk_pos: ChunkPos) -> Result<bool> {
        let sector_pos = Self::get_sector_pos(chunk_pos);
        let sector_chunk_pos = Self::get_sector_chunk_pos(chunk_pos);
        let sector = self.get_sector_mut(sector_pos)?;
        Ok(sector.chunks.contains_key(&sector_chunk_pos))
    }

    pub fn flush_chunk(&mut self, chunk_pos: ChunkPos) -> Result<()> {
        let sector_pos: SectorPos = SectorPos::new(
            chunk_pos.px / SECTOR_SIZE as i64,
            chunk_pos.pz / SECTOR_SIZE as i64,
        );

        self.flush_sector(sector_pos)
    }

    pub fn flush_sector(&mut self, sector_pos: SectorPos) -> Result<()> {
        let (fd, sector) = self.open_sectors.get_mut(&sector_pos).unwrap();

        Ok(bincode::serialize_into(fd, sector)?)
    }
}
