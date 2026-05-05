pub const CHUNK_SIZE: i32 = 64;

struct BitIter(u64);

impl Iterator for BitIter {
    type Item = i32;

    #[inline]
    fn next(&mut self) -> Option<i32> {
        if self.0 == 0 {
            None
        } else {
            let pos = self.0.trailing_zeros() as i32;
            self.0 &= self.0 - 1;
            Some(pos)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    rows: [u64; 64],
}

#[allow(dead_code)] // стоит здесь ибо импортируется в world.rs/ rule.rs. А в другие модули нет чтобы не летели попусту warning
impl Chunk {
    pub fn new() -> Self {
        Self { rows: [0u64; 64] }
    }

    #[inline]
    pub fn get_cell(&self, x: i32, y: i32) -> bool {
        debug_assert!(x >= 0 && x < 64 && y >= 0 && y < 64);
        (self.rows[y as usize] >> x) & 1 == 1
    }

    #[inline]
    pub fn set_cell(&mut self, x: i32, y: i32, alive: bool) {
        debug_assert!(x >= 0 && x < 64 && y >= 0 && y < 64);
        if alive {
            self.rows[y as usize] |= 1u64 << x;
        } else {
            self.rows[y as usize] &= !(1u64 << x);
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|&r| r == 0)
    }

    pub fn active_count(&self) -> usize {
        self.rows.iter().map(|r| r.count_ones() as usize).sum()
    }

    pub fn iter_active(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.rows.iter().enumerate().flat_map(|(y, &row)| {
            BitIter(row).map(move |x| (x, y as i32))
        })
    }

    pub fn clear(&mut self) {
        self.rows = [0u64; 64];
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
