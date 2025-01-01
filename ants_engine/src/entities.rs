use std::any::type_name;
use uuid::Uuid;

pub trait Entity {
    fn id(&self) -> &str {
        "Entity"
    }
    fn player(&self) -> usize {
        usize::MAX
    }
    fn is_alive(&self) -> bool {
        false
    }
    fn set_alive(&mut self, _value: bool) {}
    fn name(&self) -> &str {
        type_name::<Self>().rsplit("::").next().unwrap()
    }
}

pub struct Ant {
    id: String,
    player: usize,
    is_alive: bool,
}
impl Entity for Ant {
    fn id(&self) -> &str {
        &self.id
    }

    fn player(&self) -> usize {
        self.player
    }

    fn is_alive(&self) -> bool {
        self.is_alive
    }

    fn set_alive(&mut self, value: bool) {
        self.is_alive = value;
    }
}

pub struct Food;
impl Entity for Food {}

pub struct Hill {
    player: usize,
}
impl Entity for Hill {
    fn player(&self) -> usize {
        self.player
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
            is_alive: true,
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
