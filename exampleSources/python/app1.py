"""Sort a list of dataclass items by a chosen field."""

from dataclasses import dataclass


@dataclass
class Item:
    name: str
    priority: int


def sort_by_priority(items: list[Item], descending: bool = True) -> list[Item]:
    return sorted(items, key=lambda i: i.priority, reverse=descending)


def main() -> None:
    pending = [
        Item(name="Write tests", priority=2),
        Item(name="Fix bug", priority=5),
        Item(name="Refactor parser", priority=3),
    ]
    ordered = sort_by_priority(pending)
    for item in ordered:
        print(f"{item.priority:>3}  {item.name}")


if __name__ == "__main__":
    main()