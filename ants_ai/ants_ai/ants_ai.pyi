from typing import List, Optional

class Action:
    """A class representing an action that an ant can tak.

    Attributes:
        row (int): The row of the location of the ant.
        col (int): The column of the location of the ant.
        direction (Direction): The direction of the action.
    """

    row: int
    """The row of the location of the ant."""
    col: int
    """The column of the location of the ant."""
    direction: Direction
    """The direction of the action."""

class Ant:
    """A class representing an ant.

    Attributes:
        id (str): The unique identifier for the ant.
        row (int): The row of the location of the ant.
        col (int): The column of the location of the ant.
        player (int): The player that owns the ant.
        alive (bool): Whether the ant is alive.
        field_of_vision (List[Entity]): The field of vision of the ant as a list of entities the ant can see.
    """

    id: str
    """The unique identifier for the ant."""
    row: int
    """The row of the location of the ant."""
    col: int
    """The column of the location of the ant."""
    player: int
    """The player that owns the ant."""
    alive: bool
    """Whether the ant is alive."""
    field_of_vision: List[Entity]
    """The field of vision of the ant as a list of entities the ant can see."""

class Direction:
    """An enum representing a direction.

    Attributes:
        North (str): The North direction.
        East (str): The East direction.
        South (str): The South direction.
        West (str): The West direction.
    """

    North: str
    """The North direction."""
    East: str
    """The East direction."""
    South: str
    """The South direction."""
    West: str
    """The West direction."""

class Entity:
    """A class representing an entity.

    Attributes:
        name (str): The name of the entity. "Ant", "Food", "Hill" or "Water".
        row (int): The row of the location of the entity.
        col (int): The column of the location of the entity.
        player (int): The player that owns the entity.
        alive (bool): Whether the entity is alive, only applicable to ants.
    """

    name: str
    """The name of the entity. "Ant", "Food", "Hill" or "Water"."""
    row: int
    """The row of the location of the entity."""
    col: int
    """The column of the location of the entity."""
    player: int
    """The player that owns the entity."""
    alive: bool
    """Whether the entity is alive, only applicable to ants."""

class FinishedReason:
    """An enum representing the reason the game finished.

    Attributes:
        LoneSurvivor (str): The game ended because there was only one player left.
        RankStabilized (str): The game ended because the rank stabilized, i.e. no player can surpass the current leader anymore.
        TooMuchFood (str): The game ended because food was not being consumed and it reached 90% or more of the map.
        TurnLimitReached (str): The game ended because the maximum number of turns was reached.
    """

    LoneSurvivor: str
    """The game ended because there was only one player left."""
    RankStabilized: str
    """The game ended because the rank stabilized, i.e. no player can surpass the current leader anymore."""
    TooMuchFood: str
    """The game ended because food was not being consumed and it reached 90% or more of the map."""
    TurnLimitReached: str
    """The game ended because the maximum number of turns was reached."""

class Game:
    """A class representing the Ants game. Main entry point for the environment.

    :param map_contents: The contents of the map file.
    :type map_contents: str
    :param fov_radius2: The squared radius of the field of vision of the ants.
    :type fov_radius2: int
    :param attack_radius2: The squared radius of the attack range of the ants.
    :type attack_radius2: int
    :param food_radius2: The squared radius of the range around ants to harvest food.
    :type food_radius2: int
    :param food_rate: The amount of food to spawn *per player* on each round.
    :type food_rate: int
    :param max_turns: The maximum number of turns for the Ants game.
    :type max_turns: int
    :param seed: The seed for the random number generator.
    :type seed: int
    :param replay_filename: The filename to save the replay of the game to. If `None`, no replay will be saved.
    :type replay_filename: str, optional
    """

    def __init__(
        self,
        map_contents: str,
        fov_radius2: int,
        attack_radius2: int,
        food_radius2: int,
        food_rate: int,
        max_turns: int,
        seed: int,
        replay_filename: Optional[str],
    ) -> None: ...
    def start(self) -> GameState:
        """Starts the game.

        Must be called once before updating the game state.

        :return: The initial game state.
        :rtype: GameState
        """

    def update(self, actions: List[Action]) -> GameState:
        """Updates the game state with the given actions.

        :param actions: The actions to take for each ant.
        :type actions: List[Action]
        :return: The updated game state.
        :rtype: GameState
        """

    def draw(self) -> None:
        """Draws the current state of the game."""

class GameState:
    """A class representing the state of the game.

    Attributes:
        turn (int): The current turn number.
        scores (List[int]): The scores for each player.
        ants (List[List[Ant]]): The list of ants for each player.
        finished (bool): Whether the game has finished.
        finished_reason (Optional[FinishedReason]): The reason the game finished. Only present if the game has finished.
    """

    turn: int
    """The current turn number."""
    scores: List[int]
    """The scores for each player."""
    ants: List[List[Ant]]
    """The list of ants for each player."""
    finished: bool
    """Whether the game has finished."""
    finished_reason: Optional[FinishedReason]
    """The reason the game finished. Only present if the game has finished."""
