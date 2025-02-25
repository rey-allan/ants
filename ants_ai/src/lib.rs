//! # ants_engine
//!
//! The core engine for the Ants game.
//! Inspired by [Google's Ants AI Challenge](http://ants.aichallenge.org/).

use pyo3::prelude::*;

pub mod game;
pub use game::Action;
pub use game::Direction;
pub use game::FinishedReason;
pub use game::Game;
pub use game::GameState;

mod entities;
mod map;
mod replay;

// TODO: Replace this with our actual bindings for the `ants_engine` crate
#[pyfunction]
fn say_hello() -> String {
    "Hello, Ants AI from Rust!".to_string()
}

#[pymodule]
fn ants_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(say_hello, m)?)
}
