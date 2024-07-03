import math
from typing import Tuple

import pygame


ACTION_SPAWN = "SPAWN"
CHAR_WATER = "%"
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
        self._map = replay["map"]["data"]
        self._turns = replay["turns"]
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

            self._draw_map()
            self._draw_turn()

            pygame.display.flip()
            self._clock.tick(FPS)

        pygame.quit()

    def _draw_map(self) -> None:
        self._screen.fill(COLOR_LAND)

        for i, row in enumerate(self._map):
            for j, col in enumerate(row):
                if col == CHAR_WATER:
                    self._draw_water(row=i, col=j)
                elif col.isdigit():
                    self._draw_hill(row=i, col=j, hill=int(col))

    def _draw_turn(self) -> None:
        for turn in self._turns:
            for ant in turn["ants"]:
                if ant["action"]["type"] == ACTION_SPAWN:
                    self._spawn_ant(ant, ant["action"])

    def _draw_water(self, row: int, col: int) -> None:
        scaled_row, scaled_col = self._scale(row, col)
        self._screen.fill(COLOR_WATER, rect=pygame.Rect(scaled_col, scaled_row, self._cell_width, self._cell_height))

    def _draw_hill(self, row: int, col: int, hill: int) -> None:
        # Draw a circle with a random color based on the hill number
        self._draw_circle(row, col, self._cell_radius, COLOR_PLAYERS[hill % len(COLOR_PLAYERS)])
        # And a smaller black circle in the center
        self._draw_circle(row, col, self._cell_radius // 2, color=(0, 0, 0))

    def _spawn_ant(self, ant: dict, action: dict) -> None:
        location = action["location"]
        self._draw_ant(row=location[0], col=location[1], owner=ant["owner"])

    def _draw_ant(self, row: int, col: int, owner: int) -> None:
        self._draw_circle(row, col, self._cell_radius // 1.5, color=COLOR_PLAYERS[owner % len(COLOR_PLAYERS)])

    def _draw_circle(self, row: int, col: int, radius: int, color: Tuple[int, int, int]) -> None:
        scaled_row, scaled_col = self._scale(row, col)
        center = (scaled_col + self._cell_width // 2, scaled_row + self._cell_height // 2)
        pygame.draw.circle(self._screen, color, center, radius)

    def _scale(self, row: int, col: int) -> Tuple[int, int]:
        # Calculate the scaled row and column of a cell at the given row and column
        # This is only for visualization purposes to match the window size
        scaled_col = col * self._width // self._cols
        scaled_row = row * self._height // self._rows

        return scaled_row, scaled_col
