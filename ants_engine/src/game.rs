use crate::entities::Ant;
use crate::entities::Hill;
use crate::map::Map;
use std::fs;

/// The Ants game.
/// Main entry point for running the game.
pub struct Game {
    map: Map,
    map_contents: String,
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
                map_contents: contents,
            },
            Err(e) => panic!("Could not read map file {} due to: {}", map_file, e),
        }
    }

    /// Starts the game.
    pub fn start(&mut self) {
        self.map = Map::parse(&self.map_contents);

        // Spawn 1 ant per ant hill
        let ant_hills = self
            .map
            .ant_hills()
            .into_iter()
            .map(|(hill, row, col)| (hill.player(), row, col))
            .collect();
        spawn_ants(&mut self.map, ant_hills);
    }
}

fn spawn_ants(map: &mut Map, ant_hills: Vec<(usize, usize, usize)>) {
    for (player, row, col) in ant_hills {
        map.set(
            row,
            col,
            Box::new(Ant::from_ant_hill(player, Box::new(Hill::new(player)))),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Food;
    use std::path::Path;

    #[test]
    fn when_starting_a_game_the_map_is_reset() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap());

        game.map.set(0, 0, Box::new(Food));
        game.start();

        // The example map has water at (0, 0)
        assert_eq!(game.map.get(0, 0).as_ref().unwrap().name(), "Water");
    }

    #[test]
    fn when_starting_a_game_ants_are_spawned_on_ant_hills() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap());

        game.start();

        // The example map has 1 ant hill at (0, 1) for player 1
        let ant = game.map.get(0, 1).as_ref().unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player(), 1);
        assert_eq!(ant.is_alive(), true);
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player(), 1);

        // The example map has 1 ant hill at (3, 2) for player 0
        let ant = game.map.get(3, 2).as_ref().unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player(), 0);
        assert_eq!(ant.is_alive(), true);
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player(), 0);
    }
}
