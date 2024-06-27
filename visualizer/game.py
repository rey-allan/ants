import pygame


LAND_COLOR = "#795a3b"
FPS = 60


class AntsGame:
    """Defines the Ants game class using Pygame.

    :param width: The width of the game window.
    :type width: int
    :param height: The height of the game window.
    :type height: int
    """

    def __init__(self, width: int, height: int) -> None:
        pygame.init()

        self._screen = pygame.display.set_mode((width, height))
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
