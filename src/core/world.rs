use std::collections::{HashMap, HashSet};
use super::chunks::{Chunk, CHUNK_SIZE};
use super::coords::Coord;

#[inline]
fn spread_bits(v: u32) -> u64 {
    let mut v = v as u64;
    v = (v | (v << 16)) & 0x0000_FFFF_0000_FFFF;
    v = (v | (v << 8)) & 0x00FF_00FF_00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333_3333_3333;
    v = (v | (v << 1)) & 0x5555_5555_5555_5555;
    v
}

#[inline]
fn compact_bits(mut v: u64) -> u32 {
    v &= 0x5555_5555_5555_5555;
    v = (v | (v >> 1)) & 0x3333_3333_3333_3333;
    v = (v | (v >> 2)) & 0x0F0F_0F0F_0F0F_0F0F;
    v = (v | (v >> 4)) & 0x00FF_00FF_00FF_00FF;
    v = (v | (v >> 8)) & 0x0000_FFFF_0000_FFFF;
    v = (v | (v >> 16)) & 0x0000_0000_FFFF_FFFF;
    v as u32
}

#[inline]
pub(crate) fn morton_encode(x: i32, y: i32) -> u64 {
    let ux = (x ^ i32::MIN) as u32;
    let uy = (y ^ i32::MIN) as u32;
    spread_bits(ux) | (spread_bits(uy) << 1)
}

#[inline]
pub(crate) fn morton_decode(code: u64) -> (i32, i32) {
    let ux = compact_bits(code);
    let uy = compact_bits(code >> 1);
    ((ux as i32) ^ i32::MIN, (uy as i32) ^ i32::MIN)
}

#[derive(Debug, Clone)]
pub struct World {
    chunks: HashMap<u64, Chunk>,
    dirty: HashSet<u64>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            dirty: HashSet::new(),
        }
    }

    pub fn get_cell(&self, coord: Coord) -> bool {
        let (chunk_x, chunk_y) = coord.chunk_coord(CHUNK_SIZE);
        let (local_x, local_y) = coord.local_coord(CHUNK_SIZE);
        let key = morton_encode(chunk_x, chunk_y);

        self.chunks
            .get(&key)
            .map(|chunk| chunk.get_cell(local_x, local_y))
            .unwrap_or(false)
    }

    pub fn set_cell(&mut self, coord: Coord, alive: bool) {
        let (chunk_x, chunk_y) = coord.chunk_coord(CHUNK_SIZE);
        let (local_x, local_y) = coord.local_coord(CHUNK_SIZE);
        let key = morton_encode(chunk_x, chunk_y);

        if alive {
            let chunk = self.chunks.entry(key).or_insert_with(Chunk::new);
            chunk.set_cell(local_x, local_y, true);
            self.dirty.insert(key);
        } else {
            if let Some(chunk) = self.chunks.get_mut(&key) {
                chunk.set_cell(local_x, local_y, false);
                self.dirty.insert(key);
                if chunk.is_empty() {
                    self.chunks.remove(&key);
                }
            }
        }
    }

    pub fn iter_active_cells(&self) -> impl Iterator<Item = Coord> + '_ {
        self.chunks.iter().flat_map(|(&key, chunk)| {
            let (chunk_x, chunk_y) = morton_decode(key);
            let offset_x = chunk_x * CHUNK_SIZE;
            let offset_y = chunk_y * CHUNK_SIZE;

            chunk.iter_active().map(move |(local_x, local_y)| {
                Coord::new(offset_x + local_x, offset_y + local_y)
            })
        })
    }

    pub fn get_bounds(&self) -> Option<(Coord, Coord)> {
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        let mut found = false;

        for coord in self.iter_active_cells() {
            if coord.x < min_x { min_x = coord.x; }
            if coord.x > max_x { max_x = coord.x; }
            if coord.y < min_y { min_y = coord.y; }
            if coord.y > max_y { max_y = coord.y; }
            found = true;
        }

        if found {
            Some((Coord::new(min_x, min_y), Coord::new(max_x, max_y)))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.dirty.clear();
    }

    pub fn count_neighbors(&self, coord: Coord) -> u8 {
        coord
            .neighbors()
            .iter()
            .filter(|&&neighbor| self.get_cell(neighbor))
            .count() as u8
    }

    pub fn candidate_chunk_coords(&self) -> HashSet<(i32, i32)> {
        let seeds: Vec<u64> = if !self.dirty.is_empty() {
            self.dirty.iter().copied().collect()
        } else {
            self.chunks.keys().copied().collect()
        };

        seeds
            .into_iter()
            .flat_map(|key| {
                let (cx, cy) = morton_decode(key);
                (-1i32..=1).flat_map(move |dy| (-1i32..=1).map(move |dx| (cx + dx, cy + dy)))
            })
            .collect()
    }

    pub(crate) fn iter_active_in_chunk(
        &self,
        chunk_x: i32,
        chunk_y: i32,
    ) -> impl Iterator<Item = Coord> + '_ {
        let key = morton_encode(chunk_x, chunk_y);
        let offset_x = chunk_x * CHUNK_SIZE;
        let offset_y = chunk_y * CHUNK_SIZE;

        self.chunks
            .get(&key)
            .into_iter()
            .flat_map(move |chunk| {
                chunk
                    .iter_active()
                    .map(move |(lx, ly)| Coord::new(offset_x + lx, offset_y + ly))
            })
    }

    pub fn recompute_dirty(&mut self, previous: &World) {
        self.dirty.clear();

        for (&key, next_chunk) in &self.chunks {
            match previous.chunks.get(&key) {
                Some(prev_chunk) if prev_chunk == next_chunk => {}
                _ => {
                    self.dirty.insert(key);
                }
            }
        }

        for &key in previous.chunks.keys() {
            if !self.chunks.contains_key(&key) {
                self.dirty.insert(key);
            }
        }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
