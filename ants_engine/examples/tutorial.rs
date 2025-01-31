use ants_engine::Game;
use std::fs;
use std::path::Path;
use std::thread::sleep;

fn main() {
    let map_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/example.map");
    let map_contents = match fs::read_to_string(map_file) {
        Ok(contents) => contents,
        Err(e) => panic!("Error reading map file: {}", e),
    };

    let mut game = Game::new(&map_contents, 4, 5, 1, 5, 0);

    game.start();
    game.draw();

    // Wait for 1 second for visualization
    sleep(std::time::Duration::from_secs(1));

    game.update(vec![]);
    game.draw();
}
