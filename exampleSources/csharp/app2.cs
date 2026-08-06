// C# imperative foreach filtering tasks by status.

using System;
using System.Collections.Generic;

public enum Status { Open, Done, Blocked }

public sealed class Task
{
    public string Title;
    public Status StatusValue;
    public int Priority;

    public Task(string title, Status status, int priority)
    {
        Title = title;
        StatusValue = status;
        Priority = priority;
    }

    public bool IsOpen() => StatusValue == Status.Open;
}

public class Program
{
    public static void Main()
    {
        var backlog = new List<Task>
        {
            new Task("Write tests", Status.Open, 2),
            new Task("Fix login bug", Status.Blocked, 5),
            new Task("Refactor parser", Status.Done, 3),
            new Task("Ship release", Status.Open, 1),
        };

        var open = new List<Task>();
        foreach (var t in backlog)
        {
            if (t.IsOpen())
            {
                open.Add(t);
            }
        }
        open.Sort((a, b) => a.Priority.CompareTo(b.Priority));

        foreach (var t in open)
        {
            Console.WriteLine(t.Title);
        }
    }
}