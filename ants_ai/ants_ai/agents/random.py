from typing import Any, Tuple

import gymnasium as gym
import numpy as np

from .agent import Agent


class RandomAgent(Agent):
    """An agent that takes random actions in the environment.

    :param name: The name of the agent.
    :type name: str
    :param action_space: The action space of the environment.
    :type action_space: gym.Space
    :param num_actions: The number of actions the agent can take.
    :type num_actions: int
    :param seed: The seed for the random number generator.
    :type seed: int, optional
    """

    def __init__(
        self, name: str, action_space: gym.Space, num_actions: int, seed: int = None
    ) -> None:
        super().__init__(name, seed)

        self._action_space = action_space
        self._num_actions = num_actions

    def learn(self, **kwargs: dict[str, Any]) -> None:
        pass

    def predict(
        self, observation: dict[str, Any], **kwargs: dict[str, Any]
    ) -> Tuple[np.ndarray, Any]:
        ants = observation["ants"]
        max_ants = self._action_space.shape[0]

        # Create a mask for each ant
        # Each mask is a binary array of shape (num_actions,)
        # where 1 indicates that the action is valid for the ant
        masks = []
        for i in range(max_ants):
            # If the ant is alive, then it can take any action
            if ants[i]:
                masks.append(np.ones(self._num_actions, dtype=np.int8))
            # If no ant is present (or dead), then it can only take the no-op action
            else:
                mask = np.zeros(self._num_actions, dtype=np.int8)
                mask[-1] = 1
                masks.append(mask)

        # Sample a random action for each ant using the mask
        # `sample()` expects a tuple
        actions = self._action_space.sample(mask=tuple(masks))

        return actions, None

    def save(self, path: str, **kwargs: dict[str, Any]) -> None:
        pass

    def load(self, path: str, **kwargs: dict[str, Any]) -> None:
        pass
