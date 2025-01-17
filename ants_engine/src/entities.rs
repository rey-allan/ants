use crossterm::style::Color;
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

    fn char(&self) -> char {
        '!'
    }

    fn color(&self) -> Color {
        Color::White
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

    fn char(&self) -> char {
        match self.alive {
            true => match self.on_ant_hill {
                Some(_) => (self.player + 'A' as usize) as u8 as char,
                None => (self.player + 'a' as usize) as u8 as char,
            },
            false => '.', // Dead ants are removed from the map
        }
    }

    fn color(&self) -> Color {
        match self.alive {
            true => player_to_color(self.player),
            false => Color::White, // Dead ants are removed from the map
        }
    }
}

pub struct Food;

impl Entity for Food {
    fn char(&self) -> char {
        '*'
    }

    fn color(&self) -> Color {
        Color::Grey
    }
}

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

    fn char(&self) -> char {
        self.player as u8 as char
    }

    fn color(&self) -> Color {
        player_to_color(self.player)
    }
}

pub struct Water;

impl Entity for Water {
    fn char(&self) -> char {
        '%'
    }

    fn color(&self) -> Color {
        Color::DarkBlue
    }
}

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

fn player_to_color(player: usize) -> Color {
    match player {
        0 => Color::Red,
        1 => Color::Green,
        2 => Color::Blue,
        3 => Color::Yellow,
        4 => Color::Magenta,
        5 => Color::Cyan,
        6 => Color::DarkRed,
        7 => Color::DarkGreen,
        8 => Color::DarkMagenta,
        9 => Color::DarkYellow,
        _ => panic!("Invalid player number"),
    }
}
