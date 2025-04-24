# ants_ai

:ant: Recreating Google's Ants AI Challenge for AI research.

## Installation

```bash
pip install ants-ai
```

## Environment

The environment is based on the original [Ants AI Challenge](http://ants.aichallenge.org/).
The game is played on a grid where each cell can contain food, water, ants, hills, or be empty (land).

The goal of the game is to destroy all enemies' hills while protecting your own. The game is played in turns.

To learn more about the different rules of the game, refer to the original documentation [here](http://ants.aichallenge.org/specification.php). Note, however, that this environment is not a 100% faithful recreation of the original game. Some rules have been changed. Refer to the source code for more details.

### Maps

The environment uses a map file to define the game following the original format.
You can find a variety of maps in the original repository [here](https://github.com/aichallenge/aichallenge/tree/epsilon/ants/maps).
Or follow the [instructions](http://ants.aichallenge.org/specification.php#Map-Format) to generate your own maps.

### Observation Space

The state is a **dictionary** of `Space`s with two keys: `map` and `ants`.

#### `map`

A partially observable **image-like** representation of the map of size `channels x rows x cols`. The channels are:

- `0`: The visibility mask. This is a binary mask that indicates which cells are visible to the player.
- `1`: The live colony of the player. This is a binary mask that indicates which cells contain ants of the player.
- `2`: The dead colony of the player. This is a binary mask that indicates which cells contain dead ants of the player.
- `3`: The enemy colonies. This is a binary mask that indicates which cells contain ants of the enemies.
- `4`: The dead enemy colonies. This is a binary mask that indicates which cells contain dead ants of the enemies.
- `5`: The food. This is a binary mask that indicates which cells contain food.
- `6`: The hills of the player. This is a binary mask that indicates which cells contain the hills of the player.
- `7`: The razed hills of the player. This is a binary mask that indicates which cells contain the razed hills of the player.
- `8`: The enemy hills. This is a binary mask that indicates which cells contain the hills of the enemies.
- `9`: The razed enemy hills. This is a binary mask that indicates which cells contain the razed hills of the enemies.
- `10`: The water. This is a binary mask that indicates which cells contain water.

#### `ants`

A binary array of size `MAX_COLONY_SIZE` that represents the ants of the player. Each element of the array is a binary value indicating whether the ant is alive or not.

This array should be used by agents to **mask** their actions to only the ants that are alive.

### Action Space

The action space is a list of actions for each ant. The possible discrete actions are:

- `0`: North
- `1`: East
- `2`: South
- `3`: West
- `4`: Stay

### Rewards

The reward, at each turn, is calculated as follows:

```python
total_reward = (
    food_harvested * 1.0 +
    ants_spawned * 1.0 +
    ants_killed * 2.0 +
    hills_razed * 10.0 -
    ants_lost * 2.0 -
    hills_lost * 10.0 -
    0.01 # living penalty
)
```

At the end of the game, an added bonus/penalty as follows:

- `+100` if the player wins
- `-100` if the player loses or ends in a draw

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
