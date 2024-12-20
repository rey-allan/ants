use regex::Regex;

pub struct Grid {
    width: usize,
    height: usize,
    cells: Vec<Option<char>>,
}

impl Grid {
    pub fn parse(map: &str) -> Grid {
        let metadata = Regex::new(r"rows (\d+)\s+cols (\d+)")
            .unwrap()
            .captures(map)
            .unwrap();

        let height = metadata.get(1).unwrap().as_str().parse().unwrap();
        let width = metadata.get(2).unwrap().as_str().parse().unwrap();

        let mut grid = Grid::new(width, height);

        Regex::new(r"m (.*)")
            .unwrap()
            .captures_iter(map)
            .map(|captures| captures.get(1).unwrap().as_str().trim())
            .enumerate()
            .for_each(|(row, line)| {
                line.chars().enumerate().for_each(|(col, cell)| {
                    grid.set(row, col, Some(cell));
                });
            });

        grid
    }

    pub fn get(&self, row: usize, col: usize) -> &Option<char> {
        self.cells.get(row * self.width + col).unwrap()
    }

    pub fn set(&mut self, row: usize, col: usize, value: Option<char>) {
        self.cells[row * self.width + col] = value;
    }

    fn new(width: usize, height: usize) -> Grid {
        Grid {
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
    fn when_parsing_a_map_a_grid_is_created_with_the_correct_width_and_height() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let grid = Grid::parse(map);

        assert_eq!(grid.width, 2);
        assert_eq!(grid.height, 2);
    }

    #[test]
    fn when_getting_a_cell_by_row_and_col_the_correct_cell_value_is_returned() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let grid = Grid::parse(map);

        assert_eq!(grid.get(1, 1).unwrap(), '0');
    }

    #[test]
    fn when_setting_the_value_of_cell_the_cell_is_correctly_updated() {
        let map = "\
            rows 2
            cols 2
            players 1
            m ..
            m .0";
        let mut grid = Grid::parse(map);
        grid.set(1, 1, Some('1'));

        assert_eq!(grid.get(1, 1).unwrap(), '1');
    }
}
