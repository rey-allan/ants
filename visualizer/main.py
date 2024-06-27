from game import AntsGame


def main(width: int, height: int) -> None:
    game = AntsGame(width, height)
    game.run()


if __name__ == "__main__":
    # TODO: Parse an actual replay file
    main(width=700, height=700)
