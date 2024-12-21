use crate::map::Grid;

/// The Ants game.
/// Main entry point for running the game.
pub struct Game {
    grid: Grid,
}

impl Game {
    /// Creates a new game from the string representation of a map.
    ///
    /// # Arguments
    /// * `map` - A string representation of a map.
    pub fn new(map: &str) -> Game {
        Game {
            grid: Grid::parse(map),
        }
    }
}
