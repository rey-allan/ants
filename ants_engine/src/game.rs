use crate::map::Map;
use std::fs;

/// The Ants game.
/// Main entry point for running the game.
pub struct Game {
    map: Map,
    starting_map: Map,
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
                starting_map: Map::parse(&contents),
            },
            Err(e) => panic!("Could not read map file {} due to: {}", map_file, e),
        }
    }

    /// Starts the game.
    pub fn start(&mut self) {
        // Reset the map to the starting state
        self.map = self.starting_map.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Cell;
    use std::path::Path;

    #[test]
    fn when_starting_a_game_the_map_is_reset() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap());

        game.map.set(0, 0, Cell::Food);
        game.start();

        assert_eq!(game.map, game.starting_map);
    }
}
