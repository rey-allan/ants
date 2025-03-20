use crate::entities::{Ant, Entity, Food, Hill};
use crate::map::Map;
use crate::replay::{create_replay_logger, ReplayLogger};
use pyo3::prelude::*;
use rand::distributions::{Distribution, Standard};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use std::collections::{HashMap, HashSet};

/// The Ants game.
/// Main entry point for running the game.
#[pyclass(module = "ants_engine")]
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
    started: bool,
    finished: bool,
    finished_reason: Option<FinishedReason>,
    cutoff_threshold: usize,
    turns_with_too_much_food: usize,
    points_for_razing_hill: usize,
    points_for_losing_hill: usize,
    max_turns: usize,
    replay_logger: Box<dyn ReplayLogger>,
    rng: StdRng,
}

/// Represents the state of the game.
#[pyclass(module = "ants_engine", get_all)]
pub struct GameState {
    /// The current turn.
    pub turn: usize,
    /// The scores for each player where the index is the player number.
    pub scores: Vec<usize>,
    /// The ants for each player where the index is the player number.
    pub ants: Vec<Vec<PlayerAnt>>,
    /// Whether the game has finished.
    pub finished: bool,
    /// The reason the game finished. `None` if the game has not finished.
    pub finished_reason: Option<FinishedReason>,
}

/// Represents the direction an ant can move.
#[derive(Clone, PartialEq)]
#[pyclass(module = "ants_engine", eq, eq_int)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Distribution<Direction> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..4) {
            0 => Direction::North,
            1 => Direction::East,
            2 => Direction::South,
            _ => Direction::West,
        }
    }
}

/// Represents the reason the game finished.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[pyclass(module = "ants_engine", eq, eq_int)]
pub enum FinishedReason {
    /// The game ended because there was only one player left.
    LoneSurvivor,
    /// The game ended because the rank stabilized, i.e. no player can surpass the current leader anymore.
    RankStabilized,
    /// The game ended because food was not being consumed and it reached 90% or more of the map.
    TooMuchFood,
    /// The game ended because the maximum number of turns was reached.
    TurnLimitReached,
}

/// Represents an action an ant can take.
///
/// The action is a tuple of the ant's row, column, and direction.
/// If the direction is not a valid move, the ant will stay in place.
/// Or if the provided location is not a valid ant, the action will be ignored.
#[derive(Clone)]
#[pyclass(module = "ants_engine")]
pub struct Action {
    row: usize,
    col: usize,
    direction: Direction,
}

#[pymethods]
impl Action {
    /// Creates a new action.
    ///
    /// # Arguments
    /// * `row` - The row of the ant to move.
    /// * `col` - The column of the ant to move.
    /// * `direction` - The direction the ant should move.
    #[new]
    pub fn new(row: usize, col: usize, direction: Direction) -> Action {
        Action {
            row,
            col,
            direction,
        }
    }
}

/// Represents an entity in the game state.
#[derive(Clone)]
#[pyclass(name = "Entity", module = "ants_engine", get_all)]
pub struct StateEntity {
    /// The name of the entity.
    pub name: String,
    /// The row of the location of the entity.
    pub row: usize,
    /// The column of the location of the entity.
    pub col: usize,
    /// The player who owns the entity, if applicable. For example, food does not belong to a player.
    pub player: Option<usize>,
    /// Whether the entity is alive, if applicable. For example, food does not have an alive state.
    pub alive: Option<bool>,
}

/// Represents an ant in the game state.
#[derive(Clone)]
#[pyclass(name = "Ant", module = "ants_engine", get_all)]
pub struct PlayerAnt {
    /// The unique identifier for the ant.
    pub id: String,
    /// The row of the location of the ant.
    pub row: usize,
    /// The column of the location of the ant.
    pub col: usize,
    /// The player who owns the ant.
    pub player: usize,
    /// Whether the ant is alive.
    pub alive: bool,
    /// The field of vision for the ant as a list of entities the ant can see.
    pub field_of_vision: Vec<StateEntity>,
}

