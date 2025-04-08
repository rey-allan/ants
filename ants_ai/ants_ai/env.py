from typing import Any, List, Tuple

import gymnasium as gym
import numpy as np

from .ants_ai import Action, Ant, Game, GameState

# TODO: Is this a sensible amount?
MAX_ANTS = 500


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
                "ants": gym.spaces.MultiBinary(MAX_ANTS),
            },
            seed=seed,
        )
        # The action space is a list of actions for each ant.
        # The possible actions are: N, E, S, W, Stay
        self.action_space = gym.spaces.MultiDiscrete([5] * MAX_ANTS, seed=seed)

        # Tracks the index in the action space of each ant
        self._ant_id_to_index = {}
        # Tracks the next index available to use for the next ant per player
        self._next_index_per_player = {
            player: 0 for player in range(self.game.players())
        }

    def reset(self, seed=None, options=None) -> Tuple[dict[str, Any], dict[str, Any]]:
        """Resets the environment.

        :return: The initial observation and info.

                 - The observation is a 2D grid representing a partially observable map and the vector of ants.
                 - The info is a dictionary with the keys `turn`, `scores` and `done_reason`.
        :rtype: Tuple[dict[str, Any], dict[str, Any]]
        """
        super().reset(seed=seed, options=options)

        self._ant_id_to_index = {}
        self._next_index_per_player = {
            player: 0 for player in range(self.game.players())
        }

        game_state = self.game.start()
        self._update_mapping(game_state)

        return self._get_obs(game_state), self._get_info(game_state)

    # TODO: Update method to adhere to the gym API
    # TODO: Figure out how to handle all the other players
    def step(
        self, actions: List[Action]
    ) -> Tuple[List[List[Ant]], List[int], bool, dict[str, Any]]:
        """Takes a step in the environment.

        :param actions: The actions to take for each ant.
        :type actions: List[Action]
        :return: The observation, rewards, whether the game is done and the info.

                 - The observation is a list of lists of `Ant` objects, one list of ants per player.
                 - The rewards is a list of rewards for each player. The rewards are the scores of the players. **Important:** Users are responsible for defining better rewards using other information.
                 - The done is a boolean indicating whether the game is done.
                 - The info is a dictionary with the keys `turn`, `scores` and `done_reason`.
        :rtype: Tuple[List[List[Ant]], List[int], bool, dict[str, Any]]
        """
        game_state = self.game.update(actions)
        # Rewards are the scores of the players

        return (
            game_state.ants,
            # The scores are the rewards
            game_state.scores,
            game_state.finished,
            self._get_info(game_state),
        )

    def render(self):
        """Renders the current state of the environment to the console.

        It is recommended to use `replay_filename` in the constructor to save the replay to a file, and use the visualizer to view the replay instead of using this method.
        """
        self.game.draw()

    def _get_obs(self, game_state: GameState) -> dict[str, Any]:
        # The agent is always player 0
        ants = game_state.ants[0]
        minimap = np.zeros(
            (self.channels, self.game.width(), self.game.height()), dtype=int
        )
        ants_mask = np.zeros(MAX_ANTS, dtype=int)

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
            index = self._ant_id_to_index[ant.id]
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

    def _get_info(self, game_state: GameState) -> dict[str, Any]:
        return {
            "turn": game_state.turn,
            "scores": game_state.scores,
            "done_reason": game_state.finished_reason,
        }

    def _update_mapping(self, game_state: GameState):
        for player, ants in enumerate(game_state.ants) in range(self.game.players()):
            for ant in ants:
                index = self._next_index_per_player[player]

                if ant.id in self._ant_id_to_index:
                    continue

                if index >= MAX_ANTS:
                    raise ValueError(
                        f"Too many ants for player {player} ({index}/{MAX_ANTS}!"
                    )

                self._ant_id_to_index[ant.id] = index
                self._next_index_per_player[player] += 1
