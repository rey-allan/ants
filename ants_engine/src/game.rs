use crate::entities::Ant;
use crate::entities::Entity;
use crate::entities::Food;
use crate::entities::Hill;
use crate::map::Map;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::{HashMap, HashSet};
use std::fs;

/// The Ants game.
/// Main entry point for running the game.
pub struct Game {
    map: Map,
    map_contents: String,
    fov_radius2: usize,
    attack_radius2: usize,
    food_radius2: usize,
    turn: usize,
    scores: Vec<usize>,
    hive: Vec<usize>,
    food_per_turn: usize,
    rng: StdRng,
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
    /// * `food_radius2` - The radius **squared** of the range around ants to harvest food.
    /// * `food_rate` - The amount of food to spawn *per player* on each round.
    /// * `seed` - The seed for the random number generator.
    pub fn new(
        map_file: &str,
        fov_radius2: usize,
        attack_radius2: usize,
        food_radius2: usize,
        food_rate: usize,
        seed: u64,
    ) -> Game {
        match fs::read_to_string(map_file) {
            Ok(contents) => {
                let map = Map::parse(&contents);
                let players = map.players();

                Game {
                    map,
                    map_contents: contents,
                    fov_radius2,
                    attack_radius2,
                    food_radius2,
                    turn: 0,
                    scores: vec![0; players],
                    hive: vec![0; players],
                    food_per_turn: food_rate * players,
                    // Initialize the `rng` with a seed of 0
                    rng: StdRng::seed_from_u64(seed),
                }
            }
            Err(e) => panic!("Could not read map file {} due to: {}", map_file, e),
        }
    }

    /// Starts the game.
    pub fn start(&mut self) -> GameState {
        self.turn = 0;
        self.map = Map::parse(&self.map_contents);

        // Spawn food around all ant hills
        spawn_food_around_hills(&mut self.map, &mut self.rng);
        // Spawn 1 ant per ant hill
        spawn_ants_all_hills(&mut self.map);

        // Compute the intial game state
        self.game_state()
    }

