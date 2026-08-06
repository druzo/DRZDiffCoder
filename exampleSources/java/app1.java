// Java record + stream pipeline filtering tasks by status.

import java.util.List;
import java.util.stream.Collectors;

public record Task(String title, Status status, int priority) {
    public enum Status { OPEN, DONE, BLOCKED }

    public boolean isOpen() {
        return status == Status.OPEN;
    }
}

class Main {
    public static void main(String[] args) {
        List<Task> backlog = List.of(
            new Task("Write tests", Task.Status.OPEN, 2),
            new Task("Fix login bug", Task.Status.BLOCKED, 5),
            new Task("Refactor parser", Task.Status.DONE, 3),
            new Task("Ship release", Task.Status.OPEN, 1)
        );

        List<String> openTitles = backlog.stream()
            .filter(Task::isOpen)
            .sorted((a, b) -> Integer.compare(a.priority(), b.priority()))
            .map(Task::title)
            .collect(Collectors.toList());

        openTitles.forEach(System.out::println);
    }
}