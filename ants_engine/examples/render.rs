use ants_engine::Game;
use std::path::Path;

fn main() {
    let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
    let mut game = Game::new(map_file.to_str().unwrap(), 2);

    game.start();
    game.draw();
}
