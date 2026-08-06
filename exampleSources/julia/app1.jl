# Julia — struct + broadcast over arrays.

struct Task
    title::String
    priority::Int
end

function sort_by_priority(items::Vector{Task})::Vector{Task}
    return sort(items, by = t -> t.priority)
end

function main()
    backlog = Task[
        Task("Write tests", 2),
        Task("Fix login bug", 5),
        Task("Refactor parser", 3),
    ]

    priorities = broadcast(t -> t.priority, backlog)
    max_p = maximum(priorities)
    println("max priority = ", max_p)

    for t in sort_by_priority(backlog)
        println(rpad(string(t.priority), 3), " ", t.title)
    end
end

main()