from typing import Any, Tuple

import gymnasium as gym
import numpy as np

from .agent import Agent


class RandomAgent(Agent):
    """An agent that takes random actions in the environment.

    :param name: The name of the agent.
    :type name: str
    :param env: The environment to use.
    :type env: gym.Env
    :param num_actions: The number of actions the agent can take.
    :type num_actions: int
    :param seed: The seed for the random number generator.
    :type seed: int, optional
    """

    def __init__(
        self, name: str, env: gym.Env, num_actions: int, seed: int = None
    ) -> None:
        super().__init__(name, env, seed)

        self._num_actions = num_actions

    def learn(self, **kwargs: dict[str, Any]) -> None:
        pass

    def predict(
        self, observation: dict[str, Any], **kwargs: dict[str, Any]
    ) -> Tuple[np.ndarray, Any]:
        ants = observation["ants"]
        max_ants = self._env.action_space.shape[0]

        # Build a full max of shape (max_ants, num_actions)
        mask = np.zeros((max_ants, self._num_actions), dtype=np.int8)
        for i in range(max_ants):
            # If the ant is alive, then it can take any action
            if ants[i]:
                mask[i] = [1] * self._num_actions
            # If no ant is present (or dead), then it can only take the no-op action
            else:
                mask[i, -1] = 1

        # Sample a random action for each ant using the mask
        actions = self._env.action_space.sample(mask)

        return actions, None

    def save(self, path: str, **kwargs: dict[str, Any]) -> None:
        pass

    def load(self, path: str, **kwargs: dict[str, Any]) -> None:
        pass
