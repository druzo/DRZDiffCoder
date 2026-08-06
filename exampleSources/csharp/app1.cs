// C# LINQ + record filtering tasks by status.

using System;
using System.Collections.Generic;
using System.Linq;

public record Task(string Title, Status StatusValue, int Priority)
{
    public enum Status { Open, Done, Blocked }

    public bool IsOpen() => StatusValue == Status.Open;
}

public class Program
{
    public static void Main()
    {
        var backlog = new List<Task>
        {
            new("Write tests", Task.Status.Open, 2),
            new("Fix login bug", Task.Status.Blocked, 5),
            new("Refactor parser", Task.Status.Done, 3),
            new("Ship release", Task.Status.Open, 1),
        };

        var open = backlog
            .Where(t => t.IsOpen())
            .OrderBy(t => t.Priority)
            .Select(t => t.Title);

        foreach (var title in open)
        {
            Console.WriteLine(title);
        }
    }
}