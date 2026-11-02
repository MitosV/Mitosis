#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Palette {
    Single(u32),
    Indirect(Vec<u32>),
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkSection {
    pub non_air_blocks: u16,
    pub palette: Palette,
    pub data: Vec<u64>,
    pub bits_per_entry: u8,
}

impl ChunkSection {
    pub fn empty() -> Self {
        Self {
            non_air_blocks: 0,
            palette: Palette::Single(0), // Air
            data: Vec::new(),
            bits_per_entry: 0,
        }
    }

    pub fn new_single(block_state: u32) -> Self {
        Self {
            non_air_blocks: if block_state != 0 { 4096 } else { 0 },
            palette: Palette::Single(block_state),
            data: Vec::new(),
            bits_per_entry: 0,
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u32 {
        let index = (y * 16 + z) * 16 + x;
        match &self.palette {
            Palette::Single(state) => *state,
            Palette::Indirect(palette) => {
                if self.bits_per_entry == 0 || self.data.is_empty() {
                    return palette.first().copied().unwrap_or(0);
                }
                let palette_idx = self.read_bit_packed(index);
                palette.get(palette_idx as usize).copied().unwrap_or(0)
            }
            Palette::Global => {
                if self.bits_per_entry == 0 || self.data.is_empty() {
                    return 0;
                }
                self.read_bit_packed(index)
            }
        }
    }

    #[inline]
    fn read_bit_packed(&self, index: usize) -> u32 {
        let bpe = self.bits_per_entry as usize;
        let entries_per_long = 64 / bpe;
        let long_idx = index / entries_per_long;
        let bit_offset = (index % entries_per_long) * bpe;
        let mask = (1u64 << bpe) - 1;

        if let Some(&val) = self.data.get(long_idx) {
            ((val >> bit_offset) & mask) as u32
        } else {
            0
        }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, state: u32) {
        // Expand to Global palette if needed for fast updates
        if let Palette::Single(current) = self.palette {
            if current == state {
                return;
            }
            // Upgrade to 15-bit Global palette (4096 entries)
            self.bits_per_entry = 15;
            self.palette = Palette::Global;
            let entries_per_long = 64 / 15; // 4
            let total_longs = (4096 + entries_per_long - 1) / entries_per_long;
            self.data = vec![0u64; total_longs];
            for i in 0..4096 {
                self.write_bit_packed(i, current);
            }
        }
        let index = (y * 16 + z) * 16 + x;
        self.write_bit_packed(index, state);
    }

    #[inline]
    fn write_bit_packed(&mut self, index: usize, val: u32) {
        let bpe = self.bits_per_entry as usize;
        if bpe == 0 {
            return;
        }
        let entries_per_long = 64 / bpe;
        let long_idx = index / entries_per_long;
        let bit_offset = (index % entries_per_long) * bpe;
        let mask = (1u64 << bpe) - 1;

        if long_idx < self.data.len() {
            self.data[long_idx] &= !(mask << bit_offset);
            self.data[long_idx] |= ((val as u64) & mask) << bit_offset;
        }
    }
}
