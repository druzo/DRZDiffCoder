"""Filter a numeric stream and compute running statistics."""

from collections.abc import Iterable


def running_stats(values: Iterable[float]) -> list[tuple[float, float]]:
    out: list[tuple[float, float]] = []
    total = 0.0
    count = 0
    for v in values:
        if v < 0:
            continue
        total += v
        count += 1
        out.append((v, total / count))
    return out


def main() -> None:
    raw = [1.0, -2.5, 3.0, 4.5, -1.0, 2.0]
    for value, avg in running_stats(raw):
        print(f"v={value:.2f}  avg={avg:.3f}")


if __name__ == "__main__":
    main()