#[pymethods]
impl Game {
    /// Creates a new game.
    ///
    /// # Arguments
    /// * `map_contents` - The map as a string.
    /// * `fov_radius2` - The radius **squared** of the field of vision for each ant.
    /// * `attack_radius2` - The radius **squared** of the attack range for each ant.
    /// * `food_radius2` - The radius **squared** of the range around ants to harvest food.
    /// * `food_rate` - The amount of food to spawn *per player* on each round.
    /// * `max_turns` - The maximum number of turns before the game ends.
    /// * `seed` - The seed for the random number generator.
    /// * `replay_filename` - The filename to save the replay of the game to. If `None`, no replay will be saved.
    #[new]
    #[pyo3(signature = (map_contents, fov_radius2, attack_radius2, food_radius2, food_rate, max_turns, seed, replay_filename=None))]
    pub fn new(
        map_contents: &str,
        fov_radius2: usize,
        attack_radius2: usize,
        food_radius2: usize,
        food_rate: usize,
        max_turns: usize,
        seed: u64,
        replay_filename: Option<String>,
    ) -> Game {
        let map = Map::parse(map_contents);
        let players = map.players();
        let width = map.width();
        let height = map.height();

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
            started: false,
            finished: false,
            finished_reason: None,
            cutoff_threshold: 150,
            turns_with_too_much_food: 0,
            points_for_razing_hill: 2,
            points_for_losing_hill: 1,
            max_turns,
            replay_logger: create_replay_logger(
                replay_filename,
                players,
                width,
                height,
                map_contents.to_string(),
            ),
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Starts the game.
    ///
    /// Must be called once before updating the game state.
    pub fn start(&mut self) -> GameState {
        self.turn = 0;
        self.started = true;
        self.finished = false;
        self.finished_reason = None;
        self.turns_with_too_much_food = 0;
        self.hive = vec![0; self.map.players()];
        self.map = Map::parse(&self.map_contents);
        self.replay_logger.clear();

        self.compute_initial_scores();
        self.spawn_food_around_hills();
        self.spawn_ants_all_hills();

        self.replay_logger.log_turn(
            self.turn,
            self.live_ants_per_player_count(),
            self.hive.clone(),
            self.scores.clone(),
        );

        // Compute the intial game state
        self.game_state()
    }

    /// Updates the game state.
    ///
    /// # Arguments
    /// * `actions` - The actions to take for each ant.
    pub fn update(&mut self, actions: Vec<Action>) -> GameState {
        if !self.started {
            panic!("Game has not started! Call `start` to start the game.");
        }

        if self.finished {
            panic!("Game is finished! Call `start` to start a new game.");
        }

        self.turn += 1;

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

        self.check_for_endgame();

        // Compute the game state before removing dead ants so that the dead ants are included in the state
        let state = self.game_state();
        self.remove_dead_ants();

        self.replay_logger.log_turn(
            self.turn,
            self.live_ants_per_player_count(),
            self.hive.clone(),
            self.scores.clone(),
        );

        // If the game finished, log the end game and save the replay
        if self.finished {
            self.replay_logger
                .log_end_game(format!("{:?}", self.finished_reason.as_ref().unwrap()));
            self.replay_logger.save();
        }

        state
    }

    /// Draws the game to the console.
    pub fn draw(&self) {
        let ants = self.live_ants_per_player_count();
        self.map.draw(self.turn, &self.scores, &ants, &self.hive);
    }
}

impl Game {
    fn compute_initial_scores(&mut self) {
        // Each agent starts with 1 point per hill
        let ants_hills_per_player = self.live_ant_hills_per_player();

        for (player, hills) in ants_hills_per_player.iter().enumerate() {
            self.scores[player] = hills.len();
        }
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
        // Make sure to only spawn food if there is less food than the food per turn
        let current_food = self.map.food().len();

        if current_food >= self.food_per_turn {
            return;
        }

        let food_to_spawn = self.food_per_turn - current_food;
        let land = self.map.land();
        let food_locations = land
            .choose_multiple(&mut self.rng, food_to_spawn)
            .cloned()
            .collect();

        self.spawn_food(food_locations);
    }

    fn spawn_food(&mut self, locations: Vec<(usize, usize)>) {
        for (row, col) in locations {
            self.map.set(row, col, Box::new(Food));
            self.replay_logger.log_spawn_food(self.turn, (row, col));
        }
    }

    fn spawn_ants_all_hills(&mut self) {
        let ant_hills = self.live_ant_hills();
        self.spawn_ants(ant_hills);
    }

