import importlib.resources
import json
import math
import os
import re
from abc import ABC, abstractmethod
from collections import defaultdict
from dataclasses import dataclass
from typing import Any, Self

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
UPDATE_SIZE_SPEED = 20
UPDATE_MOVE_SPEED = 5


@dataclass
class Event:
    """A class representing an event that occurred in the game.

    Attributes:
        event_type (str): The type of event. "Spawn", "Remove", "Move" or "Attack".
        entity (str): The entity associated with the event. "Ant", "Food" or "Hill".
        entity_id (str): The entity ID associated with the
        player (int): The player that owns the entity.
        location (tuple[int]): The location of the entity as a tuple of (row, col).
        destination (tuple[int]): The destination of the entity as a tuple of (row, col). Only used for "Move" and "Attack" events.
    """

    event_type: str
    """The type of event. "Spawn", "Remove", "Move" or "Attack"."""
    entity: str
    """The entity associated with the event. "Ant", "Food" or "Hill"."""
    entity_id: str
    """The entity ID associated with the event."""
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


@dataclass
class Entity(ABC):
    """An abstract class representing an entity in the game.

    Attributes:
        id: (str): The ID of the entity.
        location (tuple[int]): The location of the entity as a tuple of (row, col).
        target_location (tuple[int]): The target location of the entity as a tuple of (row, col).
        size (int): The size of the entity.
        target_size (int): The target size of the entity.
        alive (bool): Whether the entity is alive or not.
        ready (bool): Whether the entity is ready
    """

    id: str
    """The ID of the entity."""
    location: tuple[int]
    """The location of the entity as a tuple of (row, col)."""
    target_location: tuple[int]
    """The target location of the entity as a tuple of (row, col)."""
    scale: int
    """The scale of the entity on the screen."""
    size: int
    """The size of the entity."""
    target_size: int
    """The target size of the entity."""
    alive: bool
    """Whether the entity is alive or not."""
    ready: bool
    """Whether the entity is ready or not."""

    @abstractmethod
    def draw(self, screen: pygame.Surface) -> None:
        """Draws the entity.

        :param screen: The screen to draw the entity on.
        :type screen: pygame.Surface
        """
        raise NotImplementedError

    def update(self, dt: float) -> None:
        """Updates the entity.

        :param dt: The time since the last update.
        :type dt: float
        """
        # Use `math.copysign` to get the sign of the difference between the target size and the current size
        # This is needed to ensure that the size is updated in the correct direction (grow or shrink)
        self.size += (
            dt * UPDATE_SIZE_SPEED * math.copysign(1, self.target_size - self.size)
        )
        # Cap the size to 0 when shrinking and the target size when growing
        self.size = (
            max(0, self.size)
            if self.target_size == 0
            else min(self.target_size, self.size)
        )

        # We do the same for the location
        location_dt = dt * UPDATE_MOVE_SPEED
        # Cap the update to 1 if it's larger since ants only move 1 cell at a time
        location_dt = min(1, location_dt)

        self.location = (
            self.location[0]
            + location_dt
            * math.copysign(1, self.target_location[0] - self.location[0]),
            self.location[1]
            + location_dt
            * math.copysign(1, self.target_location[1] - self.location[1]),
        )
        # Cap the location to the target location
        self.location = (
            min(self.target_location[0], self.location[0]),
            min(self.target_location[1], self.location[1]),
        )

        self.ready = (
            self.size == self.target_size and self.location == self.target_location
        )


@dataclass
class Ant(Entity):
    """A class representing an ant in the game.

    Attributes:
        player (int): The player that owns the ant.
    """

    player: int
    """The player that owns the ant."""

    def draw(self, screen: pygame.Surface) -> None:
        row, col = self.location
        color = PLAYER_COLORS[self.player]
        radius = self.scale // 2
        center = (col * self.scale + radius, row * self.scale + radius)
        pygame.draw.circle(screen, color, center, self.size)


@dataclass
class Food(Entity):
    """A class representing food in the game."""

    def draw(self, screen: pygame.Surface) -> None:
        row, col = self.location
        offset = (self.scale - self.size) // 2
        rect = (
            col * self.scale + offset,
            row * self.scale + offset,
            self.size,
            self.size,
        )
        pygame.draw.rect(screen, (153, 145, 102), rect)


