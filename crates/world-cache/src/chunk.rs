use std::sync::RwLock;
use crate::section::ChunkSection;

pub const MIN_Y: i32 = -64;
pub const MAX_Y: i32 = 320;
pub const TOTAL_SECTIONS: usize = 24; // (320 - (-64)) / 16

#[derive(Debug)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    sections: Vec<RwLock<ChunkSection>>,
}

impl Chunk {
    pub fn new(x: i32, z: i32) -> Self {
        let mut sections = Vec::with_capacity(TOTAL_SECTIONS);
        for _ in 0..TOTAL_SECTIONS {
            sections.push(RwLock::new(ChunkSection::empty()));
        }
        Self { x, z, sections }
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> u32 {
        if !(MIN_Y..MAX_Y).contains(&y) {
            return 0; // Void / outside world is Air
        }
        let section_idx = ((y - MIN_Y) / 16) as usize;
        let local_x = (x.rem_euclid(16)) as usize;
        let local_y = ((y - MIN_Y).rem_euclid(16)) as usize;
        let local_z = (z.rem_euclid(16)) as usize;

        if let Some(sec_lock) = self.sections.get(section_idx) {
            sec_lock.read().unwrap().get_block(local_x, local_y, local_z)
        } else {
            0
        }
    }

    pub fn set_block(&self, x: i32, y: i32, z: i32, state: u32) {
        if !(MIN_Y..MAX_Y).contains(&y) {
            return;
        }
        let section_idx = ((y - MIN_Y) / 16) as usize;
        let local_x = (x.rem_euclid(16)) as usize;
        let local_y = ((y - MIN_Y).rem_euclid(16)) as usize;
        let local_z = (z.rem_euclid(16)) as usize;

        if let Some(sec_lock) = self.sections.get(section_idx) {
            sec_lock.write().unwrap().set_block(local_x, local_y, local_z, state);
        }
    }

    pub fn set_section(&self, section_idx: usize, section: ChunkSection) {
        if let Some(sec_lock) = self.sections.get(section_idx) {
            *sec_lock.write().unwrap() = section;
        }
    }
}