    /// Updates the game state based on the actions provided for each ant.
    pub fn update(&mut self, actions: Vec<Action>) -> GameState {
        remove_dead_ants(&mut self.map);
        move_ants(&mut self.map, actions);
        attack(&mut self.map, self.attack_radius2);
        raze_hills(&mut self.map);
        spawn_ants_from_hive(&mut self.map, &mut self.hive, &mut self.rng);
        harvest_food(&mut self.map, &mut self.hive, self.food_radius2);
        // Opted for spawning food randomly across the map instead of doing the symmetric spawning that the original Ants game used.
        // The reason is that random food makes the game more challenging as it could lead to scenarios where agents aren't near any food.
        // This will require better learning and handling of complex world states.
        // Which we hope will ultimately lead to more robust agents.
        spawn_food_randomly(&mut self.map, &mut self.rng, self.food_per_turn);

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

fn spawn_ants_all_hills(map: &mut Map) {
    spawn_ants(map, live_ant_hills(map));
}

fn spawn_ants_from_hive(map: &mut Map, hive: &mut [usize], rng: &mut StdRng) {
    let players = map.players();
    let hills_by_player = live_ant_hills(map)
        .into_iter()
        // Group hills by player
        .fold(vec![vec![]; players], |mut acc, hill| {
            acc[hill.0].push(hill);
            acc
        });

    for player in 0..players {
        let hills = &hills_by_player[player];
        let available_food = hive[player];

        if available_food == 0 {
            continue;
        }

        // Randomly choose hills, up to the available food, to spawn ants on
        // We do this withouth repetition to avoid spawning multiple ants on the same hill
        let ant_hills = hills.choose_multiple(rng, available_food);

        // Update the hive with the remaining food
        hive[player] -= ant_hills.len();

        // Spawn ants on the chosen hills
        spawn_ants(map, ant_hills.cloned().collect());
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

fn spawn_food_around_hills(map: &mut Map, rng: &mut StdRng) {
    let ant_hills = live_ant_hills(map);

    // For each ant hill, collect up to 3 random land cells around it
    let lands: Vec<(usize, usize)> = ant_hills
        .iter()
        .flat_map(|(_, row, col)| {
            map.land_around(*row, *col)
                .choose_multiple(rng, 3)
                .cloned()
                .collect::<Vec<(usize, usize)>>()
        })
        .collect();

    // Spawn food on the random land cells
    spawn_food(map, lands);
}

fn spawn_food_randomly(map: &mut Map, rng: &mut StdRng, food_per_turn: usize) {
    let land = map.land();
    let food_locations = land.choose_multiple(rng, food_per_turn).cloned().collect();

    spawn_food(map, food_locations);
}

fn spawn_food(map: &mut Map, locations: Vec<(usize, usize)>) {
    for (row, col) in locations {
        map.set(row, col, Box::new(Food));
    }
}

fn remove_dead_ants(map: &mut Map) {
    let dead_ants = map
        .ants()
        .into_iter()
        .filter(|(ant, _, _)| !ant.alive().unwrap())
        .map(|(_, row, col)| (row, col))
        .collect::<Vec<(usize, usize)>>();

    for (row, col) in dead_ants {
        map.remove(row, col);
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

fn harvest_food(map: &mut Map, hive: &mut [usize], food_radius2: usize) {
    let food = map.food();

    for (row, col) in food {
        let unique_player_ants_around_food = map
            .field_of_vision((row, col), food_radius2)
            .into_iter()
            .filter(|(entity, _, _)| entity.name() == "Ant")
            .map(|(entity, _, _)| entity.player().unwrap())
            .collect::<HashSet<usize>>();

        if unique_player_ants_around_food.is_empty() {
            continue;
        }

        // If there is only one player around the food, they consume it into their hive
        if unique_player_ants_around_food.len() == 1 {
            let player = unique_player_ants_around_food.iter().next().unwrap();
            hive[*player] += 1;
        }

        map.remove(row, col);
    }
}

fn live_ant_hills(map: &Map) -> Vec<(usize, usize, usize)> {
    map.ant_hills()
        .into_iter()
        .filter(|(hill, _, _)| hill.alive().unwrap())
        .map(|(hill, row, col)| (hill.player().unwrap(), row, col))
        .collect()
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
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4, 1, 5, 0);

        game.map.set(0, 0, Box::new(Food));
        game.start();

        // The example map has water at (0, 0)
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Water");
    }

    #[test]
    fn when_starting_a_game_ants_are_spawned_on_ant_hills() {
        let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4, 1, 5, 0);

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
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4, 1, 5, 0);

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
        let mut game = Game::new(map_file.to_str().unwrap(), 4, 4, 1, 5, 0);

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
    fn when_removing_dead_ants_all_dead_ants_are_removed() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m a.";
        let mut map = Map::parse(map);

        assert!(map.get(1, 0).unwrap().alive().unwrap());
        map.get_mut(1, 0).unwrap().set_alive(false);

        remove_dead_ants(&mut map);

        assert!(map.get(1, 0).is_none());
    }

    #[test]
    fn when_removing_dead_ants_if_there_are_no_dead_ants_no_ants_are_removed() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m a.";
        let mut map = Map::parse(map);

        assert!(map.get(1, 0).unwrap().alive().unwrap());

        remove_dead_ants(&mut map);

        assert!(map.get(1, 0).is_some());
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

    #[test]
    fn when_spawning_ants_from_hive_if_there_is_no_food_no_ants_are_spawned() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 01
            m ..";
        let mut map = Map::parse(map);
        let mut hive = vec![0, 0];

        spawn_ants_from_hive(&mut map, &mut hive, &mut StdRng::seed_from_u64(0));

        assert_eq!(map.get(0, 0).unwrap().name(), "Hill");
        assert_eq!(map.get(0, 1).unwrap().name(), "Hill");
        assert_eq!(hive, vec![0, 0]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_the_hill_is_razed_no_ants_are_spawned() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m ..";
        let mut map = Map::parse(map);
        let mut hive = vec![1];

        // Raze the hill
        map.get_mut(0, 0).unwrap().set_alive(false);

        spawn_ants_from_hive(&mut map, &mut hive, &mut StdRng::seed_from_u64(0));

        assert_eq!(map.get(0, 0).unwrap().name(), "Hill");
        assert_eq!(hive, vec![1]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_there_is_food_and_one_hill_one_ant_is_spawned() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 01
            m ..";
        let mut map = Map::parse(map);
        let mut hive = vec![1, 1];

        spawn_ants_from_hive(&mut map, &mut hive, &mut StdRng::seed_from_u64(0));

        assert_eq!(map.get(0, 0).unwrap().name(), "Ant");
        assert_eq!(map.get(0, 0).unwrap().player().unwrap(), 0);
        assert_eq!(map.get(0, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(0, 1).unwrap().player().unwrap(), 1);
        assert_eq!(hive, vec![0, 0]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_there_is_more_food_and_one_hill_only_one_ant_is_spawned() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m ..";
        let mut map = Map::parse(map);
        let mut hive = vec![5];

        spawn_ants_from_hive(&mut map, &mut hive, &mut StdRng::seed_from_u64(0));

        assert_eq!(map.get(0, 0).unwrap().name(), "Ant");
        assert_eq!(hive, vec![4]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_there_is_only_one_food_and_multiple_hills_only_one_ant_is_spawned(
    ) {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m .0";
        let mut map = Map::parse(map);
        let mut hive = vec![1];

        spawn_ants_from_hive(&mut map, &mut hive, &mut StdRng::seed_from_u64(0));

        // Hill is chosen at random and we make it predictable based on the seed
        assert_eq!(map.get(0, 0).unwrap().name(), "Hill");
        assert_eq!(map.get(1, 1).unwrap().name(), "Ant");
        assert_eq!(hive, vec![0]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_there_is_enough_food_and_multiple_hills_one_ant_is_spawned_per_hill(
    ) {
        let map = "\
            rows 2
            cols 2
            players 2
            m 01
            m 10";
        let mut map = Map::parse(map);
        let mut hive = vec![5, 2];

        spawn_ants_from_hive(&mut map, &mut hive, &mut StdRng::seed_from_u64(0));

        assert_eq!(map.get(0, 0).unwrap().name(), "Ant");
        assert_eq!(map.get(0, 0).unwrap().player().unwrap(), 0);
        assert_eq!(map.get(1, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(1, 1).unwrap().player().unwrap(), 0);

        assert_eq!(map.get(0, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(0, 1).unwrap().player().unwrap(), 1);
        assert_eq!(map.get(1, 0).unwrap().name(), "Ant");
        assert_eq!(map.get(1, 0).unwrap().player().unwrap(), 1);

        assert_eq!(hive, vec![3, 0]);
    }

    #[test]
    fn when_harvesting_food_if_there_are_no_ants_around_the_food_nothing_happens() {
        let map = "\
            rows 3
            cols 3
            players 1
            m *..
            m .*.
            m ..*";
        let mut map = Map::parse(map);
        let mut hive = vec![0];

        harvest_food(&mut map, &mut hive, 1);

        assert_eq!(map.get(0, 0).unwrap().name(), "Food");
        assert_eq!(map.get(1, 1).unwrap().name(), "Food");
        assert_eq!(map.get(2, 2).unwrap().name(), "Food");
        assert_eq!(hive, vec![0]);
    }

    #[test]
    fn when_harvesting_food_if_there_only_ants_from_the_same_player_around_the_food_the_food_is_harvested_into_the_hive(
    ) {
        let map = "\
            rows 3
            cols 3
            players 2
            m *ab
            m .aa
            m b.*";
        let mut map = Map::parse(map);
        let mut hive = vec![0, 0];

        harvest_food(&mut map, &mut hive, 1);

        assert!(map.get(0, 0).is_none());
        assert!(map.get(2, 2).is_none());
        assert_eq!(hive, vec![2, 0]);
    }

    #[test]
    fn when_harvesting_food_if_there_are_ants_from_different_players_around_the_food_the_food_is_destroyed(
    ) {
        let map = "\
            rows 3
            cols 3
            players 2
            m *a.
            m b.a
            m .b*";
        let mut map = Map::parse(map);
        let mut hive = vec![0, 0];

        harvest_food(&mut map, &mut hive, 1);

        assert!(map.get(0, 0).is_none());
        assert!(map.get(2, 2).is_none());
        assert_eq!(hive, vec![0, 0]);
    }

    #[test]
    fn when_spawning_food_randomly_and_there_is_enough_land_all_food_is_spawned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m ...";
        let mut map = Map::parse(map);

        spawn_food_randomly(&mut map, &mut StdRng::seed_from_u64(0), 8);

        let food = map.food();
        let expected_food = vec![
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 2),
            (2, 0),
            (2, 1),
            (2, 2),
        ];

        assert_eq!(food.len(), 8);
        assert_eq!(food, expected_food);
    }

    #[test]
    fn when_spawning_food_randomly_and_there_is_not_enough_land_not_all_food_is_spawned() {
        let map = "\
            rows 3
            cols 3
            players 2
            m aa.
            m .a.
            m b.b";
        let mut map = Map::parse(map);

        spawn_food_randomly(&mut map, &mut StdRng::seed_from_u64(0), 9);

        let food = map.food();
        let expected_food = vec![(0, 2), (1, 0), (1, 2), (2, 1)];

        assert_eq!(food.len(), 4);
        assert_eq!(food, expected_food);
    }

    #[test]
    fn when_spawning_food_randomly_and_there_is_no_land_no_food_is_spawned() {
        let map = "\
            rows 3
            cols 3
            players 2
            m aaa
            m aaa
            m aba";
        let mut map = Map::parse(map);

        spawn_food_randomly(&mut map, &mut StdRng::seed_from_u64(0), 9);
        assert!(map.food().is_empty());
    }
}
