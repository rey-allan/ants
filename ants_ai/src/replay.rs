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

pub trait ReplayLogger: Send + Sync {
    #[allow(unused_variables)]
    fn log_turn(&mut self, turn: usize, ants: Vec<usize>, hive: Vec<usize>, scores: Vec<usize>) {}

    #[allow(unused_variables)]
    fn log_end_game(&mut self, reason: String) {}

    #[allow(unused_variables)]
    fn log_event(&mut self, turn: usize, event: Event) {}

    fn clear(&mut self) {}

    fn save(&self) {}

    fn log_spawn_ant(&mut self, turn: usize, id: String, player: usize, location: (usize, usize)) {
        self.log_spawn(turn, "Ant".to_string(), Some(id), Some(player), location);
    }

    fn log_spawn_food(&mut self, turn: usize, location: (usize, usize)) {
        self.log_spawn(turn, "Food".to_string(), None, None, location);
    }

    fn log_remove_ant(&mut self, turn: usize, id: String) {
        self.log_remove(turn, Some(id), "Ant".to_string(), None);
    }

    fn log_move_ant(
        &mut self,
        turn: usize,
        id: String,
        location: (usize, usize),
        destination: (usize, usize),
    ) {
        self.log_event(
            turn,
            Event {
                event_type: EventType::Move,
                entity: "Ant".to_string(),
                entity_id: Some(id),
                player: None,
                location: Some(location),
                destination: Some(destination),
            },
        );
    }

    fn log_remove_hill(&mut self, turn: usize, location: (usize, usize)) {
        self.log_remove(turn, None, "Hill".to_string(), Some(location));
    }

    fn log_remove_food(&mut self, turn: usize, location: (usize, usize)) {
        self.log_remove(turn, None, "Food".to_string(), Some(location));
    }

    fn log_attack(&mut self, turn: usize, location: (usize, usize), destination: (usize, usize)) {
        self.log_event(
            turn,
            Event {
                event_type: EventType::Attack,
                entity: "Ant".to_string(),
                entity_id: None,
                player: None,
                location: Some(location),
                destination: Some(destination),
            },
        );
    }

    fn log_spawn(
        &mut self,
        turn: usize,
        entity: String,
        entity_id: Option<String>,
        player: Option<usize>,
        location: (usize, usize),
    ) {
        self.log_event(
            turn,
            Event {
                event_type: EventType::Spawn,
                entity,
                entity_id,
                player,
                location: Some(location),
                destination: None,
            },
        );
    }

    fn log_remove(
        &mut self,
        turn: usize,
        id: Option<String>,
        entity: String,
        location: Option<(usize, usize)>,
    ) {
        self.log_event(
            turn,
            Event {
                event_type: EventType::Remove,
                entity,
                entity_id: id,
                player: None,
                location,
                destination: None,
            },
        );
    }
}

#[derive(serde::Serialize)]
enum EventType {
    Spawn,
    Remove,
    Move,
    Attack,
}

#[derive(serde::Serialize)]
pub struct Event {
    event_type: EventType,
    entity: String,
    entity_id: Option<String>,
    player: Option<usize>,
    location: Option<(usize, usize)>,
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
    finished_reason: Option<String>,
}

impl JsonReplayLogger {
    pub fn new(
        filename: String,
        players: usize,
        map_width: usize,
        map_height: usize,
        map_contents: String,
    ) -> JsonReplayLogger {
        JsonReplayLogger {
            filename,
            players,
            map_width,
            map_height,
            map_contents,
            turns: Vec::new(),
            events: HashMap::new(),
            finished_reason: None,
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

    fn log_end_game(&mut self, reason: String) {
        self.finished_reason = Some(reason);
    }

    fn log_event(&mut self, turn: usize, event: Event) {
        self.events.entry(turn).or_default().push(event);
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
            "finished_reason": self.finished_reason,
        });

        let mut writer = BufWriter::new(&file);
        serde_json::to_writer_pretty(&mut writer, &data).unwrap();
    }
}
