// Kotlin — sealed class + when expression over a small state.

sealed class Status {
    object Open : Status()
    object Done : Status()
    object Blocked : Status()
}

data class Task(val title: String, val priority: Int, val status: Status)

fun describe(t: Task): String = when (t.status) {
    Status.Open    -> "TODO  ${t.title} (pri=${t.priority})"
    Status.Done    -> "OK   ${t.title}"
    Status.Blocked -> "WAIT ${t.title}"
}

fun main() {
    val backlog = listOf(
        Task("Write tests", 2, Status.Open),
        Task("Fix login bug", 5, Status.Blocked),
        Task("Refactor parser", 3, Status.Done),
    )
    backlog.forEach { println(describe(it)) }
}