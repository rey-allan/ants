import importlib.resources
import json
import os
import re
from abc import ABC, abstractmethod
from collections import defaultdict
from dataclasses import dataclass
from enum import Enum
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
UPDATE_EPSILON = 0.001


class TurnPhase(Enum):
    """An enumeration representing the different phases of a turn."""

    Spawn = "Spawn"
    Move = "Move"
    Attack = "Attack"
    Remove = "Remove"

    def all() -> list[Self]:
        """Returns a list of all the turn phases."""
        return [phase for phase in TurnPhase]


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
        events (dict[TurnPhase, list[Event]]): A dictionary mapping turn phases to the events that occurred in that phase.
    """

    turn_number: int
    """The number of the turn."""
    ants: list[int]
    """The number of live ants per player."""
    hive: list[int]
    """The number of ants in the hive per player."""
    scores: list[int]
    """The scores of the players."""
    events: dict[TurnPhase, list[Event]]
    """A dictionary mapping turn phases to the events that occurred in that phase."""

    @classmethod
    def from_json(cls, dict: dict[str, Any]) -> Self:
        """Creates a `Turn` object from a JSON dictionary.

        :param dict: The JSON dictionary to create the `Turn` object from.
        :type dict: dict[str, Any]
        :return: The `Turn` object created from the JSON dictionary.
        :rtype: Self
        """
        # Group events by phase
        events = defaultdict(list)
        for event in dict["events"]:
            phase = TurnPhase(event["event_type"])
            events[phase].append(Event(**event))

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
            cls._parse_finished_reason(dict["finished_reason"]),
        )

    @classmethod
    def _parse_finished_reason(cls, finished_reason: str) -> str:
        if finished_reason == "LoneSurvivor":
            return "Lone Survivor (1 player left)"
        elif finished_reason == "RankStabilized":
            return "Rank Stabilized (Winner determined)"
        elif finished_reason == "TooMuchFood":
            return "Too Much Food"
        elif finished_reason == "TurnLimitReached":
            return "Max Turns Reached"
        else:
            return finished_reason


@dataclass
class Entity(ABC):
    """An abstract class representing an entity in the game.

    Attributes:
        id: (str): The ID of the entity.
        location (tuple[int]): The location of the entity as a tuple of (row, col).
        target_location (tuple[int]): The target location of the entity as a tuple of (row, col).
        scale (int): The scale of the entity on the screen.
        size (int): The size of the entity.
        target_size (int): The target size of the entity.
        alive (bool): Whether the entity is alive or not.
    """

    id: str = ""
    """The ID of the entity."""
    location: tuple[int] = (-1, -1)
    """The location of the entity as a tuple of (row, col)."""
    target_location: tuple[int] = (-1, -1)
    """The target location of the entity as a tuple of (row, col)."""
    scale: int = 0
    """The scale of the entity on the screen."""
    size: int = 0
    """The size of the entity."""
    target_size: int = 0
    """The target size of the entity."""
    alive: bool = False
    """Whether the entity is alive or not."""

    @abstractmethod
    def draw(self, screen: pygame.Surface) -> None:
        """Draws the entity.

        :param screen: The screen to draw the entity on.
        :type screen: pygame.Surface
        """
        raise NotImplementedError

    def update(self, phase: int, dt: float) -> None:
        """Updates the entity.

        :param phase: The current phase.
        :type phase: int
        :param dt: The time since the last update.
        :type dt: float
        """
        update_t = dt - phase
        phase = TurnPhase.all()[phase]

        if phase in [TurnPhase.Spawn, TurnPhase.Remove]:
            # Update the size
            if self.size == self.target_size:
                return

            size_update = (self.target_size - self.size) * update_t
            self.size += size_update

            # Cap the size to 0 when shrinking and the target size when growing
            self.size = (
                max(0, self.size)
                if self.target_size == 0
                else min(self.target_size, self.size)
            )

            # If the size is within UPDATE_EPSILON of the target size, snap to the target size
            if abs(self.size - self.target_size) <= UPDATE_EPSILON:
                self.size = self.target_size
        elif phase == TurnPhase.Move:
            self.location = self._update_location(
                self.location, self.target_location, update_t
            )

    def _update_location(
        self,
        location: tuple[int],
        target_location: tuple[int],
        update_t: float,
    ) -> tuple[int]:
        if location == target_location:
            return location

        current_row, current_col = location
        target_row, target_col = target_location

        # Update the appropriate location
        if target_row != current_row:
            row_diff = target_row - current_row
            row_update = row_diff * update_t
            new_row = current_row + row_update

            # Cap the new row to the target row based on the direction
            new_row = (
                min(target_row, new_row) if row_diff > 0 else max(target_row, new_row)
            )

            # If the new row is within UPDATE_EPSILON of the target row, snap to the target row
            if abs(new_row - target_row) <= UPDATE_EPSILON:
                new_row = target_row

            current_row = new_row

        if target_col != current_col:
            col_diff = target_col - current_col
            col_update = col_diff * update_t
            new_col = current_col + col_update

            # Cap the new col to the target col based on the direction
            new_col = (
                min(target_col, new_col) if col_diff > 0 else max(target_col, new_col)
            )

            # If the new col is within UPDATE_MOVE_EPSILON of the target col, snap to the target col
            if abs(new_col - target_col) <= UPDATE_EPSILON:
                new_col = target_col

            current_col = new_col

        return current_row, current_col


@dataclass
class Ant(Entity):
    """A class representing an ant in the game.

    Attributes:
        player (int): The player that owns the ant.
    """

    player: int = -1
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

    player: int = -1
    """The player that owns the hill."""
    sprites: tuple[pygame.Surface] = (None, None)
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

    sprite: pygame.Surface = None
    """The sprite to use for the water."""

    def draw(self, screen: pygame.Surface) -> None:
        row, col = self.location
        sprite = pygame.transform.scale(self.sprite, (self.scale, self.scale))
        screen.blit(sprite, (col * self.scale, row * self.scale))


@dataclass
class Attack(Entity):
    """A class representing an attack in the game.

    Attributes:
        current_target_location (tuple[int]): The current target location of the attack that will be updated towards the target location.
    """

    current_target_location: tuple[int] = (-1, -1)
    """The current target location of the attack that will be updated towards the target location."""

    def draw(self, screen: pygame.Surface) -> None:
        row, col = self.location
        dest_row, dest_col = self.current_target_location
        # Offset to center the line
        offset = self.scale // 2
        pygame.draw.line(
            screen,
            (255, 255, 255),
            (col * self.scale + offset, row * self.scale + offset),
            (dest_col * self.scale + offset, dest_row * self.scale + offset),
            2,
        )

    def update(self, phase, dt):
        # Update the current _target_ location towards the target location
        update_t = dt - phase
        self.current_target_location = self._update_location(
            self.current_target_location, self.target_location, update_t
        )


class Visualizer:
    """A class for visualizing a replay of a full Ants game.

    :param replay_filename: The filename of the replay to visualize.
    :type replay_filename: str
    :param scale: The scale factor for the map when visualizing, defaults to 10.
    :type scale: int
    :param show_grid: Whether to show the grid lines on the map, defaults to False.
    :type show_grid: bool
    """

    def __init__(
        self,
        replay_filename: str,
        scale: int = 10,
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
        self._attacks: list[Attack] = []
        self._parse_map()

        self._land_color = (120, 89, 58)

        self._game_size = (self._width * self._scale, self._height * self._scale)
        self._info_size = (self._game_size[0], int(self._game_size[1] * 0.12))
        self._window_size = (
            self._game_size[0],
            self._game_size[1] + self._info_size[1],
        )

        self._screen = pygame.display.set_mode(self._window_size)
        self._game_screen = pygame.Surface(self._game_size)
        self._info_screen = pygame.Surface(self._info_size)

        with importlib.resources.path(
            "ants_ai.assets", "RobotoMono-Regular.ttf"
        ) as font_path:
            self._font = pygame.font.Font(str(font_path), int(0.8 * self._scale))

        self._clock = pygame.time.Clock()
        # Simulation time
        self._time = 0.0
        # Per-turn time
        self._turn_time = 0.0

        self._show_grid = show_grid
        self._turn_phases = TurnPhase.all()
        self._replayed: dict[int, dict[TurnPhase, bool]] = {
            turn: {phase: False for phase in self._turn_phases}
            for turn in range(len(self._replay.turns))
        }

    def run(self) -> None:
        """Runs the visualizer."""
        running = True
        finished = False
        turn = 0
        prev_turn = 0

        while running:
            if finished:
                running = not self._should_quit()
                continue

            dt = self._clock.tick(60) / 1000.0
            self._time += dt

            # Track the per-turn time
            self._turn_time = (self._time * len(self._turn_phases)) % len(
                self._turn_phases
            )

            turn = int(self._time)

            if turn != prev_turn:
                # Remove dead entities and attack lines at the start of each turn
                self._remove_dead_entities()
                self._attacks.clear()
                prev_turn = turn

                # Check if the simulation has finished
                finished = turn >= len(self._replay.turns)
                if finished:
                    self._draw_endgame(self._replay.finished_reason)
                    # Re-draw the game surface to show the endgame message
                    self._screen.blit(self._game_screen, (0, self._info_size[1]))
                    pygame.display.flip()
                    continue

            phase_index = int(self._turn_time)
            phase = self._turn_phases[phase_index]

            if not self._replayed[turn][phase]:
                events = self._replay.turns[turn].events[phase]
                self._do_replay(events)
                self._replayed[turn][phase] = True

            self._update_map(phase_index)

            self._draw_info(turn, self._replay.turns[-1].turn_number)
            self._draw_grid()
            self._draw_map()

            # Draw the full screen as a combination of the game and info surfaces
            self._screen.blit(self._info_screen, (0, 0))
            self._screen.blit(self._game_screen, (0, self._info_size[1]))

            pygame.display.flip()

            running = not self._should_quit()

    def _should_quit(self) -> bool:
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                return True
        return False

    def _draw_info(self, turn: int, last_turn: int) -> None:
        self._info_screen.fill((0, 0, 0))
        width, _ = self._info_size

        # Draw number of players
        _, players_text_height = self._draw_text(
            f"Players: {self._replay.players}",
            color=(255, 255, 255),
            location=(10, 10),
            surface=self._info_screen,
        )

        # Draw the turn number
        turn_text = f"Turn: {turn} / {last_turn}"
        # Center the turn number in the middle of the info surface
        turn_text_location = (width // 2 - self._font.size(turn_text)[0] // 2, 10)
        self._draw_text(
            turn_text,
            color=(255, 255, 255),
            location=turn_text_location,
            surface=self._info_screen,
        )

        live_ants_text_width = self._font.size("Live ants: ")[0]
        scores_text_width = self._font.size("Scores: ")[0]
        hive_text_width = self._font.size("Hive: ")[0]

        # Draw the player scores as a colored bar proportional to the score
        self._draw_players_bar(
            label="Scores: ",
            label_location=(10, 10 + players_text_height),
            data=self._replay.turns[turn].scores,
            # Align it with the "Live ants" bar
            offset_x=10 + live_ants_text_width - scores_text_width,
        )

        # Draw the number of live ants as a colored bar proportional to the number of ants
        self._draw_players_bar(
            label="Live ants: ",
            label_location=(10, 10 + players_text_height * 2),
            data=self._replay.turns[turn].ants,
            offset_x=10,
        )

        # Draw the number of ants in the hive as a colored bar proportional to the number of ants
        self._draw_players_bar(
            label="Hive: ",
            label_location=(10, 10 + players_text_height * 3),
            data=self._replay.turns[turn].hive,
            # Align it with the "Live ants" bar
            offset_x=10 + live_ants_text_width - hive_text_width,
        )

    def _draw_players_bar(
        self,
        label: str,
        label_location: tuple[int],
        data: list[int],
        offset_x: int,
    ) -> None:
        label_width, label_height = self._draw_text(
            label,
            color=(255, 255, 255),
            location=label_location,
            surface=self._info_screen,
        )

        bar_width = self._info_size[0] - offset_x - label_width
        bar_height = label_height

        total = sum(data)
        current_offset_x = offset_x + label_width
        offset_y = label_location[1]

        # If there is no data to draw, draw a "0" in the center
        if total == 0:
            self._draw_text(
                "0",
                color=(255, 255, 255),
                location=(current_offset_x + bar_width // 2, offset_y),
                surface=self._info_screen,
            )
            return

        for i, value in enumerate(data):
            # Draw a bar proportional to the value
            portion = value / total if total > 0 else 0
            color = PLAYER_COLORS[i]
            width = int(portion * bar_width)
            rect = (
                current_offset_x,
                offset_y,
                width,
                bar_height,
            )
            pygame.draw.rect(self._info_screen, color, rect)

            # Draw the value text in the center of the bar
            value_text = f"{value}"
            self._draw_text(
                value_text,
                color=(0, 0, 0),
                location=(current_offset_x + width // 2, offset_y),
                surface=self._info_screen,
            )

            current_offset_x += width

        # Draw a border around the bar
        pygame.draw.rect(
            self._info_screen,
            (255, 255, 255),
            (offset_x + label_width, offset_y, bar_width, bar_height),
            2,
        )

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
                pygame.draw.rect(self._game_screen, (0, 0, 0), rect, 1)

    def _draw_map(self) -> None:
        self._game_screen.fill(self._land_color)
        for entity in [
            *self._water,
            *self._hills.values(),
            *self._food.values(),
            *self._ants.values(),
            *self._attacks,
        ]:
            entity.draw(self._game_screen)

    def _draw_endgame(self, finished_reason: str) -> None:
        # Draw the endgame in the center of the game surface
        text = f"Game Over: {finished_reason}"
        text_width, text_height = self._font.size(text)
        location = (
            self._game_size[0] // 2 - text_width // 2,
            self._game_size[1] // 2 - text_height // 2,
        )
        self._draw_text(
            text, color=(255, 255, 255), location=location, surface=self._game_screen
        )

    def _draw_text(
        self,
        text: str,
        color: tuple[int],
        location: tuple[int],
        surface: pygame.Surface,
    ) -> tuple[int]:
        surface.blit(
            self._font.render(text, True, color),
            location,
        )

        return self._font.size(text)

    def _update_map(self, phase: int) -> None:
        for entity in [
            *self._food.values(),
            *self._ants.values(),
            *self._attacks,
        ]:
            entity.update(phase, self._turn_time)

    def _do_replay(self, events: list[Event]) -> None:
        for event in events:
            if event.event_type == "Spawn":
                self._replay_spawn(event)
            elif event.event_type == "Remove":
                self._replay_remove(event)
            elif event.event_type == "Move":
                self._replay_move(event)
            elif event.event_type == "Attack":
                self._replay_attack(event)
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

    def _replay_attack(self, event: Event) -> None:
        self._attacks.append(
            Attack(
                location=tuple(event.location),
                target_location=tuple(event.destination),
                current_target_location=tuple(event.location),
                scale=self._scale,
            )
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
                        scale=self._scale,
                        alive=True,
                        player=player,
                        sprites=sprites,
                    )
                elif char == "*":
                    self._food[location] = self._spawn_food(location)
                elif char == "%":
                    self._water.append(
                        Water(
                            location=location,
                            scale=self._scale,
                            alive=True,
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
        )
