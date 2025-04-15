import time
from pathlib import Path

import numpy as np

from ants_ai import AntsEnv, RandomAgent, Visualizer


def main() -> None:
    map_file = Path(__file__).parent / "maps" / "tutorial.map"

    env = AntsEnv(map_file, replay_filename="/tmp/random_agent_replay.json")
    p1 = RandomAgent("RL Agent", env.action_space, env.num_actions, seed=42)

    start = time.time()
    done = False
    obs, _ = env.reset(seed=42)
    rewards = []
    while not done:
        action, _ = p1.predict(obs)
        obs, reward, done, _, info = env.step(action)
        rewards.append(reward)

    print("Game finished")
    print(f"Avg. Reward: {np.average(rewards)}")
    print(f"Reason: {info['done_reason']}")
    print(f"Winner: {info['winner']}")
    print(f"Game took {time.time() - start} seconds")

    Visualizer("/tmp/random_agent_replay.json", scale=20, show_grid=True).run()


if __name__ == "__main__":
    main()
