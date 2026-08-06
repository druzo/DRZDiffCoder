// Kotlin — data class + collection map.

data class Task(val title: String, val priority: Int, val done: Boolean)

fun main() {
    val backlog = listOf(
        Task("Write tests", 2, false),
        Task("Fix login bug", 5, false),
        Task("Refactor parser", 3, true),
        Task("Ship release", 1, false),
    )

    val open = backlog.filter { !it.done }
    val ordered = open.sortedBy { it.priority }
    val titles = ordered.map { it.title }

    titles.forEach { println(it) }
}