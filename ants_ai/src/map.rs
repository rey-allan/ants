use crate::entities::{from_char, player_to_color, Ant, Entity, Hill};
use crossterm::{
    cursor::Hide,
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use regex::Regex;
use std::io::{stdout, Write};

pub struct Map {
    width: usize,
    height: usize,
    players: usize,
    grid: Vec<Option<Box<dyn Entity>>>,
}

impl Map {
    pub fn parse(map_contents: &str) -> Map {
        let metadata = Regex::new(r"rows (\d+)\s+cols (\d+)")
            .unwrap()
            .captures(map_contents)
            .unwrap();

        let height = metadata.get(1).unwrap().as_str().parse().unwrap();
        let width = metadata.get(2).unwrap().as_str().parse().unwrap();

        let players = Regex::new(r"players (\d+)")
            .unwrap()
            .captures(map_contents)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str()
            .parse()
            .unwrap();

        let mut map = Map::new(width, height, players);

        Regex::new(r"m (.*)")
            .unwrap()
            .captures_iter(map_contents)
            .map(|captures| captures.get(1).unwrap().as_str().trim())
            .enumerate()
            .for_each(|(row, line)| {
                line.chars().enumerate().for_each(|(col, value)| {
                    if let Some(entity) = from_char(value) {
                        map.set(row, col, entity);
                    }
                });
            });

        map
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&Box<dyn Entity>> {
        self.grid
            .get(row * self.width + col)
            .and_then(|opt| opt.as_ref())
    }

    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut Box<dyn Entity>> {
        self.grid
            .get_mut(row * self.width + col)
            .and_then(|opt| opt.as_mut())
    }

    pub fn set(&mut self, row: usize, col: usize, value: Box<dyn Entity>) {
        self.grid[row * self.width + col] = Some(value);
    }

    pub fn remove(&mut self, row: usize, col: usize) {
        self.grid[row * self.width + col] = None;
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn players(&self) -> usize {
        self.players
    }

    pub fn ant_hills(&self) -> Vec<(&dyn Entity, usize, usize)> {
        self.all(|entity| matches!(entity.name(), "Hill"))
    }

    pub fn ants(&self) -> Vec<(&dyn Entity, usize, usize)> {
        self.all(|entity| matches!(entity.name(), "Ant"))
    }

    pub fn food(&self) -> Vec<(usize, usize)> {
        self.all(|entity| matches!(entity.name(), "Food"))
            .into_iter()
            .map(|(_, row, col)| (row, col))
            .collect()
    }

    pub fn land(&self) -> Vec<(usize, usize)> {
        // Land are all the empty cells
        // As with the `all` method, this is inefficient but should be fine for the size of our maps
        // If we end up using larger maps, we will need to optimize this
        self.grid
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                if entity.is_none() {
                    let row = index / self.width;
                    let col = index % self.width;
                    return Some((row, col));
                }
                None
            })
            .collect()
    }

    pub fn land_around(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        // For each coordinate around the given one, check if the cell is empty
        // If it is, add it to the list of coordinates
        let mut lands = Vec::new();

        // For each coordinate around the given one in all 8 directions
        for i in -1..=1 {
            for j in -1..=1 {
                let n_row = row as i32 + i;
                let n_col = col as i32 + j;

                // Skip if the coordinate is out of bounds
                if n_row < 0
                    || n_row >= self.height as i32
                    || n_col < 0
                    || n_col >= self.width as i32
                {
                    continue;
                }

                // Skip if the cell is not empty
                if self.get(n_row as usize, n_col as usize).is_some() {
                    continue;
                }

                // If the cell is empty then it's land
                lands.push((n_row as usize, n_col as usize));
            }
        }

        lands
    }

    pub fn field_of_vision(
        &self,
        center: (usize, usize),
        radius2: usize,
    ) -> Vec<(&dyn Entity, usize, usize)> {
        let (row, col) = center;
        let radius = (radius2 as f64).sqrt() as usize;
        let mut fov = Vec::new();

        // Compute the field of vision around the center coordinate
        // These are all the entities that are within the radius of the center
        // i.e. the entities whose coordinates are at most `radius` distance away from the center
        // using the euclidean distance formula: (x1 - x2)^2 + (y1 - y2)^2 <= radius^2
        for i in row.saturating_sub(radius)..=(row + radius).min(self.height - 1) {
            for j in col.saturating_sub(radius)..=(col + radius).min(self.width - 1) {
                if (i as i32 - row as i32).pow(2) + (j as i32 - col as i32).pow(2) <= radius2 as i32
                {
                    if let Some(entity) = self.get(i, j) {
                        // If the entity is on a hill (i.e. an ant on a hill), include the hill in the field of vision
                        if let Some(hill) = entity.on_ant_hill() {
                            fov.push((hill.as_ref(), i, j));
                        }

                        // Skip the actual entity if it's the given center coordinate
                        if i == row && j == col {
                            continue;
                        }

                        // Add the entity to the field of vision
                        fov.push((entity.as_ref(), i, j));
                    }
                }
            }
        }

        fov
    }

    pub fn move_entity(&mut self, from: (usize, usize), to: (usize, usize)) -> bool {
        if !self.is_valid_move(from, to) {
            return false;
        }

        let collision = {
            match self.get(to.0, to.1) {
                Some(entity) => entity.name() == "Ant" && entity.alive().is_some_and(|alive| alive),
                None => false,
            }
        };

        // If there was a collision, both ants die
        if collision {
            self.get_mut(from.0, from.1).unwrap().set_alive(false);
            self.get_mut(to.0, to.1).unwrap().set_alive(false);

            // Even though the ants died from a collision, a movement still occurred
            return true;
        }

        // Actually move the ant
        let ant = {
            let entity = self.get(from.0, from.1).unwrap();
            // Check if the ant is moving to a hill
            let to_hill: Option<Box<dyn Entity>> = {
                if let Some(to) = self.get(to.0, to.1) {
                    if to.name() == "Hill" {
                        Some(Box::new(Hill::new(
                            to.player().unwrap(),
                            to.alive().unwrap(),
                        )))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            Box::new(Ant::new(
                entity.id().to_string(),
                entity.player().unwrap(),
                entity.alive().unwrap(),
                to_hill,
            ))
        };
        self.set(to.0, to.1, ant);

        // If the ant was on a hill, replace the location with the hill, otherwise remove the ant
        let hill = self.get(from.0, from.1).and_then(|entity| {
            entity
                .on_ant_hill()
                .as_ref()
                .map(|hill| (hill.player().unwrap(), hill.alive().unwrap()))
        });

        if let Some(hill) = hill {
            self.set(from.0, from.1, Box::new(Hill::new(hill.0, hill.1)));
        } else {
            self.remove(from.0, from.1);
        }

        true
    }

    pub fn draw(&self, turn: usize, scores: &[usize], ants: &[usize], hive: &[usize]) {
        let mut stdout = stdout();

        // Display information about the game
        execute!(
            stdout,
            Clear(ClearType::All),
            Hide,
            Print("Players: "),
            Print(self.players.to_string()),
            Print("\nTurn: "),
            Print(turn.to_string())
        )
        .unwrap();

        // Display information about the players
        for player in 0..self.players {
            execute!(
                stdout,
                SetForegroundColor(player_to_color(player)),
                Print("\nPlayer "),
                Print(player.to_string()),
                Print(": "),
                Print("Score = "),
                Print(scores[player].to_string()),
                Print(", Ants = "),
                Print(ants[player].to_string()),
                Print(", Hive = "),
                Print(hive[player].to_string()),
                SetForegroundColor(Color::Reset)
            )
            .unwrap();
        }
        execute!(stdout, Print("\n\n")).unwrap();

        // Display the map
        for row in 0..self.height {
            for col in 0..self.width {
                let entity = self.get(row, col);
                execute!(
                    stdout,
                    SetForegroundColor(entity.map_or(Color::Reset, |entity| entity.color())),
                    Print(entity.map_or('.', |entity| entity.char())),
                    SetForegroundColor(Color::Reset)
                )
                .unwrap();
            }
            execute!(stdout, Print("\n")).unwrap();
        }

        stdout.flush().unwrap();
    }

    fn new(width: usize, height: usize, players: usize) -> Map {
        let mut grid = Vec::with_capacity(width * height);
        // Initialize the grid with `None` values
        grid.resize_with(width * height, || None);

        Map {
            width,
            height,
            players,
            grid,
        }
    }

    fn all(&self, filter: fn(&Box<dyn Entity>) -> bool) -> Vec<(&dyn Entity, usize, usize)> {
        // Inefficient way to get all entities using some filter (linear time complexity)
        // But it should be fine since maps are small, the largest having roughly 15K or so cells
        // If we end up using larger maps, we will need to optimize this
        self.grid
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                if let Some(entity) = entity {
                    if filter(entity) {
                        let row = index / self.width;
                        let col = index % self.width;
                        return Some((entity.as_ref(), row, col));
                    }
                }
                None
            })
            .collect()
    }

    fn is_valid_move(&self, from: (usize, usize), to: (usize, usize)) -> bool {
        // If there is no movement, the move is invalid
        if from == to {
            return false;
        }

        // If the coordinates are out of bounds, the move is invalid
        if from.0 >= self.height
            || from.1 >= self.width
            || to.0 >= self.height
            || to.1 >= self.width
        {
            return false;
        }

        let from = self.get(from.0, from.1);
        if from.is_none() {
            return false;
        }

        // Only alive ants can move
        let from = from.unwrap();
        if from.name() != "Ant" || from.alive().is_none() || !from.alive().unwrap() {
            return false;
        }

        if let Some(to) = self.get(to.0, to.1) {
            // Water, food or a dead ant blocks the movement
            if to.name() == "Water"
                || to.name() == "Food"
                || (to.name() == "Ant" && !to.alive().unwrap())
            {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Water;

    #[test]
    fn when_parsing_a_map_it_is_created_with_the_correct_width_height_and_players() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let map = Map::parse(map);

        assert_eq!(map.width, 2);
        assert_eq!(map.height, 2);
        assert_eq!(map.players, 1);
    }

    #[test]
    fn when_getting_a_cell_by_row_and_col_the_correct_entity_is_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m .b.
            m *0%";
        let map = Map::parse(map);

        assert!(map.get(0, 0).is_none());
        assert_eq!(map.get(0, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(0, 1).unwrap().player().unwrap(), 1);
        assert!(map.get(0, 1).unwrap().alive().unwrap());
        assert_eq!(map.get(1, 0).unwrap().name(), "Food");
        assert_eq!(map.get(1, 1).unwrap().name(), "Hill");
        assert_eq!(map.get(1, 1).unwrap().player().unwrap(), 0);
        assert_eq!(map.get(1, 2).unwrap().name(), "Water");
    }

    #[test]
    fn when_getting_a_cell_by_row_and_col_and_mutating_it_the_entity_is_correctly_updated() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .a";
        let mut map = Map::parse(map);
        map.get_mut(1, 1).unwrap().set_alive(false);

        assert!(!map.get(1, 1).unwrap().alive().unwrap());
    }

    #[test]
    fn when_setting_the_value_of_an_entity_the_entity_is_correctly_updated() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let mut map = Map::parse(map);
        map.set(1, 1, Box::new(Water));

        assert_eq!(map.get(1, 1).unwrap().name(), "Water");
    }

    #[test]
    fn when_removing_an_entity_the_cell_becomes_empty() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let mut map = Map::parse(map);
        map.remove(1, 1);

        assert!(map.get(1, 1).is_none());
    }

    #[test]
    fn when_getting_all_ant_hills_the_correct_entities_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 3
            m .0.
            m .1.
            m .2.";
        let map = Map::parse(map);

        let ant_hills = map.ant_hills();
        assert_eq!(ant_hills.len(), 3);

        assert_eq!(ant_hills[0].0.name(), "Hill");
        assert_eq!(ant_hills[0].0.player().unwrap(), 0);
        assert_eq!(ant_hills[0].1, 0);
        assert_eq!(ant_hills[0].2, 1);

        assert_eq!(ant_hills[1].0.name(), "Hill");
        assert_eq!(ant_hills[1].0.player().unwrap(), 1);
        assert_eq!(ant_hills[1].1, 1);
        assert_eq!(ant_hills[1].2, 1);

        assert_eq!(ant_hills[2].0.name(), "Hill");
        assert_eq!(ant_hills[2].0.player().unwrap(), 2);
        assert_eq!(ant_hills[2].1, 2);
        assert_eq!(ant_hills[2].2, 1);
    }

    #[test]
    fn when_getting_all_ants_the_correct_entities_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 3
            m ..a
            m b..
            m .c.";
        let map = Map::parse(map);

        let ants = map.ants();
        assert_eq!(ants.len(), 3);

        assert_eq!(ants[0].0.name(), "Ant");
        assert_eq!(ants[0].0.player().unwrap(), 0);
        assert_eq!(ants[0].1, 0);
        assert_eq!(ants[0].2, 2);

        assert_eq!(ants[1].0.name(), "Ant");
        assert_eq!(ants[1].0.player().unwrap(), 1);
        assert_eq!(ants[1].1, 1);
        assert_eq!(ants[1].2, 0);

        assert_eq!(ants[2].0.name(), "Ant");
        assert_eq!(ants[2].0.player().unwrap(), 2);
        assert_eq!(ants[2].1, 2);
        assert_eq!(ants[2].2, 1);
    }

    #[test]
    fn when_getting_all_food_the_correct_entities_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m .0.
            m .*.
            m .0.";
        let map = Map::parse(map);

        let food = map.food();
        assert_eq!(food.len(), 1);

        assert_eq!(food[0].0, 1);
        assert_eq!(food[0].1, 1);
    }

    #[test]
    fn when_getting_all_land_the_correct_coordinates_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m .0.
            m .*.
            m .0.";
        let map = Map::parse(map);

        let land = map.land();
        let expected_land = vec![(0, 0), (0, 2), (1, 0), (1, 2), (2, 0), (2, 2)];

        assert_eq!(land.len(), 6);
        assert_eq!(land, expected_land);
    }

    #[test]
    fn when_getting_all_land_around_a_middle_cell_the_correct_coordinates_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .0.
            m ...";
        let map = Map::parse(map);

        let lands = map.land_around(1, 1);
        let expected_lands = vec![
            (0, 0),
            (0, 1),
            (0, 2),
            (1, 0),
            (1, 2),
            (2, 0),
            (2, 1),
            (2, 2),
        ];

        assert_eq!(lands.len(), 8);
        assert_eq!(lands, expected_lands);
    }

    #[test]
    fn when_getting_all_land_around_an_edge_cell_the_correct_coordinates_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m ...
            m .0.";
        let map = Map::parse(map);

        let lands = map.land_around(2, 1);
        let expected_lands = vec![(1, 0), (1, 1), (1, 2), (2, 0), (2, 2)];

        assert_eq!(lands.len(), 5);
        assert_eq!(lands, expected_lands);
    }

    #[test]
    fn when_getting_all_land_around_a_corner_cell_the_correct_coordinates_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m 0..
            m ...
            m ...";
        let map = Map::parse(map);

        let lands = map.land_around(0, 0);
        let expected_lands = vec![(0, 1), (1, 0), (1, 1)];

        assert_eq!(lands.len(), 3);
        assert_eq!(lands, expected_lands);
    }

    #[test]
    fn when_getting_all_land_around_a_cell_with_no_land_no_coordinates_are_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m .*0
            m .**
            m ...";
        let map = Map::parse(map);

        let lands = map.land_around(0, 2);

        assert_eq!(lands.len(), 0);
    }

    #[test]
    fn when_getting_the_field_of_vision_of_a_cell_the_correct_entities_are_returned() {
        let map = "\
            rows 5
            cols 5
            players 2
            m ..*..
            m ..*%.
            m .*A.%
            m .1...
            m ..*..";
        let map = Map::parse(map);

        // Get the field of vision of the ant at (2, 2), on top of its own hill, with a radius of 2
        let fov = map.field_of_vision((2, 2), 4);

        assert_eq!(fov.len(), 8);

        assert_eq!(fov[0].0.name(), "Food");
        assert_eq!(fov[0].1, 0);
        assert_eq!(fov[0].2, 2);

        assert_eq!(fov[1].0.name(), "Food");
        assert_eq!(fov[1].1, 1);
        assert_eq!(fov[1].2, 2);

        assert_eq!(fov[2].0.name(), "Water");
        assert_eq!(fov[2].1, 1);
        assert_eq!(fov[2].2, 3);

        assert_eq!(fov[3].0.name(), "Food");
        assert_eq!(fov[3].1, 2);
        assert_eq!(fov[3].2, 1);

        // The ant is on its own hill which should be included in the field of vision
        // The ant itself should not be included in the field of vision because it's the center
        assert_eq!(fov[4].0.name(), "Hill");
        assert_eq!(fov[4].0.player().unwrap(), 0);
        assert_eq!(fov[4].1, 2);
        assert_eq!(fov[4].2, 2);

        assert_eq!(fov[5].0.name(), "Water");
        assert_eq!(fov[5].1, 2);
        assert_eq!(fov[5].2, 4);

        assert_eq!(fov[6].0.name(), "Hill");
        assert_eq!(fov[6].0.player().unwrap(), 1);
        assert_eq!(fov[6].1, 3);
        assert_eq!(fov[6].2, 1);

        assert_eq!(fov[7].0.name(), "Food");
        assert_eq!(fov[7].1, 4);
        assert_eq!(fov[7].2, 2);
    }

    #[test]
    fn when_moving_an_ant_to_an_empty_cell_the_ant_is_moved() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m ...";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((1, 1), (0, 1));

        assert!(map.get(1, 1).is_none());
        assert_eq!(map.get(0, 1).unwrap().name(), "Ant");
        assert!(did_move);
    }

    #[test]
    fn when_moving_an_ant_from_a_hill_to_an_empty_cell_the_ant_is_moved_and_the_hill_is_restored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .A.
            m ...";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((1, 1), (0, 1));

        assert_eq!(map.get(0, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(1, 1).unwrap().name(), "Hill");
        assert!(did_move);
    }

    #[test]
    fn when_moving_an_ant_to_a_hill_the_ant_is_moved_on_the_hill() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m .0.";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((1, 1), (2, 1));

        assert!(map.get(1, 1).is_none());
        assert_eq!(map.get(2, 1).unwrap().name(), "Ant");
        assert!(map.get(2, 1).unwrap().on_ant_hill().is_some());
        assert_eq!(
            map.get(2, 1)
                .unwrap()
                .on_ant_hill()
                .unwrap()
                .player()
                .unwrap(),
            0
        );
        assert!(map
            .get(2, 1)
            .unwrap()
            .on_ant_hill()
            .unwrap()
            .alive()
            .unwrap());
        assert!(did_move);
    }

    #[test]
    fn when_moving_an_empty_entity_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m ...";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((0, 1), (0, 2));

        assert!(map.get(0, 1).is_none());
        assert!(!did_move);
    }

    #[test]
    fn when_moving_an_entity_that_is_not_an_ant_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m %..
            m .a.
            m ...";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((0, 0), (1, 0));

        assert_eq!(map.get(0, 0).unwrap().name(), "Water");
        assert!(map.get(1, 0).is_none());
        assert!(!did_move);
    }

    #[test]
    fn when_moving_a_dead_ant_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m ...";
        let mut map = Map::parse(map);
        map.get_mut(1, 1).unwrap().set_alive(false);
        let did_move = map.move_entity((1, 1), (0, 1));

        assert!(map.get(0, 1).is_none());
        assert_eq!(map.get(1, 1).unwrap().name(), "Ant");
        assert!(!did_move);
    }

    #[test]
    fn when_moving_an_ant_to_water_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m .%.";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((1, 1), (2, 1));

        assert_eq!(map.get(1, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(2, 1).unwrap().name(), "Water");
        assert!(!did_move);
    }

    #[test]
    fn when_moving_an_ant_to_food_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a*
            m ...";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((1, 1), (1, 2));

        assert_eq!(map.get(1, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(1, 2).unwrap().name(), "Food");
        assert!(!did_move);
    }

    #[test]
    fn when_moving_an_ant_outside_of_the_right_side_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 2
            m ...
            m ..a
            m ...";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((1, 2), (1, 3));

        assert_eq!(map.get(1, 2).unwrap().name(), "Ant");
        assert!(!did_move);
    }

    #[test]
    fn when_moving_an_ant_outside_of_the_bottom_side_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 2
            m ...
            m ...
            m ..a";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((2, 2), (3, 2));

        assert_eq!(map.get(2, 2).unwrap().name(), "Ant");
        assert!(!did_move);
    }

    #[test]
    fn when_moving_an_ant_to_a_cell_with_another_ant_both_ants_die() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m .b.";
        let mut map = Map::parse(map);
        let did_move = map.move_entity((1, 1), (2, 1));

        assert!(!map.get(1, 1).unwrap().alive().unwrap());
        assert!(!map.get(2, 1).unwrap().alive().unwrap());
        assert!(did_move);
    }

    #[test]
    fn when_moving_an_ant_to_a_cell_with_another_dead_ant_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m .a.";
        let mut map = Map::parse(map);
        map.get_mut(2, 1).unwrap().set_alive(false);
        let did_move = map.move_entity((1, 1), (2, 1));

        assert_eq!(map.get(1, 1).unwrap().name(), "Ant");
        assert!(map.get(1, 1).unwrap().alive().unwrap());

        assert_eq!(map.get(2, 1).unwrap().name(), "Ant");
        assert!(!map.get(2, 1).unwrap().alive().unwrap());
        assert!(!did_move);
    }

    #[test]
    fn when_moving_an_ant_to_the_same_cell_movement_is_ignored() {
        let map = "\
            rows 3
            cols 3
            players 1
            m ...
            m .a.
            m ...";
        let mut map = Map::parse(map);
        let id = map.get(1, 1).unwrap().id().to_string();

        let did_move = map.move_entity((1, 1), (1, 1));

        assert_eq!(map.get(1, 1).unwrap().name(), "Ant");
        assert_eq!(map.get(1, 1).unwrap().id(), id);
        assert!(map.get(1, 1).unwrap().alive().unwrap());
        assert!(!did_move);
    }
}
