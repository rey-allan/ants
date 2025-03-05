import json
import os
import re
from abc import ABC, abstractmethod
from dataclasses import dataclass
from typing import Any, List, Self

# Hide the annoying pygame support prompt
os.environ["PYGAME_HIDE_SUPPORT_PROMPT"] = "1"
import pygame


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

    def _draw_square(
        self,
        screen: pygame.Surface,
        location: tuple[int],
        scale: int,
        color: tuple[int],
    ) -> None:
        row, col = location
        rect = (col * scale, row * scale, scale, scale)
        pygame.draw.rect(screen, color, rect)


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
        pass


@dataclass
class Food(Entity):
    """A class representing food in the game.

    Attributes:
        location (tuple[int]): The location of the food as a tuple of (row, col).
    """

    location: tuple[int]
    """The location of the food as a tuple of (row, col)."""

    def draw(self, screen: pygame.Surface, scale: int) -> None:
        pass


@dataclass
class Hill(Entity):
    """A class representing a hill in the game.

    Attributes:
        player (int): The player that owns the hill.
        location (tuple[int]): The location of the hill as a tuple of (row, col).
        alive (bool): Whether the hill is alive or not.
    """

    player: int
    """The player that owns the hill."""
    location: tuple[int]
    """The location of the hill as a tuple of (row, col)."""
    alive: bool
    """Whether the hill is alive or not."""

    def draw(self, screen: pygame.Surface, scale: int) -> None:
        pass


@dataclass
class Water(Entity):
    """A class representing water in the game.

    Attributes:
        location (tuple[int]): The location of the water as a tuple of (row, col).
    """

    location: tuple[int]
    """The location of the water as a tuple of (row, col)."""

    def draw(self, screen: pygame.Surface, scale: int) -> None:
        self._draw_square(screen, self.location, scale, (79, 143, 186))


class Visualizer:
    """A class for visualizing a replay of a full Ants game.

    :param replay_filename: The filename of the replay to visualize.
    :type replay_filename: str
    :param scale: The scale factor for the map when visualizing, defaults to 10.
    :type scale: int
    """

    def __init__(self, replay_filename: str, scale: int = 10) -> None:
        pygame.init()
        pygame.display.set_caption("Ants Replay Visualizer")

        self._replay = self._load_replay(replay_filename)
        self._width = self._replay.map.width
        self._height = self._replay.map.height
        self._map = self._parse_map()

        self._scale = scale
        self._window_size = (self._width * self._scale, self._height * self._scale)
        self._land_color = (159, 119, 65)

        self._screen = pygame.display.set_mode(self._window_size)
        self._clock = pygame.time.Clock()

    def run(self) -> None:
        """Runs the visualizer."""
        running = True

        while running:
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    running = False

            self._draw_map()
            pygame.display.flip()

        pygame.quit()

    def _draw_map(self) -> None:
        self._screen.fill(self._land_color)

        for i in range(self._height):
            for j in range(self._width):
                for entity in self._map[i][j]:
                    entity.draw(self._screen, self._scale)

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
                    entities = [Hill(player, location, False), Ant(player, location)]
                elif "0" <= char <= "9":
                    player = int(char)
                    entities = [Hill(player, location, True)]
                elif char == "*":
                    entities = [Food(location)]
                elif char == "%":
                    entities = [Water(location)]
                else:
                    raise ValueError(
                        f"Unknown entity in map with character value: {char}"
                    )

                map[row][col] = entities

        return map