    fn spawn_ants_from_hive(&mut self) {
        let players = self.map.players();
        let hills_by_player = self.live_ant_hills_per_player();

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
            let ant = Ant::from_ant_hill(player, Box::new(Hill::new(player, true)));
            let id = ant.id().to_string();
            self.map.set(row, col, Box::new(ant));
            self.replay_logger
                .log_spawn_ant(self.turn, id, player, (row, col));
        }
    }

    fn remove_dead_ants(&mut self) {
        let dead_ants = self
            .map
            .ants()
            .into_iter()
            .filter(|(ant, _, _)| !ant.alive().unwrap())
            .map(|(ant, row, col)| (ant.id().to_string(), row, col))
            .collect::<Vec<(String, usize, usize)>>();

        for (id, row, col) in dead_ants {
            // If the ant was on a hill, replace the location with the hill, otherwise remove the ant
            if let Some(hill) = self.map.get(row, col).unwrap().on_ant_hill() {
                self.map.set(
                    row,
                    col,
                    Box::new(Hill::new(hill.player().unwrap(), hill.alive().unwrap())),
                );
            } else {
                self.map.remove(row, col);
            }

            self.replay_logger.log_remove_ant(self.turn, id);
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

            let id = self
                .map
                .get(action.row, action.col)
                .unwrap()
                .id()
                .to_string();

            let did_move = self
                .map
                .move_entity((action.row, action.col), (to_row, to_col));

            if did_move {
                self.replay_logger.log_move_ant(
                    self.turn,
                    id,
                    (action.row, action.col),
                    (to_row, to_col),
                );
            }
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
        let mut attack_logs = Vec::new();

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

                // Collect attack log from each enemy to the ant
                for (_, enemy_row, enemy_col) in ant_enemies {
                    attack_logs.push(((*enemy_row, *enemy_col), (row, col)));
                }
            }
        }

        // After all battles are resolved, kill the ants
        for (row, col) in to_kill {
            self.map.get_mut(row, col).unwrap().set_alive(false);
        }

