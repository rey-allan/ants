use crate::entities::Ant;
use crate::entities::Entity;
use crate::entities::Food;
use crate::entities::Hill;
use crate::map::Map;
use rand::seq::SliceRandom;
use std::fs;

/// The Ants game.
/// Main entry point for running the game.
pub struct Game {
    map: Map,
    map_contents: String,
    fov_radius: usize,
    turn: usize,
    scores: Vec<usize>,
}

/// Represents the state of the game.
pub struct GameState {
    /// The current turn.
    turn: usize,
    /// The scores for each player where the index is the player number.
    scores: Vec<usize>,
    /// The ants for each player where the index is the player number.
    ants: Vec<Vec<PlayerAnt>>,
}

#[derive(Clone)]
struct StateEntity {
    name: String,
    row: usize,
    col: usize,
    player: Option<usize>,
    alive: Option<bool>,
}

#[derive(Clone)]
struct PlayerAnt {
    id: String,
    row: usize,
    col: usize,
    player: usize,
    alive: bool,
    field_of_vision: Vec<StateEntity>,
}

impl Game {
    /// Creates a new game.
    ///
    /// # Arguments
    /// * `map_file` - The path to the file containing the map.
    /// * `fov_radius` - The radius of the field of vision for each ant.
    pub fn new(map_file: &str, fov_radius: usize) -> Game {
        match fs::read_to_string(map_file) {
            Ok(contents) => {
                let map = Map::parse(&contents);
                let players = map.players();

                Game {
                    map,
                    map_contents: contents,
                    fov_radius,
                    turn: 0,
                    scores: vec![0; players],
                }
            }
            Err(e) => panic!("Could not read map file {} due to: {}", map_file, e),
        }
    }

    /// Starts the game.
    pub fn start(&mut self) -> GameState {
        self.turn = 0;
        self.map = Map::parse(&self.map_contents);

        let ant_hills: Vec<(usize, usize, usize)> = self
            .map
            .ant_hills()
            .into_iter()
            .map(|(hill, row, col)| (hill.player().unwrap(), row, col))
            .collect();

        // For each ant hill, collect up to 3 random land cells around it
        let mut rng = &mut rand::thread_rng();
        let lands: Vec<(usize, usize)> = ant_hills
            .iter()
            .flat_map(|(_, row, col)| {
                self.map
                    .land_around(*row, *col)
                    .choose_multiple(&mut rng, 3)
                    .cloned()
                    .collect::<Vec<(usize, usize)>>()
            })
            .collect();

        // Spawn 1 ant per ant hill
        spawn_ants(&mut self.map, ant_hills);
        // Spawn food on the random land cells
        spawn_food(&mut self.map, lands);

        // Compute the intial game state
        self.game_state()
    }

    /// Draws the game to the console.
    pub fn draw(&self) {
        self.map.draw();
    }

    fn game_state(&self) -> GameState {
        let players = self.map.players();
        let ants = self
            .map
            .ants()
            .into_iter()
            .filter(|(ant, _, _)| ant.alive().unwrap())
            .map(|(ant, row, col)| PlayerAnt {
                id: ant.id().to_string(),
                row,
                col,
                player: ant.player().unwrap(),
                alive: ant.alive().unwrap(),
                field_of_vision: self
                    .map
                    .field_of_vision((row, col), self.fov_radius)
                    .into_iter()
                    .map(|(entity, row, col)| to_state_entity(entity, row, col))
                    .collect(),
            })
            // Group ants by player
            .fold(vec![vec![]; players], |mut acc, ant| {
                acc[ant.player].push(ant);
                acc
            });

        GameState {
            turn: self.turn,
            scores: self.scores.clone(),
            ants,
        }
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

fn spawn_food(map: &mut Map, locations: Vec<(usize, usize)>) {
    for (row, col) in locations {
        map.set(row, col, Box::new(Food));
    }
}

fn to_state_entity(entity: &dyn Entity, row: usize, col: usize) -> StateEntity {
    StateEntity {
        name: entity.name().to_string(),
        row,
        col,
        player: entity.player(),
        alive: entity.alive(),
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
        let mut game = Game::new(map_file.to_str().unwrap(), 2);

        game.map.set(0, 0, Box::new(Food));
        game.start();

        // The example map has water at (0, 0)
        assert_eq!(game.map.get(0, 0).as_ref().unwrap().name(), "Water");
    }

    #[test]
    fn when_starting_a_game_ants_are_spawned_on_ant_hills() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap(), 2);

        game.start();

        // The example map has 1 ant hill at (0, 1) for player 1
        let ant = game.map.get(0, 1).as_ref().unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player().unwrap(), 1);
        assert!(ant.alive().unwrap());
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player().unwrap(), 1);

        // The example map has 1 ant hill at (3, 2) for player 0
        let ant = game.map.get(3, 2).as_ref().unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player().unwrap(), 0);
        assert!(ant.alive().unwrap());
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player().unwrap(), 0);
    }

    #[test]
    fn when_starting_a_game_food_is_spawned_around_land_locations_for_each_ant_hill() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap(), 2);

        game.start();

        // The example map has 1 ant hill at (0, 1) for player 1 with 3 land cells around it
        // So food should be spawned at (0, 2), (1, 1), and (1, 2)
        assert_eq!(game.map.get(0, 2).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(1, 1).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(1, 2).as_ref().unwrap().name(), "Food");

        // The example map has 1 ant hill at (3, 2) for player 0 with 3 land cells around it
        // So food should be spawned at (2, 1), (2, 2), and (3, 1)
        assert_eq!(game.map.get(2, 1).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(2, 2).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(3, 1).as_ref().unwrap().name(), "Food");
    }

    #[test]
    fn when_starting_a_game_the_correct_game_state_is_returned() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap(), 2);

        let state = game.start();

        assert_eq!(state.turn, 0);

        // The example map has 2 players
        assert_eq!(state.scores, vec![0, 0]);
        assert_eq!(state.ants.len(), 2);

        // The example map has 1 ant hill at (3, 2) for player 0
        assert_eq!(state.ants[0].len(), 1);
        assert_eq!(state.ants[0][0].row, 3);
        assert_eq!(state.ants[0][0].col, 2);
        assert_eq!(state.ants[0][0].player, 0);
        assert!(state.ants[0][0].alive);
        // Given the fov radius of 2, the ant at (3, 2) should see 7 entities
        assert_eq!(state.ants[0][0].field_of_vision.len(), 7);
        // Let's check that it was able to see the water next to it at (3, 3)
        assert!(state.ants[0][0]
            .field_of_vision
            .iter()
            .any(|entity| entity.name == "Water" && entity.row == 3 && entity.col == 3));

        // The example map has 1 ant hill at (0, 1) for player 1
        assert_eq!(state.ants[1].len(), 1);
        assert_eq!(state.ants[1][0].row, 0);
        assert_eq!(state.ants[1][0].col, 1);
        assert_eq!(state.ants[1][0].player, 1);
        assert!(state.ants[1][0].alive);
        // Given the fov radius of 2, the ant at (0, 1) should see 7 entities
        assert_eq!(state.ants[1][0].field_of_vision.len(), 7);
        // Let's check that it was able to see the water next to it at (0, 0)
        assert!(state.ants[1][0]
            .field_of_vision
            .iter()
            .any(|entity| entity.name == "Water" && entity.row == 0 && entity.col == 0));
    }
}
