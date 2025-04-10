from typing import Any, List, Tuple

import gymnasium as gym
import numpy as np

from .agents import Agent, RandomAgent
from .ants_ai import Action, Direction, Game

type ActType = np.ndarray
type InfoType = dict[str, Any]
type ObsType = dict[str, Any]


class AntsEnv(gym.Env):
    """The Ants environment.

    :param map_file: The path to the map file.
    :type map_file: str
    :param fov_radius2: The radius **squared** of the field of vision for each ant, defaults to 77.
    :type fov_radius2: int, optional
    :param attack_radius2: The radius **squared** of the attack range for each ant, defaults to 5.
    :type attack_radius2: int, optional
    :param food_radius2: The radius **squared** of the range around ants to harvest food, defaults to 1
    :type food_radius2: int, optional
    :param food_rate: The amount of food to spawn *per player* on each round, defaults to 5.
    :type food_rate: int, optional
    :param max_turns: The maximum number of turns for the Ants game, defaults to 1500.
    :type max_turns: int, optional
    :param max_colony_size: The maximum number of live ants a player can have at any time, defaults to 500.
    :type max_colony_size: int, optional
    :param other_agents: The other agents to use in the game. If `None`, agents that act randomly will be created for each other player.
    :type other_agents: List[Agent], optional
    :param seed: The seed for the random number generator, defaults to 0.
    :type seed: int, optional
    :param replay_filename: The filename to save the replay of the game to. If `None`, no replay will be saved, defaults to `None`.
    :type replay_filename: _type_, optional
    """

    metadata = {"render_modes": ["ansi", "human"]}

    def __init__(
        self,
        map_file: str,
        fov_radius2: int = 77,
        attack_radius2: int = 5,
        food_radius2: int = 1,
        food_rate: int = 5,
        max_turns: int = 1500,
        max_colony_size: int = 500,
        other_agents: List[Agent] = None,
        seed: int = 0,
        replay_filename=None,
    ):
        super().__init__()

        with open(map_file) as f:
            map_contents = f.read()

        self.game = Game(
            map_contents,
            fov_radius2,
            attack_radius2,
            food_radius2,
            food_rate,
            max_turns,
            max_colony_size,
            seed,
            replay_filename,
        )

        self.channels = 11
        self.observation_space = gym.spaces.Dict(
            {
                # The observation space is a 2D grid representing a partially observable map.
                # The first channel represents the visibility "mask", i.e. `1` means that the entity is visible and `0` means that it is not.
                # The remaining channels represent the entities in the game, as follows:
                # 1: Live colony (i.e. ants of the player)
                # 2: Dead colony (i.e. ants of the player)
                # 3: Enemy colonies (i.e. ants of the enemies)
                # 4: Dead enemy colonies (i.e. ants of the enemies)
                # 5: Food
                # 6: Hills (i.e. the hills of the player)
                # 7: Razed hills (i.e. the hills of the player)
                # 8: Enemy hills (i.e. the hills of the enemies)
                # 9: Razed enemy hills (i.e. the hills of the enemies)
                # 10: Water
                # The observation is channel-first.
                "map": gym.spaces.Box(
                    low=0,
                    high=1,
                    shape=(self.channels, self.game.width(), self.game.height()),
                    dtype=int,
                ),
                # This extra space is used to represent the ants of the player.
                # It's an array where each element is a binary value indicating whether the ant is alive or not.
                "ants": gym.spaces.MultiBinary(max_colony_size),
            },
            seed=seed,
        )
        self.num_actions = 5
        # The action space is a list of actions for each ant.
        # The possible actions are: N, E, S, W, Stay
        self.action_space = gym.spaces.MultiDiscrete(
            [self.num_actions] * max_colony_size, seed=seed
        )

        self._max_colony_size = max_colony_size
        # Tracks the latest game state
        self._game_state = None
        # Tracks the index in the action space of each ant per player
        self._ant_id_to_index = {player: {} for player in range(self.game.players())}
        # Tracks the next index available to use for the next ant per player
        self._next_index_per_player = {
            player: 0 for player in range(self.game.players())
        }
        # Track reusable indices from dead ants per player
        self._free_indices_per_player = {
            player: [] for player in range(self.game.players())
        }
        self._action_to_direction = {
            0: Direction.North,
            1: Direction.East,
            2: Direction.South,
            3: Direction.West,
            4: "Stay",
        }

        self._validate_other_agents()
        self._other_agents = (
            other_agents
            if isinstance(other_agents, list)
            else [
                RandomAgent(
                    name=f"Player {i + 1}",
                    action_space=self.action_space,
                    num_actions=self.num_actions,
                    seed=seed + i,
                )
                for i in range(self.game.players() - 1)
            ]
        )

    def reset(self, seed=None, options=None) -> Tuple[ObsType, InfoType]:
        """Resets the environment.

        :return: The initial observation and info.

                 - The observation is a 2D grid representing a partially observable map and the vector of ants.
                 - The info is a dictionary with the keys `turn`, `scores` and `done_reason`.
        :rtype: Tuple[ObsType, InfoType]
        """
        super().reset(seed=seed, options=options)

        self._ant_id_to_index = {player: {} for player in range(self.game.players())}
        self._next_index_per_player = {
            player: 0 for player in range(self.game.players())
        }
        self._free_indices_per_player = {
            player: [] for player in range(self.game.players())
        }

        game_state = self.game.start()

        self._game_state = game_state
        self._update_index_mapping()

        return self._get_obs(player=0), self._get_info()

    def step(self, action: ActType) -> Tuple[ObsType, float, bool, bool, InfoType]:
        """Takes a step in the environment.

        :param action: The action to take. The action is an array of actions for each ant.
        :type actions: ActType
        :return: The observation, reward, whether the game is done, whether the game was truncated and extra info.
        :rtype: Tuple[ObsType, float, bool, bool, InfoType]
        """
        # Map the RL agent's action to the game actions
        game_actions: List[Action] = []
        for player in range(self.game.players()):
            # Player 0 is the main RL agent
            if player == 0:
                raw_action = action
            else:
                # Get the action from the other agents
                raw_action = self._other_agents[player].predict(
                    self._get_obs(player=player)
                )

            for ant in self._game_state.ants[player]:
                index = self._ant_id_to_index[player][ant.id]
                _action = raw_action[index]

                if _action == "Stay":
                    # `Stay` means the ant does not move (i.e. doesn't take any action)
                    continue

                direction = self._action_to_direction[_action]
                game_actions.append(Action(ant.row, ant.col, direction))

        game_state = self.game.update(game_actions)

        return (
            self._get_obs(player=0),
            # TODO: Define a proper reward function. The scores are the rewards for now.
            game_state.scores,
            game_state.finished,
            # Truncated is always False, since the game is not truncated
            False,
            self._get_info(),
        )

    def render(self):
        """Renders the current state of the environment to the console.

        It is recommended to use `replay_filename` in the constructor to save the replay to a file, and use the visualizer to view the replay instead of using this method.
        """
        self.game.draw()

    def _get_obs(self, player: int) -> ObsType:
        ants = self._game_state.ants[player]
        indices = self._ant_id_to_index[player]
        minimap = np.zeros(
            (self.channels, self.game.width(), self.game.height()), dtype=int
        )
        ants_mask = np.zeros(self._max_colony_size, dtype=int)

        # 0: Visibility mask
        # 1: Live colony (i.e. ants of the player)
        # 2: Dead colony (i.e. ants of the player)
        # 3: Enemy colonies (i.e. ants of the enemies)
        # 4: Dead enemy colonies (i.e. ants of the enemies)
        # 5: Food
        # 6: Hills (i.e. the hills of the player)
        # 7: Razed hills (i.e. the hills of the player)
        # 8: Enemy hills (i.e. the hills of the enemies)
        # 9: Razed enemy hills (i.e. the hills of the enemies)
        # 10: Water
        for ant in ants:
            # Add the ant to the mask
            index = indices[ant.id]
            ants_mask[index] = 1

            # Add the ant to the minimap
            minimap[0, ant.col, ant.row] = 1
            minimap[1, ant.col, ant.row] = 1

            # Add the field of vision of the ant to the minimap
            for entity in ant.field_of_vision:
                minimap[0, entity.col, entity.row] = 1

                if entity.name == "Ant":
                    colony = 1 if entity.player == ant.player else 3
                    alive = 1 - int(entity.alive)
                    minimap[colony + alive, entity.col, entity.row] = 1
                elif entity.name == "Food":
                    minimap[5, entity.col, entity.row] = 1
                elif entity.name == "Hill":
                    colony = 6 if entity.player == ant.player else 8
                    alive = 1 - int(entity.alive)
                    minimap[colony + alive, entity.col, entity.row] = 1
                elif entity.name == "Water":
                    minimap[10, entity.col, entity.row] = 1

        return {
            "map": minimap,
            "ants": ants_mask,
        }

    def _get_info(self) -> InfoType:
        return {
            "turn": self._game_state.turn,
            "scores": self._game_state.scores,
            "done_reason": self._game_state.finished_reason,
        }

    def _update_index_mapping(self) -> None:
        for player, ants in enumerate(self._game_state.ants) in range(
            self.game.players()
        ):
            self._free_dead_ants_indices(player)

            for ant in ants:
                if ant.id in self._ant_id_to_index[player]:
                    continue

                if self._free_indices_per_player[player]:
                    index = self._free_indices_per_player[player].pop()
                else:
                    index = self._next_index_per_player[player]

                    if index >= self._max_colony_size:
                        raise ValueError(
                            f"Player {player} has too many ants ({index}/{self._max_colony_size}). This is a bug!"
                        )
                    self._next_index_per_player[player] += 1

                self._ant_id_to_index[player][ant.id] = index

    def _free_dead_ants_indices(self, player: int) -> None:
        dead_ant_ids = set(self._ant_id_to_index[player]) - set(
            [ant.id for ant in self._game_state.ants[player]]
        )
        for dead_ant_id in dead_ant_ids:
            index = self._ant_id_to_index[player][dead_ant_id]
            self._free_indices_per_player[player].append(index)
            del self._ant_id_to_index[player][dead_ant_id]

    def _validate_other_agents(self) -> None:
        if self._other_agents is None or self._other_agents == "random":
            return

        if len(self._other_agents) != self.game.players() - 1:
            raise ValueError(
                f"Number of agents ({len(self._other_agents)}) doesn't match number of other players ({self.game.players() - 1})."
            )
