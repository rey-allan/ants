use ants_engine::Game;
use std::path::Path;
use std::thread::sleep;

fn main() {
    let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
    let mut game = Game::new(map_file.to_str().unwrap(), 4, 5, 1, 5, 0);

    game.start();
    game.draw();

    // Wait for 1 second for visualization
    sleep(std::time::Duration::from_secs(1));

    game.update(vec![]);
    game.draw();
}
