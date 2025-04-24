from abc import ABC, abstractmethod
from typing import Any, Tuple

import numpy as np


class Agent(ABC):
    """Defines the base class for all agents.

    :param name: The name of the agent
    :type name: str
    :param seed: The seed for the random number generator, defaults to None
    :type seed: int, optional
    """

    def __init__(self, name: str, seed: int = None) -> None:
        super().__init__()

        self._name = name
        self._seed = seed

    @abstractmethod
    def learn(self, **kwargs: dict[str, Any]) -> None:
        """Runs a learning procedure to train the agent"""
        raise NotImplementedError

    @abstractmethod
    def predict(
        self, observation: dict[str, Any], **kwargs: dict[str, Any]
    ) -> Tuple[np.ndarray, Any]:
        """Predicts the action(s) to take given the input observation

        :param observation: The input observation to predict for
        :type observation: dict[str, Any]
        :return: The action(s) to take, and any other extra information (optional)
        :rtype: Tuple[np.ndarray, Any]
        """
        raise NotImplementedError

    @abstractmethod
    def save(self, path: str, **kwargs: dict[str, Any]) -> None:
        """Saves the agent's parameters and related artifacts to the given path

        :param path: The path to save to
        :type path: str
        """
        raise NotImplementedError

    @abstractmethod
    def load(self, path: str, **kwargs: dict[str, Any]) -> None:
        """Loads the agent's artifacts from the given path

        :param path: The path to load from
        :type path: str
        """
        raise NotImplementedError
