use regex::Regex;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Ant {
        id: String,
        player: usize,
        is_alive: bool,
    },
    Food,
    Hill {
        player: usize,
    },
    Water,
}

impl Cell {
    pub fn from_char(value: char) -> Option<Cell> {
        match value {
            // Ignore land cells to reduce memory usage
            '.' => None,
            // Max 10 players
            'a'..='j' => Some(Cell::Ant {
                // Generate a uuid for the ant
                id: Uuid::new_v4().to_string(),
                // Convert char to digit for player number where 'a' is 0 and so on
                player: value as usize - 'a' as usize,
                is_alive: true,
            }),
            '*' => Some(Cell::Food),
            // Max 10 players
            '0'..='9' => Some(Cell::Hill {
                player: value.to_digit(10).unwrap() as usize,
            }),
            '%' => Some(Cell::Water),
            _ => panic!("Invalid character value: {}", value),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    width: usize,
    height: usize,
    cells: Vec<Option<Cell>>,
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
                    if let Some(cell) = Cell::from_char(value) {
                        map.set(row, col, cell);
                    }
                });
            });

        map
    }

    pub fn get(&self, row: usize, col: usize) -> &Option<Cell> {
        self.cells.get(row * self.width + col).unwrap()
    }

    pub fn set(&mut self, row: usize, col: usize, value: Cell) {
        self.cells[row * self.width + col] = Some(value);
    }

    fn new(width: usize, height: usize) -> Map {
        Map {
            width,
            height,
            cells: vec![None; width * height],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn when_getting_a_cell_by_row_and_col_the_correct_cell_value_is_returned() {
        let map = "\
            rows 3
            cols 3
            players 1
            m .b.
            m *0%";
        let map = Map::parse(map);

        assert!(map.get(0, 0).is_none());
        assert_eq!(map.get(1, 0).as_ref().unwrap(), &Cell::Food);
        assert_eq!(map.get(1, 1).as_ref().unwrap(), &Cell::Hill { player: 0 });
        assert_eq!(map.get(1, 2).as_ref().unwrap(), &Cell::Water);

        if let Cell::Ant {
            id,
            player,
            is_alive,
        } = map.get(0, 1).as_ref().unwrap()
        {
            assert_eq!(id.len(), 36);
            assert_eq!(player, &1);
            assert_eq!(is_alive, &true);
        } else {
            panic!("Expected an Ant cell at (0, 1)");
        }
    }

    #[test]
    fn when_setting_the_value_of_a_cell_the_cell_is_correctly_updated() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let mut map = Map::parse(map);
        map.set(1, 1, Cell::Water);

        assert_eq!(map.get(1, 1).as_ref().unwrap(), &Cell::Water);
    }
}
