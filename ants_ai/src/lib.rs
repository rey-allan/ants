//! # ants_engine
//!
//! The core engine for the Ants game.
//! Inspired by [Google's Ants AI Challenge](http://ants.aichallenge.org/).

pub mod game;
pub use game::Action;
pub use game::Direction;
pub use game::FinishedReason;
pub use game::Game;
pub use game::GameState;

mod entities;
mod map;
mod replay;

use game::PlayerAnt;
use game::StateEntity;
use pyo3::prelude::*;

#[pymodule]
fn ants_ai(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Action>()?;
    m.add_class::<Direction>()?;
    m.add_class::<FinishedReason>()?;
    m.add_class::<Game>()?;
    m.add_class::<GameState>()?;
    m.add_class::<PlayerAnt>()?;
    m.add_class::<StateEntity>()?;
    Ok(())
}
