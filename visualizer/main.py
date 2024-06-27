import argparse
import json

from game import AntsGame


def main(map: dict) -> None:
    game = AntsGame(width=map["cols"], height=map["rows"], map=map["data"])
    game.run()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Visualize an Ants game.")
    parser.add_argument("--file", "-f", type=str, required=True, help="The replay file to visualize.")

    args = parser.parse_args()

    with open(args.file, "r") as f:
        replay = json.load(f)

    main(replay["map"])
