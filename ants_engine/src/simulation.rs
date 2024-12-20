use crate::map::Grid;

/// A simulation of the Ants game.
/// Main entry point for running the game.
pub struct Simulation {
    grid: Grid,
}

impl Simulation {
    /// Creates a new simulation from the string representation of a map.
    ///
    /// # Arguments
    /// * `map` - A string representation of a map.
    pub fn new(map: &str) -> Simulation {
        Simulation {
            grid: Grid::parse(map),
        }
    }
}