        // Log all attack events
        for (enemy_pos, ant_pos) in attack_logs {
            self.replay_logger.log_attack(self.turn, enemy_pos, ant_pos);
        }
    }

    fn raze_hills(&mut self) {
        let ants = self.live_ants();
        let hills_to_raze: Vec<(usize, usize, usize, usize)> = ants
            .into_iter()
            .filter_map(|(ant, row, col)| {
                // If the ant is on an ant hill that is not its own, the hill should be razed
                if ant.on_ant_hill().is_some()
                    && ant.player().unwrap()
                        != ant.on_ant_hill().as_ref().unwrap().player().unwrap()
                {
                    let hill_owner = ant.on_ant_hill().as_ref().unwrap().player().unwrap();
                    let player = ant.player().unwrap();
                    Some((hill_owner, player, row, col))
                } else {
                    None
                }
            })
            .collect();

        for (hill_owner, player, row, col) in hills_to_raze {
            // Add the points for razin the hill to the player's score
            self.scores[player] += self.points_for_razing_hill;
            // Subtract the points for losing the hill from the hill owner's score
            self.scores[hill_owner] -= self.points_for_losing_hill;
            // Update the hill to be razed
            self.map
                .get_mut(row, col)
                .unwrap()
                .set_on_ant_hill(Box::new(Hill::new(hill_owner, false)));
            self.replay_logger.log_remove_hill(self.turn, (row, col));
        }
    }

    fn harvest_food(&mut self) {
        let food = self.map.food();
        let mut ants_that_harvested_food: HashSet<(usize, usize)> = HashSet::new();

        for (row, col) in food {
            let ants_around_food: Vec<(usize, usize, usize)> = self
                .map
                .field_of_vision((row, col), self.food_radius2)
                .into_iter()
                .filter(|(entity, _, _)| entity.name() == "Ant")
                .map(|(entity, row, col)| (row, col, entity.player().unwrap()))
                .collect();

            if ants_around_food.is_empty() {
                continue;
            }

            // Check to see if there is only one player around the food
            let unique_player_ants_around_food: HashSet<usize> = ants_around_food
                .iter()
                .map(|(_, _, player)| *player)
                .collect();

            // If there is only one player around the food, they consume it into their hive
            // Otherwise, it's simply removed from the map without being consumed by anyone
            if unique_player_ants_around_food.len() == 1 {
                let mut can_harvest = false;

                // But first, check if the ants around the food already harvested this turn
                for (row, col, player) in &ants_around_food {
                    if ants_that_harvested_food.contains(&(*row, *col)) {
                        continue;
                    }

                    // This ant can harvest the food
                    self.hive[*player] += 1;
                    ants_that_harvested_food.insert((*row, *col));
                    can_harvest = true;
                    break;
                }

                // No ants around the food could harvest it but since they all belong to
                // the same player, we don't remove the food
                if !can_harvest {
                    continue;
                }
            }

            self.map.remove(row, col);
            self.replay_logger.log_remove_food(self.turn, (row, col));
        }
    }

    fn live_ant_hills_per_player(&self) -> Vec<Vec<(usize, usize, usize)>> {
        let players = self.map.players();
        self.live_ant_hills()
            .into_iter()
            // Group hills by player
            .fold(vec![vec![]; players], |mut acc, hill| {
                acc[hill.0].push(hill);
                acc
            })
    }

    fn live_ant_hills(&self) -> Vec<(usize, usize, usize)> {
        self.map
            .ant_hills()
            .into_iter()
            .filter(|(hill, _, _)| hill.alive().unwrap())
            .map(|(hill, row, col)| (hill.player().unwrap(), row, col))
            .collect()
    }

    fn live_ants_per_player_count(&self) -> Vec<usize> {
        let players = self.map.players();
        self.live_ants()
            .into_iter()
            .fold(vec![vec![]; players], |mut acc, (ant, _, _)| {
                acc[ant.player().unwrap()].push(ant);
                acc
            })
            .iter()
            .map(|ants| ants.len())
            .collect::<Vec<usize>>()
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
            finished: self.finished,
            finished_reason: self.finished_reason.clone(),
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

    fn check_for_endgame(&mut self) {
        self.check_for_food_not_being_gathered();

        if self.turns_with_too_much_food >= self.cutoff_threshold {
            self.finished = true;
            self.finished_reason = Some(FinishedReason::TooMuchFood);

            return;
        }

        if self.remaining_players() == 1 {
            self.finished = true;
            self.finished_reason = Some(FinishedReason::LoneSurvivor);

            return;
        }

        if self.rank_stabilized() {
            self.finished = true;
            self.finished_reason = Some(FinishedReason::RankStabilized);

            return;
        }

        if self.turn >= self.max_turns {
            self.finished = true;
            self.finished_reason = Some(FinishedReason::TurnLimitReached);
        }
    }

    fn check_for_food_not_being_gathered(&mut self) {
        let total_food = self.map.food().len();
        let total_ants = self.map.ants().len();
        let food_pct = total_food as f64 / (total_food + total_ants) as f64;

        // If the food is 85% or more of the count of ants and food then the food is not being gathered properly
        if food_pct >= 0.85 {
            self.turns_with_too_much_food += 1;
        } else {
            // Reset the count if the food is being gathered properly
            self.turns_with_too_much_food = 0;
        }
    }

    fn remaining_players(&self) -> usize {
        self.live_ants()
            .into_iter()
            .map(|(ant, _, _)| ant.player().unwrap())
            .collect::<HashSet<usize>>()
            .len()
    }

    fn rank_stabilized(&self) -> bool {
        let live_ant_hills_per_player = self.live_ant_hills_per_player();
        let current_scores = &self.scores;

        // If all players are tied, the rank isn't stabilized yet
        if current_scores
            .iter()
            .all(|score| *score == current_scores[0])
        {
            return false;
        }

        // Get the player that is in the lead
        let (leader, leader_score) = current_scores
            .iter()
            .enumerate()
            .max_by_key(|(_, score)| *score)
            .unwrap();

        // For each other player, compute their score as if they were to raze all other hills
        for player in 0..self.map.players() {
            if player == leader {
                continue;
            }

            let mut scores = current_scores.clone();
            for (other_player, hills) in live_ant_hills_per_player.iter().enumerate() {
                if other_player == player {
                    continue;
                }

                // Add to the score as if the player razed all hills from the other player
                scores[player] += hills.len() * self.points_for_razing_hill;
                // Subtract from the score as if the other player lost all their hills
                scores[other_player] -= hills.len() * self.points_for_losing_hill;
            }

            // If this player can surpass the leader, the rank isn't stabilized yet
            if scores[player] > *leader_score {
                return false;
            }
        }

        // If no player can surpass the leader, the rank is stabilized
        true
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

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
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

        let state = game.start();

        assert_eq!(state.turn, 0);
        assert!(!state.finished);
        assert!(state.finished_reason.is_none());

        // The map has 2 players
        assert_eq!(state.scores, vec![1, 1]);
        assert_eq!(state.ants.len(), 2);

        // The map has 1 ant hill at (3, 2) for player 0
        assert_eq!(state.ants[0].len(), 1);
        assert_eq!(state.ants[0][0].row, 3);
        assert_eq!(state.ants[0][0].col, 2);
        assert_eq!(state.ants[0][0].player, 0);
        assert!(state.ants[0][0].alive);
        // Given the fov radius of 2, the ant at (3, 2) should see 8 entities
        assert_eq!(state.ants[0][0].field_of_vision.len(), 8);
        // Let's check that it was able to see the water next to it at (3, 3)
        assert!(state.ants[0][0]
            .field_of_vision
            .iter()
            .any(|entity| entity.name == "Water" && entity.row == 3 && entity.col == 3));
        // Let's also check that it was able to see the ant hill where it is standing at (3, 2)
        assert!(state.ants[0][0]
            .field_of_vision
            .iter()
            .any(|entity| entity.name == "Hill"
                && entity.row == 3
                && entity.col == 2
                && entity.player.unwrap() == 0
                && entity.alive.unwrap()));

        // The map has 1 ant hill at (0, 1) for player 1
        assert_eq!(state.ants[1].len(), 1);
        assert_eq!(state.ants[1][0].row, 0);
        assert_eq!(state.ants[1][0].col, 1);
        assert_eq!(state.ants[1][0].player, 1);
        assert!(state.ants[1][0].alive);
        // Given the fov radius of 2, the ant at (0, 1) should see 8 entities
        assert_eq!(state.ants[1][0].field_of_vision.len(), 8);
        // Let's check that it was able to see the water next to it at (0, 0)
        assert!(state.ants[1][0]
            .field_of_vision
            .iter()
            .any(|entity| entity.name == "Water" && entity.row == 0 && entity.col == 0));
        // Let's also check that it was able to see the ant hill where it is standing at (0, 1)
        assert!(state.ants[1][0]
            .field_of_vision
            .iter()
            .any(|entity| entity.name == "Hill"
                && entity.row == 0
                && entity.col == 1
                && entity.player.unwrap() == 1
                && entity.alive.unwrap()));
    }

    #[test]
    fn when_starting_a_game_the_initial_scores_are_computed_as_the_number_of_ant_hills_per_player()
    {
        let map = "\
            rows 4
            cols 4
            players 2
            m %1.%
            m %1.%
            m %..%
            m %00%";
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

        game.start();

        assert_eq!(game.scores, vec![2, 2]);
    }

    #[test]
    #[should_panic(expected = "Game has not started! Call `start` to start the game.")]
    fn when_updating_a_game_that_has_not_started_a_panic_occurs() {
        let map = "\
            rows 4
            cols 4
            players 2
            m %1.%
            m %..%
            m %..%
            m %.0%";
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);
        game.update(vec![]);
    }

    #[test]
    #[should_panic(expected = "Game is finished! Call `start` to start a new game.")]
    fn when_updating_a_game_that_has_finished_a_panic_occurs() {
        let map = "\
            rows 4
            cols 4
            players 2
            m %1.%
            m %..%
            m %..%
            m %.0%";
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);
        game.started = true;
        game.finished = true;

        game.update(vec![]);
    }

    #[test]
    fn when_removing_dead_ants_all_dead_ants_are_removed() {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m a.";
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

        assert!(game.map.get(1, 0).unwrap().alive().unwrap());

        game.remove_dead_ants();

        assert!(game.map.get(1, 0).is_some());
    }

    #[test]
    fn when_removing_dead_ants_if_a_dead_ant_is_on_a_hill_the_hill_is_restored() {
        let map = "\
            rows 2
            cols 2
            players 1
            m A.
            m ..";
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        game.map.get_mut(0, 0).unwrap().set_alive(false);

        game.remove_dead_ants();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert_eq!(game.map.get(0, 0).unwrap().player().unwrap(), 0);
    }

    #[test]
    fn when_removing_dead_ants_if_a_dead_ant_is_on_enemy_hill_the_hill_is_restored() {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m b.";
        let mut game = Game::new(map, 4, 4, 1, 5, 1500, 0, None);

        // Move the ant to the enemy hill
        game.map.move_entity((1, 0), (0, 0));

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");
        assert_eq!(game.map.get(0, 0).unwrap().player().unwrap(), 1);
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        game.map.get_mut(0, 0).unwrap().set_alive(false);

        game.remove_dead_ants();

        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        // The hill should be restored to the original owner
        assert_eq!(game.map.get(0, 0).unwrap().player().unwrap(), 0);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
    fn when_razing_hills_if_a_hill_does_not_have_an_ant_the_hill_is_not_razed_and_scores_are_not_changed(
    ) {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m ..";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        game.compute_initial_scores();

        assert_eq!(game.scores, vec![1]);
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        game.raze_hills();

        assert_eq!(game.scores, vec![1]);
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_has_an_ant_of_the_same_player_the_hill_is_not_razed_and_scores_are_not_changed(
    ) {
        let map = "\
            rows 2
            cols 2
            players 1
            m 0.
            m a.";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        game.compute_initial_scores();

        assert_eq!(game.scores, vec![1]);
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        // Move the ant to the hill
        game.map.move_entity((1, 0), (0, 0));
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");

        game.raze_hills();

        assert_eq!(game.scores, vec![1]);
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());
    }

    #[test]
    fn when_razing_hills_if_a_hill_has_a_dead_enemy_ant_the_hill_is_not_razed_and_scores_are_not_changed(
    ) {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m b1";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        game.compute_initial_scores();

        assert_eq!(game.scores, vec![1, 1]);
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        // Move the enemy to the hill
        game.map.move_entity((1, 0), (0, 0));
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");

        // Kill the enemy
        game.map.get_mut(0, 0).unwrap().set_alive(false);

        game.raze_hills();

        assert_eq!(game.scores, vec![1, 1]);
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
    fn when_razing_hills_if_a_hill_has_an_alive_enemy_ant_the_hill_is_razed_and_scores_are_updated()
    {
        let map = "\
            rows 2
            cols 2
            players 2
            m 0.
            m b1";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        game.compute_initial_scores();

        assert_eq!(game.scores, vec![1, 1]);
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Hill");
        assert!(game.map.get(0, 0).unwrap().alive().unwrap());

        // Move the enemy to the hill
        game.map.move_entity((1, 0), (0, 0));
        assert_eq!(game.map.get(0, 0).unwrap().name(), "Ant");

        game.raze_hills();

        // Player 0 loses 1 point for losing the hill
        // Player 1 gains 2 points for razing the hill
        assert_eq!(game.scores, vec![0, 3]);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

        game.harvest_food();

        assert!(game.map.get(0, 0).is_none());
        assert!(game.map.get(2, 2).is_none());
        assert_eq!(game.hive, vec![0, 0]);
    }

    #[test]
    fn when_harvesting_food_an_ant_can_only_consume_one_food_at_a_time() {
        let map = "\
            rows 3
            cols 3
            players 1
            m .*.
            m *a*
            m .*.";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

        game.harvest_food();

        assert!(game.map.get(0, 1).is_none());
        assert!(game.map.get(1, 0).is_some());
        assert!(game.map.get(1, 2).is_some());
        assert!(game.map.get(2, 1).is_some());
        assert_eq!(game.hive, vec![1]);
    }

    #[test]
    fn when_harvesting_food_two_distinct_ants_from_the_same_player_can_consume_food_at_the_same_time(
    ) {
        let map = "\
            rows 3
            cols 3
            players 1
            m .*a
            m *a*
            m .*.";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

        game.harvest_food();

        assert!(game.map.get(0, 1).is_none());
        assert!(game.map.get(1, 0).is_none());
        assert!(game.map.get(1, 2).is_some());
        assert!(game.map.get(2, 1).is_some());
        assert_eq!(game.hive, vec![2]);
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
        let mut game = Game::new(map, 4, 5, 1, 8, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 9, 1500, 0, None);

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
        let mut game = Game::new(map, 4, 5, 1, 9, 1500, 0, None);

        game.spawn_food_randomly();
        assert!(game.map.food().is_empty());
    }

    #[test]
    fn when_spawning_food_randomly_and_there_is_enough_current_food_no_more_food_is_spawned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m *..
            m .a.
            m ...";
        // If we use a `food_rate` of 1, we will only spawn 1 food per turn
        // and since the map already has 1 food, we should not spawn any more
        let mut game = Game::new(map, 4, 5, 1, 1, 1500, 0, None);

        game.spawn_food_randomly();
        assert_eq!(game.map.food().len(), 1);
    }

    #[test]
    fn when_spawning_food_randomly_and_there_is_some_food_already_only_the_missing_food_is_spawned()
    {
        let map = "\
            rows 3
            cols 3
            players 1
            m *..
            m .a.
            m ...";
        // If we use a `food_rate` of 2, we will spawn 2 food per turn
        // and since the map already has 1 food, we should spawn 1 more
        let mut game = Game::new(map, 4, 5, 1, 2, 1500, 0, None);

        game.spawn_food_randomly();
        assert_eq!(game.map.food().len(), 2);
    }

    #[test]
    fn when_checking_for_endgame_if_the_food_is_not_being_gathered_the_game_ends() {
        let map = "\
            rows 3
            cols 3
            players 1
            m *a*
            m ***
            m .**";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        game.cutoff_threshold = 1;

        game.check_for_endgame();

        assert!(game.finished);
        assert_eq!(game.finished_reason, Some(FinishedReason::TooMuchFood));
    }

    #[test]
    fn when_checking_for_endgame_if_only_one_player_remains_with_ants_the_game_ends() {
        let map = "\
            rows 3
            cols 3
            players 2
            m a..
            m aa.
            m ...";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);

        game.check_for_endgame();

        assert!(game.finished);
        assert_eq!(game.finished_reason, Some(FinishedReason::LoneSurvivor));
    }

    #[test]
    fn when_checking_for_endgame_if_all_players_are_tied_rank_is_not_stabilized_and_the_game_does_not_end(
    ) {
        let map = "\
            rows 3
            cols 3
            players 2
            m 0..
            m ...
            m ..1";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        game.compute_initial_scores();

        game.check_for_endgame();

        assert!(!game.finished);
        assert!(game.finished_reason.is_none());
        // Sanity check to make sure the original scores are not changed
        assert_eq!(game.scores, vec![1, 1]);
    }

    #[test]
    fn when_checking_for_endgame_if_the_current_leader_cannot_be_surpassed_the_rank_is_stabilized_and_the_game_ends(
    ) {
        let map = "\
            rows 3
            cols 3
            players 4
            m 0..
            m ...
            m .3.";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        // If player 0 razes the hills of player 1 and 2, the scores are 0=5, 1=0, 2=0, 3=1
        // In this case, even if player 3 were to raze the hill of player 0, the score would be 0=4, 1=0, 2=0, 3=3
        // so player 3 can't possibly do better than 2nd place and the game ends
        game.scores = vec![5, 0, 0, 1];

        game.check_for_endgame();

        assert!(game.finished);
        assert_eq!(game.finished_reason, Some(FinishedReason::RankStabilized));
        // Sanity check to make sure the original scores are not changed
        assert_eq!(game.scores, vec![5, 0, 0, 1]);
    }

    #[test]
    fn when_checking_for_endgame_if_the_current_leader_can_be_surpassed_the_rank_is_not_stabilized_and_the_game_does_not_end(
    ) {
        let map = "\
            rows 3
            cols 3
            players 4
            m 0..
            m .2.
            m .3.";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        // If player 0 razes the hills of player 1, the scores are 0=3, 1=0, 2=1, 3=1
        // In this case, if player 2 were to raze all the other hills, the score would be 0=2, 1=0, 2=3, 3=0
        // and player 2 would win, so the rank is not stabilized yet.
        // Note that the same happens if player 3 were to raze all the other hills.
        game.scores = vec![3, 0, 1, 1];

        game.check_for_endgame();

        assert!(!game.finished);
        assert!(game.finished_reason.is_none());
        // Sanity check to make sure the original scores are not changed
        assert_eq!(game.scores, vec![3, 0, 1, 1]);
    }

    #[test]
    fn when_checking_for_endgame_if_the_max_number_of_turns_is_reached_the_game_ends() {
        let map = "\
            rows 3
            cols 3
            players 2
            m 0..
            m ...
            m ..1";
        let mut game = Game::new(map, 4, 5, 1, 5, 1500, 0, None);
        game.turn = 1500;

        game.check_for_endgame();

        assert!(game.finished);
        assert_eq!(game.finished_reason, Some(FinishedReason::TurnLimitReached));
    }
}
