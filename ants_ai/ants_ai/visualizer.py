import importlib.resources
import json
import os
import re
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, List, Self

# Hide the annoying pygame support prompt
os.environ["PYGAME_HIDE_SUPPORT_PROMPT"] = "1"
import pygame

PLAYER_COLORS = {
    0: (212, 17, 50),
    1: (232, 110, 48),
    2: (230, 179, 25),
    3: (246, 246, 85),
    4: (166, 229, 153),
    5: (46, 184, 126),
    6: (48, 140, 232),
    7: (99, 23, 207),
    8: (214, 92, 214),
    9: (207, 175, 183),
}


@dataclass
class Event:
    """A class representing an event that occurred in the game.

    Attributes:
        event_type (str): The type of event. "Spawn", "Remove", "Move" or "Attack".
        entity (str): The entity associated with the event. "Ant", "Food" or "Hill".
        player (int): The player that owns the entity.
        location (tuple[int]): The location of the entity as a tuple of (row, col).
        destination (tuple[int]): The destination of the entity as a tuple of (row, col). Only used for "Move" and "Attack" events.
    """

    event_type: str
    """The type of event. "Spawn", "Remove", "Move" or "Attack"."""
    entity: str
    """The entity associated with the event. "Ant", "Food" or "Hill"."""
    player: int
    """The player that owns the entity."""
    location: tuple[int]
    """The location of the entity as a tuple of (row, col)."""
    destination: tuple[int]
    """The destination of the entity as a tuple of (row, col). Only used for "Move" and "Attack" events."""


@dataclass
class Turn:
    """A class representing a turn in the game.

    Attributes:
        turn_number (int): The number of the turn.
        ants (list[int]): The number of live ants per player.
        hive (list[int]): The number of ants in the hive per player.
        scores (list[int]): The scores of the players.
        events (list[Event]): The list of events that occurred in the turn.
    """

    turn_number: int
    """The number of the turn."""
    ants: list[int]
    """The number of live ants per player."""
    hive: list[int]
    """The number of ants in the hive per player."""
    scores: list[int]
    """The scores of the players."""
    events: list[Event]
    """The list of events that occurred in the turn."""

    @classmethod
    def from_json(cls, dict: dict[str, Any]) -> Self:
        """Creates a `Turn` object from a JSON dictionary.

        :param dict: The JSON dictionary to create the `Turn` object from.
        :type dict: dict[str, Any]
        :return: The `Turn` object created from the JSON dictionary.
        :rtype: Self
        """
        events = list(map(lambda event: Event(**event), dict["events"]))

        return cls(
            dict["turn"],
            dict["ants"],
            dict["hive"],
            dict["scores"],
            events,
        )


@dataclass
class Map:
    """A class representing the map of the game.

    Attributes:
        width (int): The width of the map.
        height (int): The height of the map.
        contents (str): The contents of the map.
    """

    width: int
    """The width of the map."""
    height: int
    """The height of the map."""
    contents: str
    """The contents of the map."""


@dataclass
class Replay:
    """A class representing a replay of a full game.

    Attributes:
        players (int): The number of players in the game.
        map (Map): The map of the game.
        turns (list[Turn]): The list of turns in the game.
        finished_reason (str): The reason the game finished.
    """

    players: int
    """The number of players in the
    game."""
    map: Map
    """The map of the game."""
    turns: list[Turn]
    """The list of turns in the game."""
    finished_reason: str
    """The reason the game finished."""

    @classmethod
    def from_json(cls, dict: dict[str, Any]) -> Self:
        """Creates a `Replay` object from a JSON dictionary.

        :param dict: The JSON dictionary to create the `Replay` object from.
        :type dict: dict[str, Any]
        :return: The `Replay` object created from the JSON dictionary.
        :rtype: Self
        """
        turns = list(map(lambda turn: Turn.from_json(turn), dict["turns"]))
        _map = Map(**dict["map"])

        return cls(
            dict["players"],
            _map,
            turns,
            dict["finished_reason"],
        )


