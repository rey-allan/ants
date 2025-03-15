import random
import time
from pathlib import Path

from ants_ai import Action, AntsEnv, Direction, Visualizer


class RandomAgent:
    def __init__(self, seed: int) -> None:
        random.seed(seed)

    def act(self, row: int, col: int) -> Action:
        direction = random.choice(
            [Direction.North, Direction.East, Direction.West, Direction.South]
        )
        return Action(row, col, direction)


def main() -> None:
    map_file = Path(__file__).parent / "maps" / "tutorial.map"

    env = AntsEnv(map_file, replay_filename="/tmp/tutorial_replay.json")
    p1 = RandomAgent(24)
    p2 = RandomAgent(42)

    start = time.time()
    done = False
    obs, _ = env.reset()
    while not done:
        actions = []
        for player, ants in enumerate(obs):
            for ant in ants:
                action = (
                    p1.act(ant.row, ant.col)
                    if player == 0
                    else p2.act(ant.row, ant.col)
                )
                actions.append(action)

        obs, rewards, done, info = env.step(actions)

    print(f"Game finished. Scores: {rewards}. Reason: {info['done_reason']}")
    print(f"Game took {time.time() - start} seconds")

    Visualizer("/tmp/tutorial_replay.json", scale=20, show_grid=True).run()


if __name__ == "__main__":
    main()
