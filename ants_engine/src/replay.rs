use serde_json::json;
use std::{collections::HashMap, fs::File, io::BufWriter};

pub fn create_replay_logger(
    filename: Option<String>,
    players: usize,
    map_width: usize,
    map_height: usize,
    map_contents: String,
) -> Box<dyn ReplayLogger> {
    match filename {
        None => Box::new(NoOpReplayLogger {}),
        Some(filename) => Box::new(JsonReplayLogger::new(
            filename,
            players,
            map_width,
            map_height,
            map_contents,
        )),
    }
}

pub trait ReplayLogger {
    #[allow(unused_variables)]
    fn log_turn(&mut self, turn: usize, ants: Vec<usize>, hive: Vec<usize>, scores: Vec<usize>) {}

    fn log_spawn_ant(&mut self, turn: usize, player: usize, location: (usize, usize)) {
        self.log_spawn(turn, "Ant".to_string(), Some(player), location);
    }

    fn log_spawn_food(&mut self, turn: usize, location: (usize, usize)) {
        self.log_spawn(turn, "Food".to_string(), None, location);
    }

    #[allow(unused_variables)]
    fn log_spawn(
        &mut self,
        turn: usize,
        entity: String,
        player: Option<usize>,
        location: (usize, usize),
    ) {
    }

    fn clear(&mut self) {}

    fn save(&self) {}
}

#[derive(serde::Serialize)]
enum EventType {
    Spawn,
    Kill,
    Move,
    Attack,
    Havest,
}

#[derive(serde::Serialize)]
struct Event {
    event_type: EventType,
    entity: String,
    player: Option<usize>,
    location: (usize, usize),
    destination: Option<(usize, usize)>,
}

struct Turn {
    turn: usize,
    ants: Vec<usize>,
    hive: Vec<usize>,
    scores: Vec<usize>,
}

struct NoOpReplayLogger;
impl ReplayLogger for NoOpReplayLogger {}

struct JsonReplayLogger {
    filename: String,
    players: usize,
    map_width: usize,
    map_height: usize,
    map_contents: String,
    turns: Vec<Turn>,
    events: HashMap<usize, Vec<Event>>,
}

impl JsonReplayLogger {
    pub fn new(
        filename: String,
        players: usize,
        map_width: usize,
        map_height: usize,
        map_contents: String,
    ) -> Self {
        JsonReplayLogger {
            filename,
            players,
            map_width,
            map_height,
            map_contents,
            turns: Vec::new(),
            events: HashMap::new(),
        }
    }
}

impl ReplayLogger for JsonReplayLogger {
    fn log_turn(&mut self, turn: usize, ants: Vec<usize>, hive: Vec<usize>, scores: Vec<usize>) {
        self.turns.push(Turn {
            turn,
            ants,
            hive,
            scores,
        });
    }

    fn log_spawn(
        &mut self,
        turn: usize,
        entity: String,
        player: Option<usize>,
        location: (usize, usize),
    ) {
        self.events.entry(turn).or_default().push(Event {
            event_type: EventType::Spawn,
            entity,
            player,
            location,
            destination: None,
        });
    }

    fn clear(&mut self) {
        self.turns.clear();
        self.events.clear();
    }

    fn save(&self) {
        let file = File::create(&self.filename).unwrap();
        let turns: Vec<_> = self
            .turns
            .iter()
            .map(|turn| {
                json!({
                    "turn": turn.turn,
                    "ants": turn.ants,
                    "hive": turn.hive,
                    "scores": turn.scores,
                    "events": self.events.get(&turn.turn).unwrap_or(&Vec::new()),
                })
            })
            .collect();

        let data = json!({
            "players": self.players,
            "map": {
                "width": self.map_width,
                "height": self.map_height,
                "contents": self.map_contents,
            },
            "turns": turns,
        });

        let mut writer = BufWriter::new(&file);
        serde_json::to_writer_pretty(&mut writer, &data).unwrap();
    }
}