class Entity(ABC):
    """An abstract class representing an entity in the game."""

    @abstractmethod
    def draw(self, screen: pygame.Surface, scale: int) -> None:
        """Draws the entity.

        :param screen: The screen to draw the entity on.
        :type screen: pygame.Surface
        :param scale: The scale of the entity.
        :type scale: int
        """
        raise NotImplementedError


@dataclass
class Ant(Entity):
    """A class representing an ant in the game.

    Attributes:
        player (int): The player that owns the ant.
        location (tuple[int]): The location of the ant as a tuple of (row, col).
    """

    player: int
    """The player that owns the ant."""
    location: tuple[int]
    """The location of the ant as a tuple of (row, col)."""

    def draw(self, screen: pygame.Surface, scale: int) -> None:
        row, col = self.location
        color = PLAYER_COLORS[self.player]
        radius = scale // 2
        center = (col * scale + radius, row * scale + radius)
        pygame.draw.circle(screen, color, center, scale // 5)


@dataclass
class Food(Entity):
    """A class representing food in the game.

    Attributes:
        location (tuple[int]): The location of the food as a tuple of (row, col).
    """

    location: tuple[int]
    """The location of the food as a tuple of (row, col)."""

    def draw(self, screen: pygame.Surface, scale: int) -> None:
        row, col = self.location
        rect = (col * scale, row * scale, scale // 3, scale // 3)
        pygame.draw.rect(screen, (153, 145, 102), rect)


@dataclass
class Hill(Entity):
    """A class representing a hill in the game.

    Attributes:
        player (int): The player that owns the hill.
        location (tuple[int]): The location of the hill as a tuple of (row, col).
        alive (bool): Whether the hill is alive or not.
        sprites (tuple[pygame.Surface]): The sprites for the hill (alive and razed).
    """

    player: int
    """The player that owns the hill."""
    location: tuple[int]
    """The location of the hill as a tuple of (row, col)."""
    alive: bool
    """Whether the hill is alive or not."""
    sprites: tuple[pygame.Surface]
    """The sprites for the hill (alive and razed)."""

    def draw(self, screen: pygame.Surface, scale: int) -> None:
        row, col = self.location
        sprite = self.sprites[0] if self.alive else self.sprites[1]
        sprite = pygame.transform.scale(sprite, (scale, scale))

        # Draw an outline of the player's color on the sprite to indicate ownership of the hill
        # Only draw the outline if the hill is alive
        if self.alive:
            color = PLAYER_COLORS[self.player]
            overlay = pygame.Surface((scale, scale), pygame.SRCALPHA)
            center = (scale // 2, scale // 2)
            radius = scale // 4
            pygame.draw.circle(overlay, color, center, radius, width=3)
            sprite.blit(overlay, (0, 0))

        screen.blit(sprite, (col * scale, row * scale))


@dataclass
class Water(Entity):
    """A class representing water in the game.

    Attributes:
        location (tuple[int]): The location of the water as a tuple of (row, col).
        sprite: (pygame.Surface): The sprite to use for the water.
    """

    location: tuple[int]
    """The location of the water as a tuple of (row, col)."""
    sprite: pygame.Surface
    """The sprite to use for the water."""

    def draw(self, screen: pygame.Surface, scale: int) -> None:
        row, col = self.location
        sprite = pygame.transform.scale(self.sprite, (scale, scale))
        screen.blit(sprite, (col * scale, row * scale))


class Visualizer:
    """A class for visualizing a replay of a full Ants game.

    :param replay_filename: The filename of the replay to visualize.
    :type replay_filename: str
    :param scale: The scale factor for the map when visualizing, defaults to 10.
    :type scale: int
    :param speed: The speed of the visualization in FPS, defaults to 1.
    :type speed: int
    """

    def __init__(self, replay_filename: str, scale: int = 10, speed: int = 1) -> None:
        pygame.init()
        pygame.display.set_caption("Ants Replay Visualizer")

        self._hill_sprites = self._load_hill_sprites()
        self._water_sprite = self._load_water_sprite()

        self._replay = self._load_replay(replay_filename)
        self._width = self._replay.map.width
        self._height = self._replay.map.height
        self._map = self._parse_map()

        self._scale = scale
        self._window_size = (self._width * self._scale, self._height * self._scale)
        self._land_color = (120, 89, 58)

        self._screen = pygame.display.set_mode(self._window_size)
        self._clock = pygame.time.Clock()
        self._speed = speed

    def run(self) -> None:
        """Runs the visualizer."""
        running = True
        turn = 0

        while running:
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    running = False

            self._draw_map()
            pygame.display.flip()

            if turn >= len(self._replay.turns):
                continue

            self._do_replay(self._replay.turns[turn])
            turn += 1
            self._clock.tick(self._speed)

        pygame.quit()

    def _draw_map(self) -> None:
        self._screen.fill(self._land_color)

        for i in range(self._height):
            for j in range(self._width):
                for entity in self._map[i][j]:
                    entity.draw(self._screen, self._scale)

    def _do_replay(self, turn: Turn) -> None:
        for event in turn.events:
            if event.event_type == "Spawn":
                self._replay_spawn(event)

    def _replay_spawn(self, event: Event) -> None:
        row, col = event.location

        if event.entity == "Ant":
            # Ants are only spawned in hills
            self._map[row][col].append(Ant(event.player, event.location))
        elif event.entity == "Food":
            # Food is guaranteed to be spawn only in empty spaces
            self._map[row][col] = [Food(event.location)]
        else:
            raise RuntimeError(f"Invalid 'Spawn' event for entity '{event.entity}'!")

    def _load_hill_sprites(self) -> tuple[pygame.Surface]:
        with importlib.resources.path("ants_ai.assets", "hill.png") as img_path:
            spritesheet = pygame.image.load(str(img_path))
            w, h = spritesheet.get_size()
            # We have 2 states (alive and razed) stacked vertically
            sprite_height = h // 2
            # Extract the "alive" anthill (top half)
            alive = spritesheet.subsurface((0, 0, w, sprite_height))
            # Extract the "razed" anthill (bottom half)
            razed = spritesheet.subsurface((0, sprite_height, w, sprite_height))

        return alive, razed

    def _load_water_sprite(self) -> pygame.Surface:
        with importlib.resources.path("ants_ai.assets", "water.png") as img_path:
            return pygame.image.load(str(img_path))

    def _load_replay(self, replay_filename: str) -> Replay:
        with open(replay_filename, "r") as file:
            return Replay.from_json(json.load(file))

    def _parse_map(self) -> List[List[List[Entity]]]:
        regex = re.compile(r"m (.*)")
        map = [[[] for _ in range(self._width)] for _ in range(self._height)]

        for row, line in enumerate(regex.finditer(self._replay.map.contents)):
            for col, char in enumerate(line.group(1).strip()):
                # Ignore land
                if char == ".":
                    continue

                location = (row, col)
                entities = None

                # Max 10 players
                if "a" <= char <= "j":
                    player = ord(char) - ord("a")
                    entities = [Ant(player, location)]
                elif "A" <= char <= "J":
                    player = ord(char) - ord("A")
                    sprites = [
                        self._hill_sprites[0].copy(),
                        self._hill_sprites[1].copy(),
                    ]
                    entities = [
                        Hill(player, location, True, sprites),
                        Ant(player, location),
                    ]
                elif "0" <= char <= "9":
                    player = int(char)
                    sprites = [
                        self._hill_sprites[0].copy(),
                        self._hill_sprites[1].copy(),
                    ]
                    entities = [Hill(player, location, True, sprites)]
                elif char == "*":
                    entities = [Food(location)]
                elif char == "%":
                    entities = [Water(location, self._water_sprite)]
                else:
                    raise ValueError(
                        f"Unknown entity in map with character value: {char}"
                    )

                map[row][col] = entities

        return map
