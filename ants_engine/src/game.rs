use crate::entities::Ant;
use crate::entities::Entity;
use crate::entities::Food;
use crate::entities::Hill;
use crate::map::Map;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs;

/// The Ants game.
/// Main entry point for running the game.
pub struct Game {
    map: Map,
    map_contents: String,
    fov_radius2: usize,
    attack_radius2: usize,
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

/// Represents the direction an ant can move.
pub enum Direction {
    North,
    East,
    South,
    West,
}

/// Represents an action an ant can take.
/// The action is a tuple of the ant's row, column, and direction.
/// If the direction is not a valid move, the ant will stay in place.
/// Or if the provided location is not a valid ant, the action will be ignored.
pub struct Action {
    row: usize,
    col: usize,
    direction: Direction,
}

impl Action {
    /// Creates a new action.
    ///
    /// # Arguments
    /// * `row` - The row of the ant to move.
    /// * `col` - The column of the ant to move.
    /// * `direction` - The direction the ant should move.
    pub fn new(row: usize, col: usize, direction: Direction) -> Action {
        Action {
            row,
            col,
            direction,
        }
    }
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
    /// * `fov_radius2` - The radius **squared** of the field of vision for each ant.
    /// * `attack_radius2` - The radius **squared** of the attack range for each ant.
    pub fn new(map_file: &str, fov_radius2: usize, attack_radius2: usize) -> Game {
        match fs::read_to_string(map_file) {
            Ok(contents) => {
                let map = Map::parse(&contents);
                let players = map.players();

                Game {
                    map,
                    map_contents: contents,
                    fov_radius2,
                    attack_radius2,
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

    /// Updates the game state based on the actions provided for each ant.
    pub fn update(&mut self, actions: Vec<Action>) -> GameState {
        move_ants(&mut self.map, actions);
        attack(&mut self.map, self.attack_radius2);
        raze_hills(&mut self.map);

        self.turn += 1;
        self.game_state()
    }

    /// Draws the game to the console.
    pub fn draw(&self) {
        self.map.draw(self.turn);
    }

    fn game_state(&self) -> GameState {
        let players = self.map.players();
        let ants = live_ants(&self.map)
            .into_iter()
            .map(|(ant, row, col)| PlayerAnt {
                id: ant.id().to_string(),
                row,
                col,
                player: ant.player().unwrap(),
                alive: ant.alive().unwrap(),
                field_of_vision: self
                    .map
                    .field_of_vision((row, col), self.fov_radius2)
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
            Box::new(Ant::from_ant_hill(
                player,
                Box::new(Hill::new(player, true)),
            )),
        );
    }
}

fn spawn_food(map: &mut Map, locations: Vec<(usize, usize)>) {
    for (row, col) in locations {
        map.set(row, col, Box::new(Food));
    }
}

fn move_ants(map: &mut Map, actions: Vec<Action>) {
    for action in actions {
        let (to_row, to_col) = match action.direction {
            Direction::North => (action.row.saturating_sub(1), action.col),
            Direction::East => (action.row, action.col + 1),
            Direction::South => (action.row + 1, action.col),
            Direction::West => (action.row, action.col.saturating_sub(1)),
        };

        map.move_entity((action.row, action.col), (to_row, to_col));
    }
}

fn attack(map: &mut Map, attack_radius: usize) {
    // Pre-calculate the number of enemies for each live ant as a map of ant `id` to the Vec of enemies
    let ants = live_ants(map);
    let enemies: HashMap<String, Vec<(&dyn Entity, usize, usize)>> = ants
        .iter()
        .map(|(ant, row, col)| {
            let fov = map.field_of_vision((*row, *col), attack_radius);
            let enemies = enemies(fov, ant.player().unwrap());
            (ant.id().to_string(), enemies)
        })
        .collect();

    // Determine which ants to kill
    let mut to_kill = Vec::new();
    for (ant, row, col) in ants {
        let ant_enemies = enemies.get(ant.id()).unwrap();
        let focus = ant_enemies.len();

        if focus == 0 {
            continue;
        }

        // Find the enemy with the most attention power, i.e. the enemy with the least other ants focused on it
        let min_enemy_focus = ant_enemies
            .iter()
            .map(|(enemy, _, _)| enemies.get(enemy.id()).unwrap().len())
            .min()
            .unwrap();

        // Ant dies if its focused on more or equal enemies than its enemy with the most attention power
        if focus >= min_enemy_focus {
            to_kill.push((row, col));
        }
    }

    // After all battles are resolved, kill the ants
    for (row, col) in to_kill {
        map.get_mut(row, col).unwrap().set_alive(false);
    }
}

fn raze_hills(map: &mut Map) {
    let ants = live_ants(map);
    let hills_to_raze: Vec<(usize, usize, usize)> = ants
        .into_iter()
        .filter_map(|(ant, row, col)| {
            // If the ant is on an ant hill that is not its own, the hill should be razed
            if ant.on_ant_hill().is_some()
                && ant.player().unwrap() != ant.on_ant_hill().as_ref().unwrap().player().unwrap()
            {
                let hill_owner = ant.on_ant_hill().as_ref().unwrap().player().unwrap();
                Some((hill_owner, row, col))
            } else {
                None
            }
        })
        .collect();

    for (player, row, col) in hills_to_raze {
        map.get_mut(row, col)
            .unwrap()
            .set_on_ant_hill(Box::new(Hill::new(player, false)));
    }
}

fn live_ants(map: &Map) -> Vec<(&dyn Entity, usize, usize)> {
    map.ants()
        .into_iter()
        .filter(|(ant, _, _)| ant.alive().unwrap())
        .collect()
}

fn enemies(
    field_of_vision: Vec<(&dyn Entity, usize, usize)>,
    player: usize,
) -> Vec<(&dyn Entity, usize, usize)> {
    field_of_vision
        .into_iter()
        .filter(|(entity, _, _)| {
            entity.name() == "Ant"
                && entity.player().is_some()
                && entity.player().unwrap() != player
        })
        .collect()
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
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4);

        game.map.set(0, 0, Box::new(Food));
        game.start();

        // The example map has water at (0, 0)
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Water");
    }

    #[test]
    fn when_starting_a_game_ants_are_spawned_on_ant_hills() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4);

        game.start();

        // The example map has 1 ant hill at (0, 1) for player 1
        let ant = game.map.get(0, 1).unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player().unwrap(), 1);
        assert!(ant.alive().unwrap());
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player().unwrap(), 1);

