pub mod chunk;
pub mod section;

use std::sync::Arc;
use dashmap::DashMap;

pub use chunk::{Chunk, MAX_Y, MIN_Y, TOTAL_SECTIONS};
pub use section::{ChunkSection, Palette};

#[derive(Debug, Clone)]
pub struct SharedWorldCache {
    pub dimension: String,
    chunks: Arc<DashMap<(i32, i32), Arc<Chunk>>>,
}

impl SharedWorldCache {
    pub fn new(dimension: impl Into<String>) -> Self {
        Self {
            dimension: dimension.into(),
            chunks: Arc::new(DashMap::new()),
        }
    }

    pub fn insert_chunk(&self, chunk: Arc<Chunk>) {
        self.chunks.insert((chunk.x, chunk.z), chunk);
    }

    pub fn get_chunk(&self, x: i32, z: i32) -> Option<Arc<Chunk>> {
        self.chunks.get(&(x, z)).map(|c| c.clone())
    }

    pub fn remove_chunk(&self, x: i32, z: i32) {
        self.chunks.remove(&(x, z));
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn clear(&self) {
        self.chunks.clear();
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> u32 {
        let chunk_x = x.div_euclid(16);
        let chunk_z = z.div_euclid(16);
        if let Some(chunk) = self.chunks.get(&(chunk_x, chunk_z)) {
            chunk.get_block(x, y, z)
        } else {
            0 // Default Air if chunk not loaded
        }
    }

    pub fn set_block(&self, x: i32, y: i32, z: i32, state: u32) {
        let chunk_x = x.div_euclid(16);
        let chunk_z = z.div_euclid(16);
        let chunk = self
            .chunks
            .entry((chunk_x, chunk_z))
            .or_insert_with(|| Arc::new(Chunk::new(chunk_x, chunk_z)));
        chunk.set_block(x, y, z, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_world_cache_operations() {
        let world = SharedWorldCache::new("minecraft:overworld");
        let chunk = Arc::new(Chunk::new(0, 0));
        world.insert_chunk(chunk);

        assert_eq!(world.chunk_count(), 1);
        assert_eq!(world.get_block(0, 64, 0), 0);

        world.set_block(0, 64, 0, 1); // Set Stone (state 1)
        assert_eq!(world.get_block(0, 64, 0), 1);
    }
}
