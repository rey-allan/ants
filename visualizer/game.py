import math
import time
from typing import List, Tuple

import pygame


CHAR_WATER = "%"
COLOR_FOOD = "#999166"
COLOR_LAND = "#795a3b"
COLOR_PLAYERS = [
    "#D41132",
    "#E86E30",
    "#E6B319",
    "#F6F655",
    "#A6E599",
    "#308CE8",
    "#6317CF",
    "#D65CD6",
    "#CFAFB7",
    "#2EB87E",
]
COLOR_WATER = "#010647"
DELAY = 0.5
DIRECTION_EAST = "E"
DIRECTION_NORTH = "N"
DIRECTION_SOUTH = "S"
DIRECTION_WEST = "W"
FPS = 60
MIN_HEIGHT = 600
MIN_WIDTH = 600


class AntsGame:
    """Defines the Ants game class using Pygame.

    :param replay: The replay file to visualize.
    :type replay: dict
    """

    def __init__(self, replay: dict) -> None:
        pygame.init()

        self._rows = replay["map"]["rows"]
        self._cols = replay["map"]["cols"]
        self._width = max(self._cols, MIN_WIDTH)
        self._height = max(self._rows, MIN_HEIGHT)
        self._cell_width = math.ceil(self._width / self._cols)
        self._cell_height = math.ceil(self._height / self._rows)
        self._cell_radius = min(self._cell_width, self._cell_height) // 2
        self._map = self._parse_map(replay["map"]["data"])
        self._turns = replay["turns"]
        self._current_turn = 0
        self._ants = {}
        self._attacks = []
        self._ants_to_remove = []
        self._food = {}
        self._screen = pygame.display.set_mode((self._width, self._height))
        self._clock = pygame.time.Clock()
        self._running = False

    def run(self) -> None:
        """Runs the game loop."""
        self._running = True

        while self._running:
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    self._running = False

            if self._current_turn >= len(self._turns):
                continue

            turn = self._turns[self._current_turn]
            self._move_ants(turn.get("move", []))
            self._attack_ants(turn.get("attack", []))
            self._raze_hills(turn.get("raze", []))
            self._spawn_ants(turn.get("spawn", []))
            self._spawn_food(turn.get("food", []))

            self._draw_map()
            self._draw_ants()
            self._draw_food()
            self._draw_attacks()

            self._remove_ants()

            pygame.display.flip()
            self._clock.tick(FPS)
            # Add a delay for better visualization
            time.sleep(DELAY)

            self._current_turn += 1

        pygame.quit()

    def _parse_map(self, map: List[str]) -> dict[str, List[dict]]:
        parsed_map = {}

        for i, row in enumerate(map):
            for j, col in enumerate(row):
                if col == CHAR_WATER:
                    parsed_map[f"{i}-{j}"] = {"type": "water", "row": i, "col": j}
                elif col.isdigit():
                    parsed_map[f"{i}-{j}"] = {"type": "hill", "row": i, "col": j, "owner": int(col), "razed": False}

        return parsed_map

    def _move_ants(self, to_move: List[dict]) -> None:
        for move in to_move:
            ant = self._ants.get(f"{move['id']}-{move['owner']}", None)

            if not ant:
                continue

            ant["location"] = self._move_ant(ant, move["direction"])

    def _move_ant(self, ant: dict, direction: str) -> Tuple[int, int]:
        location = ant["location"]
        row, col = location

        if direction == DIRECTION_NORTH:
            row -= 1
        elif direction == DIRECTION_SOUTH:
            row += 1
        elif direction == DIRECTION_WEST:
            col -= 1
        elif direction == DIRECTION_EAST:
            col += 1

        return row, col

    def _attack_ants(self, to_attack: List[dict]) -> None:
        for attack in to_attack:
            attacker = self._ants.get(f"{attack['id']}-{attack['owner']}", None)
            attacked = self._ants.get(f"{attack['target']['id']}-{attack['target']['owner']}", None)

            if not attacker or not attacked:
                continue

            self._attacks.append((attacker["location"], attacked["location"]))
            self._ants_to_remove.append(attacked)

    def _raze_hills(self, to_raze: List[dict]) -> None:
        for raze in to_raze:
            location = raze["location"]
            hill = self._map.get(f"{location[0]}-{location[1]}", None)

            if not hill:
                continue

            hill["razed"] = True

    def _spawn_ants(self, to_spawn: List[dict]) -> None:
        for ant in to_spawn:
            self._ants[f"{ant['id']}-{ant['owner']}"] = {**ant}

    def _spawn_food(self, to_spawn: List[dict]) -> None:
        for food in to_spawn:
            location = food["location"]
            self._food[f"{location[0]}-{location[1]}"] = {**food}

    def _remove_ants(self) -> None:
        for ant in self._ants_to_remove:
            del self._ants[f"{ant['id']}-{ant['owner']}"]

        self._ants_to_remove = []

    def _draw_map(self) -> None:
        self._screen.fill(COLOR_LAND)

        for element in self._map.values():
            if element["type"] == "water":
                self._draw_water(element["row"], element["col"])
            elif element["type"] == "hill":
                self._draw_hill(element["row"], element["col"], element["owner"], element["razed"])

    def _draw_ants(self) -> None:
        for ant in self._ants.values():
            row, col = ant["location"]
            owner = ant["owner"]
            self._draw_ant(row, col, owner)

    def _draw_food(self) -> None:
        for food in self._food.values():
            row, col = food["location"]
            self._draw_square(row, col, width=self._cell_width // 1.7, height=self._cell_height // 1.7, color=COLOR_FOOD)

    def _draw_attacks(self) -> None:
        for attacker, attacked in self._attacks:
            attacker_center = self._center(*self._scale(attacker[0], attacker[1]))
            attacked_center = self._center(*self._scale(attacked[0], attacked[1]))
            pygame.draw.line(self._screen, color=(0, 0, 0), start_pos=attacker_center, end_pos=attacked_center, width=2)

        self._attacks = []

    def _draw_water(self, row: int, col: int) -> None:
        self._draw_square(row, col, width=self._cell_width, height=self._cell_height, color=COLOR_WATER)

    def _draw_hill(self, row: int, col: int, owner: int, razed: bool) -> None:
        # Draw a circle with a random color based on the hill's owner
        self._draw_circle(row, col, self._cell_radius, COLOR_PLAYERS[owner % len(COLOR_PLAYERS)])
        # And a smaller circle in the center with with black color if it's "open" and land color if it's razed
        self._draw_circle(row, col, self._cell_radius // 2, color=(0, 0, 0) if not razed else COLOR_LAND)

    def _draw_ant(self, row: int, col: int, owner: int) -> None:
        self._draw_circle(row, col, self._cell_radius // 1.5, color=COLOR_PLAYERS[owner % len(COLOR_PLAYERS)])

    def _draw_circle(self, row: int, col: int, radius: int, color: Tuple[int, int, int]) -> None:
        scaled_row, scaled_col = self._scale(row, col)
        center = self._center(scaled_row, scaled_col)
        pygame.draw.circle(self._screen, color, center, radius)

    def _draw_square(self, row: int, col: int, width: int, height: int, color: Tuple[int, int, int]) -> None:
        scaled_row, scaled_col = self._scale(row, col)
        self._screen.fill(color, rect=pygame.Rect(scaled_col, scaled_row, width, height))

    def _scale(self, row: int, col: int) -> Tuple[int, int]:
        # Calculate the scaled row and column of a cell at the given row and column
        # This is only for visualization purposes to match the window size
        scaled_col = col * self._width // self._cols
        scaled_row = row * self._height // self._rows

        return scaled_row, scaled_col

    def _center(self, row: int, col: int) -> Tuple[int, int]:
        # Calculate the center of a cell at the given row and column as an x,y coordinate
        return (col + self._cell_width // 2, row + self._cell_height // 2)