@dataclass
class Hill(Entity):
    """A class representing a hill in the game.

    Attributes:
        player (int): The player that owns the hill.
        sprites (tuple[pygame.Surface]): The sprites for the hill (alive and razed).
    """

    player: int
    """The player that owns the hill."""
    sprites: tuple[pygame.Surface]
    """The sprites for the hill (alive and razed)."""

    def draw(self, screen: pygame.Surface) -> None:
        row, col = self.location
        sprite = self.sprites[0] if self.alive else self.sprites[1]
        sprite = pygame.transform.scale(sprite, (self.scale, self.scale))

        # Draw an outline of the player's color on the sprite to indicate ownership of the hill
        # Only draw the outline if the hill is alive
        if self.alive:
            color = PLAYER_COLORS[self.player]
            overlay = pygame.Surface((self.scale, self.scale), pygame.SRCALPHA)
            center = (self.scale // 2, self.scale // 2)
            radius = self.scale // 4
            pygame.draw.circle(overlay, color, center, radius, width=3)
            sprite.blit(overlay, (0, 0))

        screen.blit(sprite, (col * self.scale, row * self.scale))


@dataclass
class Water(Entity):
    """A class representing water in the game.

    Attributes:
        sprite: (pygame.Surface): The sprite to use for the water.
    """

    sprite: pygame.Surface
    """The sprite to use for the water."""

    def draw(self, screen: pygame.Surface) -> None:
        row, col = self.location
        sprite = pygame.transform.scale(self.sprite, (self.scale, self.scale))
        screen.blit(sprite, (col * self.scale, row * self.scale))


class Visualizer:
    """A class for visualizing a replay of a full Ants game.

    :param replay_filename: The filename of the replay to visualize.
    :type replay_filename: str
    :param scale: The scale factor for the map when visualizing, defaults to 10.
    :type scale: int
    :param speed: The speed of the visualization in FPS, defaults to 1.
    :type speed: int
    :param show_grid: Whether to show the grid lines on the map, defaults to False.
    :type show_grid: bool
    """

    def __init__(
        self,
        replay_filename: str,
        scale: int = 10,
        speed: int = 50,
        show_grid: bool = False,
    ) -> None:
        pygame.init()
        pygame.display.set_caption("Ants Replay Visualizer")

        self._hill_sprites = self._load_hill_sprites()
        self._water_sprite = self._load_water_sprite()

        self._replay = self._load_replay(replay_filename)
        self._width = self._replay.map.width
        self._height = self._replay.map.height
        self._scale = scale

        self._water: list[Water] = []
        self._hills: dict[tuple[int], Hill] = {}
        self._food: dict[tuple[int], Food] = {}
        self._ants: dict[str, Ant] = {}
        self._parse_map()

        self._window_size = (self._width * self._scale, self._height * self._scale)
        self._land_color = (120, 89, 58)

        self._screen = pygame.display.set_mode(self._window_size)
        self._clock = pygame.time.Clock()
        self._speed = speed
        self._show_grid = show_grid
        self._dt = 0

    def run(self) -> None:
        """Runs the visualizer."""
        running = True
        turn = 0

        while running:
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    running = False

            if turn >= len(self._replay.turns):
                continue

            # Aggregate all events for the current turn per type in a map
            events_per_type = defaultdict(list)
            for event in self._replay.turns[turn].events:
                events_per_type[event.event_type].append(event)

            # Loop through all events in the current turn in the phase order
            for event_type in ["Move", "Attack", "Remove", "Spawn"]:
                events = events_per_type[event_type]

                self._do_replay(events)
                ready = False

                while not ready:
                    self._dt = self._clock.tick(self._speed) / 1000
                    self._update_map()
                    self._draw_map()
                    self._draw_grid()
                    pygame.display.flip()
                    ready = self._all_ready()

            self._remove_dead_entities()
            turn += 1

        pygame.quit()

    def _draw_grid(self) -> None:
        if not self._show_grid:
            return

        for row in range(self._height):
            for col in range(self._width):
                rect = (
                    col * self._scale,
                    row * self._scale,
                    self._scale,
                    self._scale,
                )
                pygame.draw.rect(self._screen, (0, 0, 0), rect, 1)

    def _draw_map(self) -> None:
        self._screen.fill(self._land_color)
        for entity in [
            *self._water,
            *self._hills.values(),
            *self._food.values(),
            *self._ants.values(),
        ]:
            entity.draw(self._screen)

    def _update_map(self) -> None:
        for entity in [
            *self._water,
            *self._hills.values(),
            *self._food.values(),
            *self._ants.values(),
        ]:
            entity.update(self._dt)

    def _all_ready(self) -> bool:
        return all(
            entity.ready
            for entity in [
                *self._water,
                *self._hills.values(),
                *self._food.values(),
                *self._ants.values(),
            ]
        )

    def _do_replay(self, events: list[Event]) -> None:
        for event in events:
            if event.event_type == "Spawn":
                self._replay_spawn(event)
            elif event.event_type == "Remove":
                self._replay_remove(event)
            elif event.event_type == "Move":
                self._replay_move(event)
            elif event.event_type == "Attack":
                self.replay_attack(event)
            else:
                raise RuntimeError(
                    f"Invalid event type '{event.event_type}' in event {event}."
                )

    def _replay_spawn(self, event: Event) -> None:
        location = tuple(event.location)

        if event.entity == "Ant":
            self._ants[event.entity_id] = self._spawn_ant(
                event.entity_id, location, event.player
            )
        elif event.entity == "Food":
            self._food[location] = self._spawn_food(location)
        else:
            raise RuntimeError(
                f"Invalid 'Spawn' event for entity '{event.entity}': {event}."
            )

    def _replay_remove(self, event: Event) -> None:
        if event.entity == "Ant":
            self._ants[event.entity_id].target_size = 0
            self._ants[event.entity_id].alive = False
        elif event.entity == "Food":
            location = tuple(event.location)
            self._food[location].target_size = 0
            self._food[location].alive = False
        elif event.entity == "Hill":
            location = tuple(event.location)
            # When hills are removed they are "razed", not removed from the map
            self._hills[location].alive = False
        else:
            raise RuntimeError(
                f"Invalid 'Remove' event for entity '{event.entity}': {event}."
            )

    def _replay_move(self, event: Event) -> None:
        to = tuple(event.destination)
        ant = self._ants.get(event.entity_id)

        if not ant:
            raise RuntimeError(
                f"No ant found with id {event.entity_id} to move in event: {event}."
            )

        # Move the ant to its new location
        ant.target_location = to

    def replay_attack(self, event: Event) -> None:
        row, col = event.location
        dest_row, dest_col = event.destination

        # Draw a line from the attacking ant to the target
        pygame.draw.line(
            self._screen,
            (0, 0, 0),
            (col * self._scale, row * self._scale),
            (dest_col * self._scale, dest_row * self._scale),
            2,
        )

    def _remove_dead_entities(self) -> None:
        # Remove dead ants
        for ant_id, ant in list(self._ants.items()):
            if not ant.alive:
                del self._ants[ant_id]

        # Remove consumed/destroyed food
        for location, food in list(self._food.items()):
            if not food.alive:
                del self._food[location]

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

    def _parse_map(self) -> None:
        regex = re.compile(r"m (.*)")

        for row, line in enumerate(regex.finditer(self._replay.map.contents)):
            for col, char in enumerate(line.group(1).strip()):
                # Ignore land
                if char == ".":
                    continue

                location = (row, col)

                # Max 10 players
                if "0" <= char <= "9":
                    player = int(char)
                    sprites = [
                        self._hill_sprites[0].copy(),
                        self._hill_sprites[1].copy(),
                    ]
                    self._hills[location] = Hill(
                        id=f"Hill(p={player},loc=({location}))",
                        location=location,
                        target_location=location,
                        scale=self._scale,
                        size=self._scale,
                        target_size=self._scale,
                        alive=True,
                        ready=True,
                        player=player,
                        sprites=sprites,
                    )
                elif char == "*":
                    self._food[location] = self._spawn_food(location)
                elif char == "%":
                    self._water.append(
                        Water(
                            id=f"Water(loc=({location}))",
                            location=location,
                            target_location=location,
                            scale=self._scale,
                            size=self._scale,
                            target_size=self._scale,
                            alive=True,
                            ready=True,
                            sprite=self._water_sprite,
                        )
                    )
                else:
                    raise ValueError(
                        f"Invalid entity in map with character value: {char}"
                    )

    def _spawn_ant(self, id: str, location: tuple[int], player: int) -> Ant:
        return Ant(
            id,
            location,
            target_location=location,
            scale=self._scale,
            size=0,
            target_size=self._scale // 5,
            alive=True,
            ready=False,
            player=player,
        )

    def _spawn_food(self, location: tuple[int]) -> Food:
        return Food(
            id=f"Food(loc=({location}))",
            location=location,
            target_location=location,
            scale=self._scale,
            size=0,
            target_size=self._scale // 3,
            alive=True,
            ready=False,
        )
