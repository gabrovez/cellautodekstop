use super::coords::Coord;
use super::world::World;
use rayon::prelude::*;
use std::collections::HashSet;

pub struct LifeRule;

impl LifeRule {
    pub fn new() -> Self {
        Self
    }

    pub fn apply_into(&self, current: &World, next: &mut World) {
        next.clear();

        let candidate_chunks = current.candidate_chunk_coords();

        let mut candidates: HashSet<Coord> = HashSet::new();
        for (cx, cy) in &candidate_chunks {
            for coord in current.iter_active_in_chunk(*cx, *cy) {
                candidates.insert(coord);
                for nb in coord.neighbors() {
                    candidates.insert(nb);
                }
            }
        }

        let candidates: Vec<Coord> = candidates.into_iter().collect();
        
        let alive: Vec<Coord> = candidates
            .par_iter()
            .filter_map(|&coord| {
                let is_alive = current.get_cell(coord);
                let neighbors = current.count_neighbors(coord);

                match (is_alive, neighbors) {
                    (true, 2) | (true, 3) => Some(coord), 
                    (false, 3) => Some(coord),             
                    _ => None,
                }
            })
            .collect();

        for coord in alive {
            next.set_cell(coord, true);
        }

        next.recompute_dirty(current);
    }
}

impl Default for LifeRule {
    fn default() -> Self {
        Self::new()
    }
}
