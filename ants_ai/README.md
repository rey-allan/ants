# ants_ai

:ant: Recreating Google's Ants AI Challenge for AI research.

## Installation

```bash
pip install ants-ai
```

## Environment

The environment is based on the original [Ants AI Challenge](http://ants.aichallenge.org/) with a few modifications of the rules.
The game is played on a grid where each cell can contain food, water, ants, hills, or be empty (land).

The goal of the game is to destroy all enemies' hills while protecting your own. The game is played in turns.

To learn more about the different rules of the game, refer to the original documentation [here](http://ants.aichallenge.org/specification.php).

### Maps

The environment uses a map file to define the game following the original format.
You can find a variety of maps in the original repository [here](https://github.com/aichallenge/aichallenge/tree/epsilon/ants/maps).
Or follow the [instructions](http://ants.aichallenge.org/specification.php#Map-Format) to generate your own maps.

### Observations

A list of all the alive ants in the game, per player. Each ant contains the following attributes:

- `id`: The unique id of the ant.
- `player`: The player id of the ant.
- `row`: The row of the location of the ant.
- `col`: The column of the location of the ant.
- `alive`: A boolean indicating if the ant is alive or not. This is always `true` for the ants in the observation.
- `field_of_vision`: A list of all the entities that the ant can see.

Each entity in the field of vision contains the following attributes:

- `name`: The name of the entity. One of "Ant", "Food", "Water" or "Hill".
- `player`: The player that owns the entity. Only present if the entity is an ant or a hill.
- `row`: The row of the location of the entity.
- `col`: The column of the location of the entity.
- `alive`: A boolean indicating if the entity is alive or not. Only present if the entity is an ant.

### Actions

Ants can only move, so each action is represented by a `row`, `col` and `direction`, where the `row` and `col` are the coordinates of the ant and the `direction` is one of "North", "South", "East" or "West".
The action is only valid if the ant can move in that direction. If the action is not valid, the ant will not move.

When stepping, a list of actions is passed to the environment for all the ants that will move. If you don't want to move an ant, don't include it in the list of actions.

### Rewards

The rewards is a list of scores for each player. The score is calculated as follows:

- Each player starts with 1 point per hill.
- Razing (i.e. destroying) an enemy hill is 2 points.
- Losing a hill is -1 points.

Note that this might not be the best rewarding scheme, and we recommend you define your own reward to improve the performance of your agent.

## Visualization

The environment implements the `render` method of the common `gym` interface. However, this renders the game in ASCII format directly to the console.

To visualize the game in a more user-friendly way, we recommend using the provided `Visualizer` which uses PyGame to render the game.
This visualizer uses a **replay** file to render the game. The replay file is a JSON file that contains the state of the game at each turn.
By default, the environment doesn't produce a replay file.
To enable it, you need to set the `replay_filename` to the path and filename where you want to save the replay file when instantiating the environment.

```python
env = AntsEnv(map_file, replay_filename="/tmp/my_replay.json")
...
Visualizer("/tmp/tutorial_replay.json", scale=20).run()
```

## Example

```python
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
```
