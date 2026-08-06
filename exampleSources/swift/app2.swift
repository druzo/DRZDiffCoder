// Swift — protocol + extension + conformance.

protocol Describable {
    var summary: String { get }
}

struct Task: Describable {
    var title: String
    var priority: Int
}

extension Task {
    var summary: String { "\(priority)  \(title)" }
}

extension Sequence where Element == Task {
    var titles: [String] { map(\.title) }
}

let backlog: [Task] = [
    .init(title: "Write tests",     priority: 2),
    .init(title: "Fix login bug",   priority: 5),
    .init(title: "Refactor parser", priority: 3),
]

for task in backlog {
    print(task.summary)
}
print("count = \(backlog.titles.count)")