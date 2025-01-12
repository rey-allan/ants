use crate::entities::from_char;
use crate::entities::Entity;
use regex::Regex;

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

    pub fn get(&self, row: usize, col: usize) -> &Option<Box<dyn Entity>> {
        self.grid.get(row * self.width + col).unwrap()
    }

    pub fn set(&mut self, row: usize, col: usize, value: Box<dyn Entity>) {
        self.grid[row * self.width + col] = Some(value);
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
        radius: usize,
    ) -> Vec<(&dyn Entity, usize, usize)> {
        let (row, col) = center;
        let mut fov = Vec::new();

        // Compute the field of vision around the center coordinate
        // These are all the entities that are within the radius of the center
        // i.e. the entities whose coordinates are at most `radius` distance away from the center
        // using the euclidean distance formula: (x1 - x2)^2 + (y1 - y2)^2 <= radius^2
        for i in row.saturating_sub(radius)..=(row + radius).min(self.height - 1) {
            for j in col.saturating_sub(radius)..=(col + radius).min(self.width - 1) {
                if (i as i32 - row as i32).pow(2) + (j as i32 - col as i32).pow(2)
                    <= radius.pow(2) as i32
                {
                    // Skip the given center coordinate
                    if i == row && j == col {
                        continue;
                    }

                    if let Some(entity) = self.get(i, j) {
                        fov.push((entity.as_ref(), i, j));
                    }
                }
            }
        }

        fov
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
        assert_eq!(map.get(0, 1).as_ref().unwrap().name(), "Ant");
        assert_eq!(map.get(0, 1).as_ref().unwrap().player().unwrap(), 1);
        assert!(map.get(0, 1).as_ref().unwrap().alive().unwrap());
        assert_eq!(map.get(1, 0).as_ref().unwrap().name(), "Food");
        assert_eq!(map.get(1, 1).as_ref().unwrap().name(), "Hill");
        assert_eq!(map.get(1, 1).as_ref().unwrap().player().unwrap(), 0);
        assert_eq!(map.get(1, 2).as_ref().unwrap().name(), "Water");
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

        assert_eq!(map.get(1, 1).as_ref().unwrap().name(), "Water");
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
            players 1
            m ..*..
            m .0*%.
            m .*a.%
            m .1...
            m ..*..";
        let map = Map::parse(map);

        let fov = map.field_of_vision((2, 2), 2);

        assert_eq!(fov.len(), 8);
        assert_eq!(map.get(0, 2).as_ref().unwrap().name(), "Food");
        assert_eq!(map.get(1, 1).as_ref().unwrap().name(), "Hill");
        assert_eq!(map.get(1, 1).as_ref().unwrap().player().unwrap(), 0);
        assert_eq!(map.get(1, 2).as_ref().unwrap().name(), "Food");
        assert_eq!(map.get(1, 3).as_ref().unwrap().name(), "Water");
        assert_eq!(map.get(2, 1).as_ref().unwrap().name(), "Food");
        assert_eq!(map.get(2, 4).as_ref().unwrap().name(), "Water");
        assert_eq!(map.get(3, 1).as_ref().unwrap().name(), "Hill");
        assert_eq!(map.get(3, 1).as_ref().unwrap().player().unwrap(), 1);
        assert_eq!(map.get(4, 2).as_ref().unwrap().name(), "Food");
    }
}
