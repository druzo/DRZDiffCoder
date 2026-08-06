// Dart — class + List.generate + reduce.

class Task {
  Task(this.title, this.priority);
  final String title;
  final int priority;

  @override
  String toString() => '$priority  $title';
}

void main() {
  final backlog = <Task>[
    Task('Write tests', 2),
    Task('Fix login bug', 5),
    Task('Refactor parser', 3),
  ];

  final priorities = List<int>.generate(backlog.length, (i) => backlog[i].priority);
  final maxP = priorities.reduce((a, b) => a > b ? a : b);

  print('max priority = $maxP');
  for (final t in backlog) {
    print(t);
  }
}