# Filter iris rows where sepal length exceeds the column mean.

library(stats)

iris_path <- file.path(tempdir(), "iris.csv")
write.csv(iris, iris_path, row.names = FALSE)

df <- read.csv(iris_path)
mean_sepal <- mean(df$Sepal.Length)

big <- subset(df, Sepal.Length > mean_sepal)
cat("rows:", nrow(big), "\n")

agg <- aggregate(
  Sepal.Width ~ Species,
  data = big,
  FUN = function(x) c(mean = mean(x), sd = sd(x))
)
print(agg)