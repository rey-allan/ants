use crate::entities::from_char;
use crate::entities::Entity;
use regex::Regex;

pub struct Map {
    width: usize,
    height: usize,
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

        let mut map = Map::new(width, height);

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

    pub fn ant_hills(&self) -> Vec<(&dyn Entity, usize, usize)> {
        self.all(|entity| matches!(entity.name(), "Hill"))
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

    pub fn get(&self, row: usize, col: usize) -> &Option<Box<dyn Entity>> {
        self.grid.get(row * self.width + col).unwrap()
    }

    pub fn set(&mut self, row: usize, col: usize, value: Box<dyn Entity>) {
        self.grid[row * self.width + col] = Some(value);
    }

    fn new(width: usize, height: usize) -> Map {
        let mut grid = Vec::with_capacity(width * height);
        // Initialize the grid with `None` values
        grid.resize_with(width * height, || None);

        Map {
            width,
            height,
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
    fn when_parsing_a_map_it_is_created_with_the_correct_width_and_height() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let map = Map::parse(map);

        assert_eq!(map.width, 2);
        assert_eq!(map.height, 2);
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
        assert_eq!(map.get(0, 1).as_ref().unwrap().player(), 1);
        assert!(map.get(0, 1).as_ref().unwrap().is_alive());
        assert_eq!(map.get(1, 0).as_ref().unwrap().name(), "Food");
        assert_eq!(map.get(1, 1).as_ref().unwrap().name(), "Hill");
        assert_eq!(map.get(1, 1).as_ref().unwrap().player(), 0);
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
        assert_eq!(ant_hills[0].0.player(), 0);
        assert_eq!(ant_hills[0].1, 0);
        assert_eq!(ant_hills[0].2, 1);

        assert_eq!(ant_hills[1].0.name(), "Hill");
        assert_eq!(ant_hills[1].0.player(), 1);
        assert_eq!(ant_hills[1].1, 1);
        assert_eq!(ant_hills[1].2, 1);

        assert_eq!(ant_hills[2].0.name(), "Hill");
        assert_eq!(ant_hills[2].0.player(), 2);
        assert_eq!(ant_hills[2].1, 2);
        assert_eq!(ant_hills[2].2, 1);
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
}
