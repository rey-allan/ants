import pygame


FPS = 60
MIN_WIDTH = 600
MIN_HEIGHT = 600
LAND_COLOR = "#795a3b"


class AntsGame:
    """Defines the Ants game class using Pygame.

    :param width: The width of the game. The window will be at least 600 pixels wide.
    :type width: int
    :param height: The height of the game. The window will be at least 600 pixels tall.
    :type height: int
    """

    def __init__(self, width: int, height: int) -> None:
        pygame.init()

        self._screen = pygame.display.set_mode((max(width, MIN_WIDTH), max(height, MIN_HEIGHT)))
        self._clock = pygame.time.Clock()
        self._running = False

    def run(self) -> None:
        """Runs the game loop."""
        self._running = True

        while self._running:
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    self._running = False

            self._screen.fill(LAND_COLOR)
            pygame.display.flip()

            self._clock.tick(FPS)

        pygame.quit()
