from typing import Any, List, Tuple

from .ants_ai import Action, Ant, Game, GameState


class AntsEnv:
    """The Ants environment.

    It follows a similar API to OpenAI Gym environments.

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

    def reset(self) -> Tuple[List[List[Ant]], dict[str, Any]]:
        """Resets the environment.

        :return: The initial observation and info.

                 - The observation is a list of lists of `Ant` objects, one list of ants per player.
                 - The info is a dictionary with the keys `turn`, `scores` and `done_reason`.
        :rtype: Tuple[List[List[Ant]], dict[str, Any]]
        """
        game_state = self.game.start()
        return game_state.ants, self._get_info(game_state)

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

    def _get_info(self, game_state: GameState) -> dict[str, Any]:
        return {
            "turn": game_state.turn,
            "scores": game_state.scores,
            "done_reason": game_state.finished_reason,
        }
