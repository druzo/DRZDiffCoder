// Scala — case class + pattern match + sort.

case class Task(title: String, priority: Int, done: Boolean)

object Main extends App {
  val backlog = List(
    Task("Write tests",     2, false),
    Task("Fix login bug",   5, false),
    Task("Refactor parser", 3, true),
    Task("Ship release",    1, false),
  )

  val open = backlog.filter(!_.done)
  val ordered = open.sortBy(_.priority)
  val titles = ordered.map(_.title)

  titles.foreach(println)
}