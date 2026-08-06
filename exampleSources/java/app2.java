// Imperative loop filtering tasks by status.

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;

public final class Task {
    public enum Status { OPEN, DONE, BLOCKED }

    public final String title;
    public final Status status;
    public final int priority;

    public Task(String title, Status status, int priority) {
        this.title = title;
        this.status = status;
        this.priority = priority;
    }

    public boolean isOpen() {
        return status == Status.OPEN;
    }
}

class Main {
    public static void main(String[] args) {
        List<Task> backlog = new ArrayList<>();
        backlog.add(new Task("Write tests", Task.Status.OPEN, 2));
        backlog.add(new Task("Fix login bug", Task.Status.BLOCKED, 5));
        backlog.add(new Task("Refactor parser", Task.Status.DONE, 3));
        backlog.add(new Task("Ship release", Task.Status.OPEN, 1));

        List<Task> open = new ArrayList<>();
        for (Task t : backlog) {
            if (t.isOpen()) {
                open.add(t);
            }
        }
        Collections.sort(open, Comparator.comparingInt(t -> t.priority));

        for (Task t : open) {
            System.out.println(t.title);
        }
    }
}