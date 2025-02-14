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
