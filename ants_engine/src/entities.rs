use std::any::type_name;
use uuid::Uuid;

pub trait Entity {
    fn name(&self) -> &str {
        type_name::<Self>().rsplit("::").next().unwrap()
    }

    fn id(&self) -> &str {
        "Entity"
    }

    fn player(&self) -> Option<usize> {
        None
    }

    fn alive(&self) -> Option<bool> {
        None
    }

    fn set_alive(&mut self, _value: bool) {}

    fn on_ant_hill(&self) -> &Option<Box<dyn Entity>> {
        &None
    }
}

pub struct Ant {
    id: String,
    player: usize,
    alive: bool,
    on_ant_hill: Option<Box<dyn Entity>>,
}

impl Ant {
    pub fn from_ant_hill(player: usize, ant_hill: Box<dyn Entity>) -> Self {
        Ant {
            id: Uuid::new_v4().to_string(),
            player,
            alive: true,
            on_ant_hill: Some(ant_hill),
        }
    }
}

impl Entity for Ant {
    fn id(&self) -> &str {
        &self.id
    }

    fn player(&self) -> Option<usize> {
        Some(self.player)
    }

    fn alive(&self) -> Option<bool> {
        Some(self.alive)
    }

    fn set_alive(&mut self, value: bool) {
        self.alive = value;
    }

    fn on_ant_hill(&self) -> &Option<Box<dyn Entity>> {
        &self.on_ant_hill
    }
}

pub struct Food;
impl Entity for Food {}

pub struct Hill {
    player: usize,
}

impl Hill {
    pub fn new(player: usize) -> Self {
        Hill { player }
    }
}

impl Entity for Hill {
    fn player(&self) -> Option<usize> {
        Some(self.player)
    }
}

pub struct Water;
impl Entity for Water {}

pub fn from_char(value: char) -> Option<Box<dyn Entity>> {
    match value {
        // Ignore land entities to reduce memory usage
        '.' => None,
        // Max 10 players
        'a'..='j' => Some(Box::new(Ant {
            // Generate a uuid for the ant
            id: Uuid::new_v4().to_string(),
            // Convert char to digit for player number where 'a' is 0 and so on
            player: value as usize - 'a' as usize,
            alive: true,
            on_ant_hill: None,
        })),
        '*' => Some(Box::new(Food)),
        // Max 10 players
        '0'..='9' => Some(Box::new(Hill {
            player: value.to_digit(10).unwrap() as usize,
        })),
        '%' => Some(Box::new(Water)),
        _ => panic!("Invalid character value: {}", value),
    }
}
