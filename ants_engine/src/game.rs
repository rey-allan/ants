use crate::entities::Ant;
use crate::entities::Entity;
use crate::entities::Food;
use crate::entities::Hill;
use crate::map::Map;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::{HashMap, HashSet};

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
    /// * `map_contents` - The map as a string.
    /// * `fov_radius2` - The radius **squared** of the field of vision for each ant.
    /// * `attack_radius2` - The radius **squared** of the attack range for each ant.
    /// * `food_radius2` - The radius **squared** of the range around ants to harvest food.
    /// * `food_rate` - The amount of food to spawn *per player* on each round.
    /// * `seed` - The seed for the random number generator.
    pub fn new(
        map_contents: &str,
        fov_radius2: usize,
        attack_radius2: usize,
        food_radius2: usize,
        food_rate: usize,
        seed: u64,
    ) -> Game {
        let map = Map::parse(map_contents);
        let players = map.players();

        Game {
            map,
            map_contents: map_contents.to_string(),
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

    /// Starts the game.
    pub fn start(&mut self) -> GameState {
        self.turn = 0;
        self.map = Map::parse(&self.map_contents);

        self.spawn_food_around_hills();
        self.spawn_ants_all_hills();

        // Compute the intial game state
        self.game_state()
    }

    /// Updates the game state based on the actions provided for each ant.
    pub fn update(&mut self, actions: Vec<Action>) -> GameState {
        self.remove_dead_ants();
        self.move_ants(actions);
        self.attack();
        self.raze_hills();
        self.spawn_ants_from_hive();
        self.harvest_food();
        // Opted for spawning food randomly across the map instead of doing the symmetric spawning that the original Ants game used.
        // The reason is that random food makes the game more challenging as it could lead to scenarios where agents aren't near any food.
        // This will require better learning and handling of complex world states.
        // Which we hope will ultimately lead to more robust agents.
        self.spawn_food_randomly();

        self.turn += 1;
        self.game_state()
    }

    /// Draws the game to the console.
    pub fn draw(&self) {
        self.map.draw(self.turn);
    }

    fn spawn_food_around_hills(&mut self) {
        let ant_hills = self.live_ant_hills();

        // For each ant hill, collect up to 3 random land cells around it
        let lands: Vec<(usize, usize)> = ant_hills
            .iter()
            .flat_map(|(_, row, col)| {
                self.map
                    .land_around(*row, *col)
                    .choose_multiple(&mut self.rng, 3)
                    .cloned()
                    .collect::<Vec<(usize, usize)>>()
            })
            .collect();

        // Spawn food on the random land cells
        self.spawn_food(lands);
    }

    fn spawn_food_randomly(&mut self) {
        let land = self.map.land();
        let food_locations = land
            .choose_multiple(&mut self.rng, self.food_per_turn)
            .cloned()
            .collect();

        self.spawn_food(food_locations);
    }

    fn spawn_food(&mut self, locations: Vec<(usize, usize)>) {
        for (row, col) in locations {
            self.map.set(row, col, Box::new(Food));
        }
    }

    fn spawn_ants_all_hills(&mut self) {
        let ant_hills = self.live_ant_hills();
        self.spawn_ants(ant_hills);
    }

    fn spawn_ants_from_hive(&mut self) {
        let players = self.map.players();
        let hills_by_player = self
            .live_ant_hills()
            .into_iter()
            // Group hills by player
            .fold(vec![vec![]; players], |mut acc, hill| {
                acc[hill.0].push(hill);
                acc
            });

        for (player, hills) in hills_by_player.iter().enumerate().take(players) {
            let available_food = self.hive[player];

            if available_food == 0 {
                continue;
            }

            // Randomly choose hills, up to the available food, to spawn ants on
            // We do this withouth repetition to avoid spawning multiple ants on the same hill
            let ant_hills = hills.choose_multiple(&mut self.rng, available_food);

            // Update the hive with the remaining food
            self.hive[player] -= ant_hills.len();

            // Spawn ants on the chosen hills
            self.spawn_ants(ant_hills.cloned().collect());
        }
    }

    fn spawn_ants(&mut self, ant_hills: Vec<(usize, usize, usize)>) {
        for (player, row, col) in ant_hills {
            self.map.set(
                row,
                col,
                Box::new(Ant::from_ant_hill(
                    player,
                    Box::new(Hill::new(player, true)),
                )),
            );
        }
    }

    fn remove_dead_ants(&mut self) {
        let dead_ants = self
            .map
            .ants()
            .into_iter()
            .filter(|(ant, _, _)| !ant.alive().unwrap())
            .map(|(_, row, col)| (row, col))
            .collect::<Vec<(usize, usize)>>();

        for (row, col) in dead_ants {
            self.map.remove(row, col);
        }
    }

    fn move_ants(&mut self, actions: Vec<Action>) {
        for action in actions {
            let (to_row, to_col) = match action.direction {
                Direction::North => (action.row.saturating_sub(1), action.col),
                Direction::East => (action.row, action.col + 1),
                Direction::South => (action.row + 1, action.col),
                Direction::West => (action.row, action.col.saturating_sub(1)),
            };

            self.map
                .move_entity((action.row, action.col), (to_row, to_col));
        }
    }

    fn attack(&mut self) {
        // Pre-calculate the number of enemies for each live ant as a map of ant `id` to the Vec of enemies
        let ants = self.live_ants();
        let enemies: HashMap<String, Vec<(&dyn Entity, usize, usize)>> = ants
            .iter()
            .map(|(ant, row, col)| {
                let fov = self.map.field_of_vision((*row, *col), self.attack_radius2);
                let enemies = self.enemies(fov, ant.player().unwrap());
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
            self.map.get_mut(row, col).unwrap().set_alive(false);
        }
    }

    fn raze_hills(&mut self) {
        let ants = self.live_ants();
        let hills_to_raze: Vec<(usize, usize, usize)> = ants
            .into_iter()
            .filter_map(|(ant, row, col)| {
                // If the ant is on an ant hill that is not its own, the hill should be razed
                if ant.on_ant_hill().is_some()
                    && ant.player().unwrap()
                        != ant.on_ant_hill().as_ref().unwrap().player().unwrap()
                {
                    let hill_owner = ant.on_ant_hill().as_ref().unwrap().player().unwrap();
                    Some((hill_owner, row, col))
                } else {
                    None
                }
            })
            .collect();

        for (player, row, col) in hills_to_raze {
            self.map
                .get_mut(row, col)
                .unwrap()
                .set_on_ant_hill(Box::new(Hill::new(player, false)));
        }
    }

    fn harvest_food(&mut self) {
        let food = self.map.food();

        for (row, col) in food {
            let unique_player_ants_around_food = self
                .map
                .field_of_vision((row, col), self.food_radius2)
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
                self.hive[*player] += 1;
            }

            self.map.remove(row, col);
        }
    }

    fn live_ant_hills(&self) -> Vec<(usize, usize, usize)> {
        self.map
            .ant_hills()
            .into_iter()
            .filter(|(hill, _, _)| hill.alive().unwrap())
            .map(|(hill, row, col)| (hill.player().unwrap(), row, col))
            .collect()
    }

    fn live_ants(&self) -> Vec<(&dyn Entity, usize, usize)> {
        self.map
            .ants()
            .into_iter()
            .filter(|(ant, _, _)| ant.alive().unwrap())
            .collect()
    }

    fn enemies<'a>(
        &'a self,
        field_of_vision: Vec<(&'a dyn Entity, usize, usize)>,
        player: usize,
    ) -> Vec<(&'a dyn Entity, usize, usize)> {
        field_of_vision
            .into_iter()
            .filter(|(entity, _, _)| {
                entity.name() == "Ant"
                    && entity.player().is_some()
                    && entity.player().unwrap() != player
            })
            .collect()
    }

    fn game_state(&self) -> GameState {
        let players = self.map.players();
        let ants = self
            .live_ants()
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
                    .map(|(entity, row, col)| self.to_state_entity(entity, row, col))
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

    fn to_state_entity(&self, entity: &dyn Entity, row: usize, col: usize) -> StateEntity {
        StateEntity {
            name: entity.name().to_string(),
            row,
            col,
            player: entity.player(),
            alive: entity.alive(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Food;

    #[test]
    fn when_starting_a_game_the_map_is_reset() {
        let map = "\
            rows 4
            cols 4
            players 2
            m %1.%
            m %..%
            m %..%
            m %.0%";
        let mut game = Game::new(map, 4, 4, 1, 5, 0);

        game.map.set(0, 0, Box::new(Food));
        game.start();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Water");
    }

    #[test]
    fn when_starting_a_game_ants_are_spawned_on_ant_hills() {
        let map = "\
            rows 4
            cols 4
            players 2
            m %1.%
            m %..%
            m %..%
            m %.0%";
        let mut game = Game::new(map, 4, 4, 1, 5, 0);

        game.start();

        let ant = game.map.get(0, 1).unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player().unwrap(), 1);
        assert!(ant.alive().unwrap());
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player().unwrap(), 1);

        let ant = game.map.get(3, 2).unwrap();
        assert_eq!(ant.name(), "Ant");
        assert_eq!(ant.player().unwrap(), 0);
        assert!(ant.alive().unwrap());
        assert_eq!(ant.on_ant_hill().as_ref().unwrap().player().unwrap(), 0);
    }

    #[test]
    fn when_starting_a_game_food_is_spawned_around_land_locations_for_each_ant_hill() {
        let map = "\
            rows 4
            cols 4
            players 2
            m %1.%
            m %..%
            m %..%
            m %.0%";
        let mut game = Game::new(map, 4, 4, 1, 5, 0);

        game.start();

        // The map has 1 ant hill at (0, 1) for player 1 with 3 land cells around it
        // So food should be spawned at (0, 2), (1, 1), and (1, 2)
        assert_eq!(game.map.get(0, 2).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(1, 1).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(1, 2).as_ref().unwrap().name(), "Food");

        // The map has 1 ant hill at (3, 2) for player 0 with 3 land cells around it
        // So food should be spawned at (2, 1), (2, 2), and (3, 1)
        assert_eq!(game.map.get(2, 1).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(2, 2).as_ref().unwrap().name(), "Food");
        assert_eq!(game.map.get(3, 1).as_ref().unwrap().name(), "Food");
    }

    #[test]
    fn when_starting_a_game_the_correct_game_state_is_returned() {
        let map = "\
            rows 4
            cols 4
            players 2
            m %1.%
            m %..%
            m %..%
            m %.0%";
        let mut game = Game::new(map, 4, 4, 1, 5, 0);

        let state = game.start();

        assert_eq!(state.turn, 0);

        // The map has 2 players
        assert_eq!(state.scores, vec![0, 0]);
        assert_eq!(state.ants.len(), 2);

        // The map has 1 ant hill at (3, 2) for player 0
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

        // The map has 1 ant hill at (0, 1) for player 1
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
        let mut game = Game::new(map, 4, 4, 1, 5, 0);

        assert!(game.map.get(1, 0).unwrap().alive().unwrap());
        game.map.get_mut(1, 0).unwrap().set_alive(false);

        game.remove_dead_ants();

        assert!(game.map.get(1, 0).is_none());
    }

    #[test]
    fn when_removing_dead_ants_if_there_are_no_dead_ants_no_ants_are_removed() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m a.";
        let mut game = Game::new(map, 4, 4, 1, 5, 0);

        assert!(game.map.get(1, 0).unwrap().alive().unwrap());

        game.remove_dead_ants();

        assert!(game.map.get(1, 0).is_some());
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(game.map.get(1, 3).unwrap().alive().unwrap());

        game.attack();

        assert!(!game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(!game.map.get(1, 3).unwrap().alive().unwrap());
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(game.map.get(2, 3).unwrap().alive().unwrap());

        game.attack();

        assert!(game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(!game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(game.map.get(2, 3).unwrap().alive().unwrap());
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(game.map.get(2, 3).unwrap().alive().unwrap());

        game.attack();

        assert!(!game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(!game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(!game.map.get(2, 3).unwrap().alive().unwrap());
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(1, 0).unwrap().alive().unwrap());
        assert!(game.map.get(1, 2).unwrap().alive().unwrap());
        assert!(game.map.get(1, 4).unwrap().alive().unwrap());

        game.attack();

        assert!(game.map.get(1, 0).unwrap().alive().unwrap());
        assert!(!game.map.get(1, 2).unwrap().alive().unwrap());
        assert!(game.map.get(1, 4).unwrap().alive().unwrap());
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(game.map.get(1, 3).unwrap().alive().unwrap());
        assert!(game.map.get(2, 3).unwrap().alive().unwrap());

        game.attack();

        assert!(!game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(game.map.get(1, 1).unwrap().alive().unwrap());
        assert!(game.map.get(1, 3).unwrap().alive().unwrap());
        assert!(!game.map.get(2, 3).unwrap().alive().unwrap());
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
        assert!(game.map.get(0, 1).unwrap().alive().unwrap());
        assert!(game.map.get(0, 2).unwrap().alive().unwrap());
        assert!(game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(game.map.get(0, 4).unwrap().alive().unwrap());
        assert!(game.map.get(0, 5).unwrap().alive().unwrap());
        assert!(game.map.get(0, 6).unwrap().alive().unwrap());
        assert!(game.map.get(0, 7).unwrap().alive().unwrap());
        assert!(game.map.get(0, 8).unwrap().alive().unwrap());
        assert!(game.map.get(1, 3).unwrap().alive().unwrap());
        assert!(game.map.get(1, 4).unwrap().alive().unwrap());
        assert!(game.map.get(1, 5).unwrap().alive().unwrap());
        assert!(game.map.get(2, 3).unwrap().alive().unwrap());
        assert!(game.map.get(2, 4).unwrap().alive().unwrap());
        assert!(game.map.get(2, 5).unwrap().alive().unwrap());

        game.attack();

        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
        assert!(game.map.get(0, 1).unwrap().alive().unwrap());
        assert!(!game.map.get(0, 2).unwrap().alive().unwrap());
        assert!(!game.map.get(0, 3).unwrap().alive().unwrap());
        assert!(!game.map.get(0, 4).unwrap().alive().unwrap());
        assert!(!game.map.get(0, 5).unwrap().alive().unwrap());
        assert!(!game.map.get(0, 6).unwrap().alive().unwrap());
        assert!(game.map.get(0, 7).unwrap().alive().unwrap());
        assert!(game.map.get(0, 8).unwrap().alive().unwrap());
        assert!(!game.map.get(1, 3).unwrap().alive().unwrap());
        assert!(!game.map.get(1, 4).unwrap().alive().unwrap());
        assert!(!game.map.get(1, 5).unwrap().alive().unwrap());
        assert!(!game.map.get(2, 3).unwrap().alive().unwrap());
        assert!(game.map.get(2, 4).unwrap().alive().unwrap());
        assert!(!game.map.get(2, 5).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_does_not_have_an_ant_the_hill_is_not_razed() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m ..";
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
        game.raze_hills();
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_has_an_ant_of_the_same_player_the_hill_is_not_razed() {
        let map = "\
            rows 2
            cols 2
            players 2
            m A.
            m ..";
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
        game.raze_hills();
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_has_a_dead_enemy_ant_the_hill_is_not_razed() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m b.";
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        // Move the enemy to the hill
        game.map.move_entity((1, 0), (0, 0));
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");

        // Kill the enemy
        game.map.get_mut(0, 0).unwrap().set_alive(false);

        game.raze_hills();

        assert!(game
            .map
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        // Move the enemy to the hill
        game.map.move_entity((1, 0), (0, 0));
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");

        game.raze_hills();

        assert!(!game
            .map
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        game.spawn_ants_from_hive();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert_eq!(game.map.get(0, 1).unwrap().name(), "Hill");
        assert_eq!(game.hive, vec![0, 0]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_the_hill_is_razed_no_ants_are_spawned() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m ..";
        let mut game = Game::new(map, 4, 5, 1, 5, 0);
        game.hive = vec![1];

        // Raze the hill
        game.map.get_mut(0, 0).unwrap().set_alive(false);

        game.spawn_ants_from_hive();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert_eq!(game.hive, vec![1]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_there_is_food_and_one_hill_one_ant_is_spawned() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 01
            m ..";
        let mut game = Game::new(map, 4, 5, 1, 5, 0);
        game.hive = vec![1, 1];

        game.spawn_ants_from_hive();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");
        assert_eq!(game.map.get(0, 0).unwrap().player().unwrap(), 0);
        assert_eq!(game.map.get(0, 1).unwrap().name(), "Ant");
        assert_eq!(game.map.get(0, 1).unwrap().player().unwrap(), 1);
        assert_eq!(game.hive, vec![0, 0]);
    }

    #[test]
    fn when_spawning_ants_from_hive_if_there_is_more_food_and_one_hill_only_one_ant_is_spawned() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m ..";
        let mut game = Game::new(map, 4, 5, 1, 5, 0);
        game.hive = vec![5];

        game.spawn_ants_from_hive();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");
        assert_eq!(game.hive, vec![4]);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);
        game.hive = vec![1];

        game.spawn_ants_from_hive();

        // Hill is chosen at random and we make it predictable based on the seed
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert_eq!(game.map.get(1, 1).unwrap().name(), "Ant");
        assert_eq!(game.hive, vec![0]);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);
        game.hive = vec![5, 2];

        game.spawn_ants_from_hive();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");
        assert_eq!(game.map.get(0, 0).unwrap().player().unwrap(), 0);
        assert_eq!(game.map.get(1, 1).unwrap().name(), "Ant");
        assert_eq!(game.map.get(1, 1).unwrap().player().unwrap(), 0);

        assert_eq!(game.map.get(0, 1).unwrap().name(), "Ant");
        assert_eq!(game.map.get(0, 1).unwrap().player().unwrap(), 1);
        assert_eq!(game.map.get(1, 0).unwrap().name(), "Ant");
        assert_eq!(game.map.get(1, 0).unwrap().player().unwrap(), 1);

        assert_eq!(game.hive, vec![3, 0]);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        game.harvest_food();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Food");
        assert_eq!(game.map.get(1, 1).unwrap().name(), "Food");
        assert_eq!(game.map.get(2, 2).unwrap().name(), "Food");
        assert_eq!(game.hive, vec![0]);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        game.harvest_food();

        assert!(game.map.get(0, 0).is_none());
        assert!(game.map.get(2, 2).is_none());
        assert_eq!(game.hive, vec![2, 0]);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 0);

        game.harvest_food();

        assert!(game.map.get(0, 0).is_none());
        assert!(game.map.get(2, 2).is_none());
        assert_eq!(game.hive, vec![0, 0]);
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
        let mut game = Game::new(map, 4, 5, 1, 8, 0);

        game.spawn_food_randomly();

        let food = game.map.food();
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
        let mut game = Game::new(map, 4, 5, 1, 9, 0);

        game.spawn_food_randomly();

        let food = game.map.food();
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
        let mut game = Game::new(map, 4, 5, 1, 9, 0);

        game.spawn_food_randomly();
        assert!(game.map.food().is_empty());
    }
}
