use std::ops::{Add, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Coord {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn chunk_coord(&self, chunk_size: i32) -> (i32, i32) {
        (
            self.x.div_euclid(chunk_size),
            self.y.div_euclid(chunk_size),
        )
    }

    pub fn local_coord(&self, chunk_size: i32) -> (i32, i32) {
        (
            self.x.rem_euclid(chunk_size),
            self.y.rem_euclid(chunk_size),
        )
    }

    pub fn neighbors(&self) -> [Coord; 8] {
        [
            Coord::new(self.x - 1, self.y - 1), 
            Coord::new(self.x, self.y - 1),     
            Coord::new(self.x + 1, self.y - 1), 
            Coord::new(self.x - 1, self.y),    
            Coord::new(self.x + 1, self.y),     
            Coord::new(self.x - 1, self.y + 1), 
            Coord::new(self.x, self.y + 1),    
            Coord::new(self.x + 1, self.y + 1),
        ]
    }
}

impl Add for Coord {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Coord {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
