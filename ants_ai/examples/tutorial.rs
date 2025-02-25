use ants_engine::{Action, Direction, Game};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fs;
use std::path::Path;

struct RandomAgent {
    rng: StdRng,
}

impl RandomAgent {
    fn new(seed: u64) -> RandomAgent {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    fn act(&mut self, row: usize, col: usize) -> Action {
        let direction: Direction = self.rng.gen();
        Action::new(row, col, direction)
    }
}

fn main() {
    let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/maps/tutorial.map");
    let map_contents = match fs::read_to_string(map_file) {
        Ok(contents) => contents,
        Err(e) => panic!("Error reading map file: {}", e),
    };

    let replay_filename = "/tmp/tutorial_replay.json".to_string();

    let mut game = Game::new(&map_contents, 4, 5, 1, 5, 1500, 0, Some(replay_filename));
    let mut player1 = RandomAgent::new(0);
    let mut player2 = RandomAgent::new(1);

    let mut state = game.start();
    while !state.finished {
        // Generate random actions for each ant belonging to each player
        let mut actions = vec![];
        for (player, ants) in state.ants.iter().enumerate() {
            for ant in ants {
                let action = match player {
                    0 => player1.act(ant.row, ant.col),
                    1 => player2.act(ant.row, ant.col),
                    _ => panic!("Invalid player number"),
                };
                actions.push(action);
            }
        }

        // Update the game state with the generated actions
        state = game.update(actions);
    }

    println!(
        "\nGame finished due to: {:?}",
        state.finished_reason.unwrap()
    );
}
