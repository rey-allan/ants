import argparse
from pathlib import Path
from typing import Any, Optional, Tuple

import gymnasium as gym
import numpy as np
import torch
import torch.nn as nn
from stable_baselines3 import PPO
from stable_baselines3.common.distributions import Categorical, Distribution
from stable_baselines3.common.policies import ActorCriticPolicy
from stable_baselines3.common.torch_layers import BaseFeaturesExtractor

from ants_ai import Agent, AntsEnv, Visualizer


class PartiallyObservableMapFeatureExtractor(BaseFeaturesExtractor):
    def __init__(self, observation_space: gym.spaces.Dict) -> None:
        map_shape = observation_space["map"].shape
        features_size = 128

        super().__init__(observation_space, features_dim=features_size)

        self.cnn = nn.Sequential(
            nn.Conv2d(map_shape[0], 16, kernel_size=3, padding=1),
            nn.ReLU(),
            nn.Flatten(),
        )

        # Compute shape by doing one forward pass
        # We need to do this to avoid computing the shape manually
        with torch.no_grad():
            n_flatten = self.cnn(torch.zeros(1, *map_shape)).shape[1]

        self.linear = nn.Sequential(
            nn.Linear(n_flatten, features_size),
            nn.ReLU(),
        )

    def forward(self, observations: dict[str, Any]) -> torch.Tensor:
        return self.linear(self.cnn(observations["map"]))


class MaskedPolicy(ActorCriticPolicy):
    def __init__(
        self, *args, max_colony_size: int = 25, num_actions: int = 5, **kwargs
    ):
        super().__init__(
            *args,
            features_extractor_class=PartiallyObservableMapFeatureExtractor,
            normalize_images=False,
            **kwargs,
        )

        self.max_colony_size = max_colony_size
        self.num_actions = num_actions
        self.action_net = nn.Linear(
            self.features_dim, self.max_colony_size * self.num_actions
        )
        self.value_net = nn.Linear(self.features_dim, 1)

    def forward(self, obs: dict[str, Any], deterministic: bool = False) -> torch.Tensor:
        features = self.extract_features(obs)
        masked_logits = self._mask(obs, features)
        dist = Categorical(logits=masked_logits)

        actions = (
            dist.sample() if not deterministic else torch.argmax(masked_logits, dim=-1)
        )
        log_prob = dist.log_prob(actions).sum(dim=1)

        return actions, self.value_net(features), log_prob

    def evaluate_actions(
        self, obs: dict[str, Any], actions: torch.Tensor
    ) -> Tuple[torch.Tensor, torch.Tensor, Optional[torch.Tensor]]:
        features = self.extract_features(obs)
        masked_logits = self._mask(obs, features)
        dist = Categorical(logits=masked_logits)
        values = self.value_net(features)
        log_prob = dist.log_prob(actions).sum(dim=1)
        entropy = dist.entropy().sum(dim=1)

        return values, log_prob, entropy

    def predict_values(self, obs: dict[str, Any]) -> torch.Tensor:
        return self.value_net(self.extract_features(obs))

    def get_distribution(self, obs: dict[str, Any]) -> Distribution:
        features = self.extract_features(obs)
        masked_logits = self._mask(obs, features)
        dist = Categorical(logits=masked_logits)

        return dist

    def _predict(
        self, observation: dict[str, Any], deterministic: bool = False
    ) -> torch.Tensor:
        actions, _, _ = self.forward(observation, deterministic)

        return actions

    def _mask(
        self, observation: dict[str, Any], features: torch.Tensor
    ) -> torch.Tensor:
        # Shape: (batch, max_colony_size * num_actions)
        logits = self.action_net(features)
        logits = logits.view(-1, self.max_colony_size, self.num_actions)

        # Mask padded ants
        # Shape: (batch, max_colony_size)
        mask = observation["ants"].bool()

        # Set logits to -inf for invalid ants
        masked_logits = logits.clone()
        masked_logits[~mask] = float("-inf")

        # Set logits to 0 for the no-op action (last action)
        batch_idx, ant_idx = torch.where(~mask)
        masked_logits[batch_idx, ant_idx, self.num_actions - 1] = 0.0

        return masked_logits


class RLAgent(Agent):
    def __init__(
        self,
        name: str,
        env: gym.Env,
        max_colony_size: int,
        num_actions: int,
        seed: int = None,
    ) -> None:
        super().__init__(name, seed)

        self._env = env
        self._model = PPO(
            MaskedPolicy,
            env,
            policy_kwargs=dict(
                max_colony_size=max_colony_size,
                num_actions=num_actions,
            ),
            seed=seed,
            tensorboard_log="/tmp/tensorboard/",
        )

    def learn(self, **kwargs: dict[str, Any]) -> None:
        self._model.learn(**kwargs, tb_log_name=self._name)

    def predict(
        self, observation: dict[str, Any], **kwargs: dict[str, Any]
    ) -> Tuple[np.ndarray, Any]:
        return self._model.predict(observation, deterministic=True)

    def save(self, path: str, **kwargs: dict[str, Any]) -> None:
        self._model.save(path)

    def load(self, path: str, **kwargs: dict[str, Any]) -> None:
        self._model = PPO.load(path, env=self._env)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--train", action="store_true", help="Train the agent (default: False)"
    )
    parser.add_argument(
        "--eval", action="store_true", help="Evaluate the agent (default: False)"
    )
    args = parser.parse_args()

    if args.train and args.eval:
        raise ValueError("Cannot train and evaluate at the same time.")

    map_file = Path(__file__).parent / "maps" / "tutorial.map"
    max_colony_size = 25
    save_path = "/tmp/ants_tutorial_ppo"
    env = AntsEnv(
        map_file,
        food_rate=15,
        max_turns=500,
        max_colony_size=max_colony_size,
        replay_filename="/tmp/tutorial_replay.json" if args.eval else None,
        seed=42,
    )
    agent = RLAgent("RL Agent", env, max_colony_size, env.num_actions, seed=42)

    if args.train:
        _train(agent, save_path)
    elif args.eval:
        agent.load(save_path)
        _eval(env, agent)


def _train(agent: Agent, save_path: str) -> None:
    print(f"Training agent {agent._name}")

    agent.learn(total_timesteps=10_000, progress_bar=True)
    agent.save(save_path)


def _eval(env: gym.Env, agent: Agent) -> None:
    print(f"Evaluating agent {agent._name}!")

    obs, _ = env.reset(seed=42)
    done = False
    rewards = []
    while not done:
        action, _ = agent.predict(obs, deterministic=True)
        obs, reward, done, _, info = env.step(action)
        rewards.append(reward)

    print("Game finished")
    print(f"Avg. Reward: {np.average(rewards)}")
    print(f"Reason: {info['done_reason']}")
    print(f"Winner: {info['winner']}")

    Visualizer("/tmp/tutorial_replay.json", scale=20, show_grid=True).run()


if __name__ == "__main__":
    main()
