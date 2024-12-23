use crate::map::Map;
use std::fs;

/// The Ants game.
/// Main entry point for running the game.
pub struct Game {
    map: Map,
}

impl Game {
    /// Creates a new game from the a map file.
    ///
    /// # Arguments
    /// * `map_file` - The path to the file containing the map.
    pub fn new(map_file: &str) -> Game {
        match fs::read_to_string(map_file) {
            Ok(contents) => Game {
                map: Map::parse(&contents),
            },
            Err(e) => panic!("Could not read map file {} due to {}", map_file, e),
        }
    }
}
