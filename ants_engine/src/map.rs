use regex::Regex;

struct Grid {
    width: usize,
    height: usize,
    cells: Vec<Option<char>>,
}

impl Grid {
    fn new(width: usize, height: usize) -> Grid {
        Grid {
            width,
            height,
            cells: vec![None; width * height],
        }
    }

    fn get(&self, row: usize, col: usize) -> &Option<char> {
        self.cells.get(row * self.width + col).unwrap()
    }

    fn set(&mut self, row: usize, col: usize, value: Option<char>) {
        self.cells[row * self.width + col] = value;
    }
}

fn parse(map: &str) -> Grid {
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

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = "\
        rows 20
        cols 20
        players 2
        m ....................
        m ....................
        m ....................
        m ....................
        m ....................
        m ....................
        m .....*..............
        m ......%..b..........
        m ....................
        m ....................
        m ........aa..........
        m ....................
        m ....................
        m ....................
        m ....................
        m ....................
        m ....................
        m ....................
        m ....................
        m ....................";

    #[test]
    fn when_parsing_a_map_a_grid_is_created_with_the_correct_width_and_height() {
        let grid = parse(MAP);

        assert_eq!(grid.width, 20);
        assert_eq!(grid.height, 20);
    }

    #[test]
    fn when_parsing_a_map_a_grid_is_created_with_the_correct_cells() {
        let grid = parse(MAP);

        assert_eq!(grid.get(6, 5).unwrap(), '*');
        assert_eq!(grid.get(7, 6).unwrap(), '%');
        assert_eq!(grid.get(7, 9).unwrap(), 'b');
        assert_eq!(grid.get(10, 8).unwrap(), 'a');
        assert_eq!(grid.get(10, 9).unwrap(), 'a');
    }
}
