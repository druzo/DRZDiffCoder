// Swift — struct + map + sort.

struct Task: Equatable {
    var title: String
    var priority: Int
    var done: Bool
}

let backlog: [Task] = [
    .init(title: "Write tests",     priority: 2, done: false),
    .init(title: "Fix login bug",   priority: 5, done: false),
    .init(title: "Refactor parser", priority: 3, done: true),
    .init(title: "Ship release",    priority: 1, done: false),
]

let open = backlog.filter { !$0.done }
let ordered = open.sorted { $0.priority < $1.priority }
let titles = ordered.map(\.title)

for t in titles {
    print(t)
}