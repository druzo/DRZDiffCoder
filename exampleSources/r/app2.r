# Base R vectorized summary of iris sepal widths.

iris_path <- file.path(tempdir(), "iris.csv")
write.csv(iris, iris_path, row.names = FALSE)

df <- read.csv(iris_path)
df$ratio <- with(df, Sepal.Width / Sepal.Length)

cat("n           =", nrow(df), "\n")
cat("ratio mean  =", mean(df$ratio), "\n")
cat("ratio range =", range(df$ratio), "\n")

species_means <- tapply(df$ratio, df$Species, mean)
print(species_means)