        // The example map has 1 ant hill at (3, 2) for player 0
        let ant = game.map.get(3, 2).unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player().unwrap(), 0);
        assert!(ant.alive().unwrap());
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player().unwrap(), 0);
    }

    #[test]
    fn when_starting_a_game_food_is_spawned_around_land_locations_for_each_ant_hill() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4);

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
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4);

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

    #[test]
    fn when_attacking_on_a_one_on_one_battle_both_ants_die() {
        let map = "\
            rows 3
            cols 5
            players 2
            m .....
            m .a.b.
            m .....";
        let mut map = Map::parse(map);

        assert!(map.get(1, 1).unwrap().alive().unwrap());
        assert!(map.get(1, 3).unwrap().alive().unwrap());

        attack(&mut map, 5);

        assert!(!map.get(1, 1).unwrap().alive().unwrap());
        assert!(!map.get(1, 3).unwrap().alive().unwrap());
    }

    #[test]
    fn when_attacking_on_a_two_on_one_battle_ant_a_dies() {
        let map = "\
            rows 3
            cols 5
            players 2
            m ...b.
            m .a...
            m ...b.";
        let mut map = Map::parse(map);

        assert!(map.get(0, 3).unwrap().alive().unwrap());
        assert!(map.get(1, 1).unwrap().alive().unwrap());
        assert!(map.get(2, 3).unwrap().alive().unwrap());

        attack(&mut map, 5);

        assert!(map.get(0, 3).unwrap().alive().unwrap());
        assert!(!map.get(1, 1).unwrap().alive().unwrap());
        assert!(map.get(2, 3).unwrap().alive().unwrap());
    }

    #[test]
    fn when_attacking_on_a_one_on_one_on_one_battle_all_ants_die() {
        let map = "\
            rows 3
            cols 5
            players 3
            m ...b.
            m .a...
            m ...c.";
        let mut map = Map::parse(map);

        assert!(map.get(0, 3).unwrap().alive().unwrap());
        assert!(map.get(1, 1).unwrap().alive().unwrap());
        assert!(map.get(2, 3).unwrap().alive().unwrap());

        attack(&mut map, 5);

        assert!(!map.get(0, 3).unwrap().alive().unwrap());
        assert!(!map.get(1, 1).unwrap().alive().unwrap());
        assert!(!map.get(2, 3).unwrap().alive().unwrap());
    }

    #[test]
    fn when_attacking_on_an_ant_sandwich_battle_the_middle_ant_dies() {
        let map = "\
            rows 3
            cols 5
            players 2
            m .....
            m a.b.c
            m .....";
        let mut map = Map::parse(map);

        assert!(map.get(1, 0).unwrap().alive().unwrap());
        assert!(map.get(1, 2).unwrap().alive().unwrap());
        assert!(map.get(1, 4).unwrap().alive().unwrap());

        attack(&mut map, 5);

        assert!(map.get(1, 0).unwrap().alive().unwrap());
        assert!(!map.get(1, 2).unwrap().alive().unwrap());
        assert!(map.get(1, 4).unwrap().alive().unwrap());
    }

    #[test]
    fn when_attacking_on_a_one_on_two_on_one_battle_ants_b_and_c_die() {
        let map = "\
            rows 3
            cols 5
            players 3
            m ...b.
            m .a.a.
            m ...c.";
        let mut map = Map::parse(map);

        assert!(map.get(0, 3).unwrap().alive().unwrap());
        assert!(map.get(1, 1).unwrap().alive().unwrap());
        assert!(map.get(1, 3).unwrap().alive().unwrap());
        assert!(map.get(2, 3).unwrap().alive().unwrap());

        attack(&mut map, 5);

        assert!(!map.get(0, 3).unwrap().alive().unwrap());
        assert!(map.get(1, 1).unwrap().alive().unwrap());
        assert!(map.get(1, 3).unwrap().alive().unwrap());
        assert!(!map.get(2, 3).unwrap().alive().unwrap());
    }

    #[test]
    fn when_attacking_on_a_wall_punch_battle_many_ants_die() {
        let map = "\
            rows 3
            cols 9
            players 2
            m aaaaaaaaa
            m ...bbb...
            m ...bbb...";
        let mut map = Map::parse(map);

        assert!(map.get(0, 0).unwrap().alive().unwrap());
        assert!(map.get(0, 1).unwrap().alive().unwrap());
        assert!(map.get(0, 2).unwrap().alive().unwrap());
        assert!(map.get(0, 3).unwrap().alive().unwrap());
        assert!(map.get(0, 4).unwrap().alive().unwrap());
        assert!(map.get(0, 5).unwrap().alive().unwrap());
        assert!(map.get(0, 6).unwrap().alive().unwrap());
        assert!(map.get(0, 7).unwrap().alive().unwrap());
        assert!(map.get(0, 8).unwrap().alive().unwrap());
        assert!(map.get(1, 3).unwrap().alive().unwrap());
        assert!(map.get(1, 4).unwrap().alive().unwrap());
        assert!(map.get(1, 5).unwrap().alive().unwrap());
        assert!(map.get(2, 3).unwrap().alive().unwrap());
        assert!(map.get(2, 4).unwrap().alive().unwrap());
        assert!(map.get(2, 5).unwrap().alive().unwrap());

        attack(&mut map, 5);

        assert!(map.get(0, 0).unwrap().alive().unwrap());
        assert!(map.get(0, 1).unwrap().alive().unwrap());
        assert!(!map.get(0, 2).unwrap().alive().unwrap());
        assert!(!map.get(0, 3).unwrap().alive().unwrap());
        assert!(!map.get(0, 4).unwrap().alive().unwrap());
        assert!(!map.get(0, 5).unwrap().alive().unwrap());
        assert!(!map.get(0, 6).unwrap().alive().unwrap());
        assert!(map.get(0, 7).unwrap().alive().unwrap());
        assert!(map.get(0, 8).unwrap().alive().unwrap());
        assert!(!map.get(1, 3).unwrap().alive().unwrap());
        assert!(!map.get(1, 4).unwrap().alive().unwrap());
        assert!(!map.get(1, 5).unwrap().alive().unwrap());
        assert!(!map.get(2, 3).unwrap().alive().unwrap());
        assert!(map.get(2, 4).unwrap().alive().unwrap());
        assert!(!map.get(2, 5).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_does_not_have_an_ant_the_hill_is_not_razed() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m ..";
        let mut map = Map::parse(map);

        assert!(map.get(0, 0).unwrap().alive().unwrap());
        raze_hills(&mut map);
        assert!(map.get(0, 0).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_has_an_ant_of_the_same_player_the_hill_is_not_razed() {
        let map = "\
            rows 2
            cols 2
            players 2
            m A.
            m ..";
        let mut map = Map::parse(map);

        assert!(map.get(0, 0).unwrap().alive().unwrap());
        raze_hills(&mut map);
        assert!(map.get(0, 0).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_has_a_dead_enemy_ant_the_hill_is_not_razed() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m b.";
        let mut map = Map::parse(map);

        assert_eq!(map.get(0, 0).unwrap().name(), "Hill");
        assert!(map.get(0, 0).unwrap().alive().unwrap());

        // Move the enemy to the hill
        map.move_entity((1, 0), (0, 0));
        assert_eq!(map.get(0, 0).unwrap().name(), "Ant");

        // Kill the enemy
        map.get_mut(0, 0).unwrap().set_alive(false);

        raze_hills(&mut map);

        assert!(map
            .get(0, 0)
            .unwrap()
            .on_ant_hill()
            .as_ref()
            .unwrap()
            .alive()
            .unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_has_an_alive_enemy_ant_the_hill_is_razed() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m b.";
        let mut map = Map::parse(map);

        assert_eq!(map.get(0, 0).unwrap().name(), "Hill");
        assert!(map.get(0, 0).unwrap().alive().unwrap());

        // Move the enemy to the hill
        map.move_entity((1, 0), (0, 0));
        assert_eq!(map.get(0, 0).unwrap().name(), "Ant");

        raze_hills(&mut map);

        assert!(!map
            .get(0, 0)
            .unwrap()
            .on_ant_hill()
            .as_ref()
            .unwrap()
            .alive()
            .unwrap());
    }
}
