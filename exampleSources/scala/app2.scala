// Scala — collection ops on a Map + fold.

object Main extends App {
  val wordCounts = Map(
    "rust"   -> 42,
    "scala"  -> 27,
    "python" -> 35,
    "go"     -> 18,
  )

  val total = wordCounts.values.foldLeft(0)(_ + _)
  val top = wordCounts.toList
    .sortBy { case (_, c) => -c }
    .take(3)

  println(s"total = $total")
  top.foreach { case (lang, c) => println(s"$lang -> $c") }
}